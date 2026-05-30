use crate::types::*;
use async_trait::async_trait;
use futures::StreamExt;
use orca_providers::provider::{ChunkStream, ModelProvider, ProviderError};
use orca_providers::types::*;
use orca_utils::message::{Message, Role, ToolCall};
use serde_json::Value;

/// DeepSeek model provider
pub struct DeepSeekProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model_id: String,
}

impl DeepSeekProvider {
    pub fn new(api_key: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.deepseek.com".to_string(),
            api_key: api_key.into(),
            model_id: model_id.into(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn convert_messages(messages: &[Message], system_prompt: Option<&str>) -> Vec<DeepSeekMessage> {
        let mut result = Vec::new();

        // Add system prompt as first message
        if let Some(system) = system_prompt {
            result.push(DeepSeekMessage {
                role: "system".to_string(),
                content: Some(system.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        for msg in messages {
            let role = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            }.to_string();

            let tool_calls = if msg.tool_calls.is_empty() {
                None
            } else {
                Some(msg.tool_calls.iter().map(|tc| DeepSeekToolCall {
                    id: tc.id.clone(),
                    call_type: "function".to_string(),
                    function: DeepSeekFunction {
                        name: tc.name.clone(),
                        arguments: tc.arguments.to_string(),
                    },
                }).collect())
            };

            result.push(DeepSeekMessage {
                role,
                content: if msg.content.is_empty() { None } else { Some(msg.content.clone()) },
                tool_calls,
                tool_call_id: msg.tool_call_id.clone(),
            });
        }

        result
    }

    fn convert_tools(tools: &[orca_utils::types::ToolDefinition]) -> Vec<DeepSeekTool> {
        tools.iter().map(|t| DeepSeekTool {
            tool_type: "function".to_string(),
            function: DeepSeekToolFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        }).collect()
    }

    fn convert_response(resp: DeepSeekResponse) -> Result<ChatResponse, ProviderError> {
        let choice = resp.choices.into_iter().next()
            .ok_or_else(|| ProviderError::ParseError("no choices in response".to_string()))?;

        let content = choice.message.content.unwrap_or_default();
        let tool_calls: Vec<ToolCall> = choice.message.tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| {
                let args: Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: args,
                }
            })
            .collect();

        let stop_reason = match choice.finish_reason.as_deref() {
            Some("stop") => StopReason::EndTurn,
            Some("tool_calls") => StopReason::ToolUse,
            Some("length") => StopReason::MaxTokens,
            Some("content_filter") => StopReason::ContentFilter,
            other => StopReason::Unknown(other.unwrap_or("none").to_string()),
        };

        let usage = resp.usage.map(|u| {
            let reasoning = u.completion_tokens_details
                .and_then(|d| d.reasoning_tokens);
            Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                reasoning_tokens: reasoning,
            }
        }).unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            stop_reason,
            reasoning: None,
        })
    }
}

#[async_trait]
impl ModelProvider for DeepSeekProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let messages = Self::convert_messages(&request.messages, request.system_prompt.as_deref());
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(Self::convert_tools(&request.tools))
        };

        let body = DeepSeekRequest {
            model: self.model_id.clone(),
            messages,
            tools,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: false,
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::NetworkError)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 => Err(ProviderError::AuthError(text)),
                429 => Err(ProviderError::RateLimited { retry_after_secs: 60 }),
                413 => Err(ProviderError::RequestTooLarge(text)),
                _ => Err(ProviderError::Other(format!("HTTP {}: {}", status, text))),
            };
        }

        let api_resp: DeepSeekResponse = resp.json().await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        Self::convert_response(api_resp)
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChunkStream, ProviderError> {
        let messages = Self::convert_messages(&request.messages, request.system_prompt.as_deref());
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(Self::convert_tools(&request.tools))
        };

        let body = DeepSeekRequest {
            model: self.model_id.clone(),
            messages,
            tools,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: true,
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::NetworkError)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 => Err(ProviderError::AuthError(text)),
                429 => Err(ProviderError::RateLimited { retry_after_secs: 60 }),
                _ => Err(ProviderError::Other(format!("HTTP {}: {}", status, text))),
            };
        }

        let stream = resp.bytes_stream();
        let chunk_stream = stream.filter_map(move |chunk_result| {
            let data = match chunk_result {
                Ok(bytes) => bytes,
                Err(e) => return futures::future::ready(Some(Err(
                    ProviderError::StreamError(e.to_string())
                ))),
            };

            let text = String::from_utf8_lossy(&data);

            // Parse SSE lines
            for line in text.lines() {
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..];
                if data == "[DONE]" {
                    return futures::future::ready(Some(Ok(StreamChunk {
                        delta_content: None,
                        delta_tool_calls: Vec::new(),
                        delta_reasoning: None,
                        done: true,
                        usage: None,
                        stop_reason: Some(StopReason::EndTurn),
                    })));
                }

                if let Ok(chunk) = serde_json::from_str::<DeepSeekStreamChunk>(data) {
                    let choice = chunk.choices.first();
                    if let Some(choice) = choice {
                        let delta_content = choice.delta.content.clone();
                        let delta_reasoning = choice.delta.reasoning_content.clone();

                        let delta_tool_calls: Vec<orca_providers::types::DeltaToolCall> = choice
                            .delta
                            .tool_calls
                            .as_ref()
                            .map(|tcs| {
                                tcs.iter().map(|tc| orca_providers::types::DeltaToolCall {
                                    index: tc.index,
                                    id: tc.id.clone(),
                                    name: tc.function.as_ref().and_then(|f| f.name.clone()),
                                    arguments_delta: tc.function.as_ref().and_then(|f| f.arguments.clone()),
                                }).collect()
                            })
                            .unwrap_or_default();

                        let stop_reason = choice.finish_reason.as_ref().map(|r| match r.as_str() {
                            "stop" => StopReason::EndTurn,
                            "tool_calls" => StopReason::ToolUse,
                            "length" => StopReason::MaxTokens,
                            _ => StopReason::Unknown(r.clone()),
                        });

                        let usage = chunk.usage.map(|u| Usage {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                            reasoning_tokens: u.completion_tokens_details
                                .and_then(|d| d.reasoning_tokens),
                        });

                        let done = stop_reason.is_some();

                        return futures::future::ready(Some(Ok(StreamChunk {
                            delta_content,
                            delta_tool_calls,
                            delta_reasoning,
                            done,
                            usage,
                            stop_reason,
                        })));
                    }
                }
            }

            futures::future::ready(None)
        });

        Ok(Box::pin(chunk_stream))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            id: orca_utils::types::ModelId(self.model_id.clone()),
            display_name: format!("DeepSeek {}", self.model_id),
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: self.model_id.contains("r1") || self.model_id.contains("reasoner"),
            input_cost_per_mtok: 0.14,
            output_cost_per_mtok: 0.28,
            is_flash: false,
        }
    }

    fn supports_reasoning(&self) -> bool {
        self.model_id.contains("r1") || self.model_id.contains("reasoner")
    }
}