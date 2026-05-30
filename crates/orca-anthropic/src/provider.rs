use crate::types::*;
use async_trait::async_trait;
use futures::StreamExt;
use orca_providers::provider::{ChunkStream, ModelProvider, ProviderError};
use orca_providers::types::*;
use orca_utils::message::{Message, Role, ToolCall};

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model_id: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model_id: model_id.into(),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    fn convert_messages(messages: &[Message]) -> Vec<AnthropicMessage> {
        let mut result = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    // System messages become the system parameter, handled separately
                    continue;
                }
                Role::User => {
                    if msg.tool_call_id.is_some() {
                        // This is a tool result message
                        result.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: msg.tool_call_id.clone().unwrap(),
                                content: msg.content.clone(),
                                is_error: None,
                            }]),
                        });
                    } else {
                        result.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Text(msg.content.clone()),
                        });
                    }
                }
                Role::Assistant => {
                    let mut blocks = Vec::new();
                    if !msg.content.is_empty() {
                        blocks.push(ContentBlock::Text { text: msg.content.clone() });
                    }
                    for tc in &msg.tool_calls {
                        blocks.push(ContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }
                    if blocks.is_empty() {
                        blocks.push(ContentBlock::Text { text: String::new() });
                    }
                    result.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Blocks(blocks),
                    });
                }
                Role::Tool => {
                    // Tool results are handled via User role with tool_result blocks
                    result.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Blocks(vec![ContentBlock::ToolResult {
                            tool_use_id: msg.tool_call_id.clone().unwrap_or_default(),
                            content: msg.content.clone(),
                            is_error: None,
                        }]),
                    });
                }
            }
        }

        result
    }

    fn convert_tools(tools: &[orca_utils::types::ToolDefinition]) -> Vec<AnthropicTool> {
        tools.iter().map(|t| AnthropicTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.parameters.clone(),
        }).collect()
    }

    fn extract_system(messages: &[Message]) -> Option<String> {
        let systems: Vec<&str> = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.as_str())
            .collect();
        if systems.is_empty() {
            None
        } else {
            Some(systems.join("\n\n"))
        }
    }

    fn convert_response(resp: AnthropicResponse) -> Result<ChatResponse, ProviderError> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &resp.content {
            match block {
                ContentBlock::Text { text } => content.push_str(text),
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: input.clone(),
                    });
                }
                _ => {}
            }
        }

        let stop_reason = match resp.stop_reason.as_deref() {
            Some("end_turn") => StopReason::EndTurn,
            Some("tool_use") => StopReason::ToolUse,
            Some("max_tokens") => StopReason::MaxTokens,
            other => StopReason::Unknown(other.unwrap_or("none").to_string()),
        };

        Ok(ChatResponse {
            content,
            tool_calls,
            usage: Usage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
                reasoning_tokens: None,
            },
            stop_reason,
            reasoning: None,
        })
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let system = request.system_prompt.clone()
            .or_else(|| Self::extract_system(&request.messages));
        let messages = Self::convert_messages(&request.messages);
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(Self::convert_tools(&request.tools))
        };

        let body = AnthropicRequest {
            model: self.model_id.clone(),
            max_tokens: request.max_tokens.unwrap_or(8192),
            messages,
            system,
            tools,
            temperature: request.temperature,
            stream: false,
        };

        let resp = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let api_resp: AnthropicResponse = resp.json().await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        Self::convert_response(api_resp)
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChunkStream, ProviderError> {
        let system = request.system_prompt.clone()
            .or_else(|| Self::extract_system(&request.messages));
        let messages = Self::convert_messages(&request.messages);
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(Self::convert_tools(&request.tools))
        };

        let body = AnthropicRequest {
            model: self.model_id.clone(),
            max_tokens: request.max_tokens.unwrap_or(8192),
            messages,
            system,
            tools,
            temperature: request.temperature,
            stream: true,
        };

        let resp = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        // Track state for assembling tool calls across stream chunks
        let mut tool_call_ids: Vec<Option<String>> = Vec::new();
        let mut tool_call_names: Vec<Option<String>> = Vec::new();
        let mut tool_call_args: Vec<String> = Vec::new();

        let chunk_stream = stream.filter_map(move |chunk_result| {
            let data = match chunk_result {
                Ok(bytes) => bytes,
                Err(e) => return futures::future::ready(Some(Err(
                    ProviderError::StreamError(e.to_string())
                ))),
            };

            let text = String::from_utf8_lossy(&data);

            // Parse SSE events
            for line in text.lines() {
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..];

                if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                    match event {
                        StreamEvent::ContentBlockDelta { index, delta } => {
                            match delta {
                                ContentDelta::TextDelta { text } => {
                                    return futures::future::ready(Some(Ok(StreamChunk {
                                        delta_content: Some(text),
                                        delta_tool_calls: Vec::new(),
                                        delta_reasoning: None,
                                        done: false,
                                        usage: None,
                                        stop_reason: None,
                                    })));
                                }
                                ContentDelta::InputJsonDelta { partial_json } => {
                                    // Accumulate tool call arguments
                                    while tool_call_args.len() <= index {
                                        tool_call_ids.push(None);
                                        tool_call_names.push(None);
                                        tool_call_args.push(String::new());
                                    }
                                    tool_call_args[index].push_str(&partial_json);
                                }
                            }
                        }
                        StreamEvent::ContentBlockStart { index, content_block: ContentBlock::ToolUse { id, name, .. } } => {
                                while tool_call_ids.len() <= index {
                                    tool_call_ids.push(None);
                                    tool_call_names.push(None);
                                    tool_call_args.push(String::new());
                                }
                                tool_call_ids[index] = Some(id);
                                tool_call_names[index] = Some(name);
                            }
                        StreamEvent::MessageDelta { delta, usage } => {
                            let stop_reason = delta.stop_reason.map(|r| match r.as_str() {
                                "end_turn" => StopReason::EndTurn,
                                "tool_use" => StopReason::ToolUse,
                                "max_tokens" => StopReason::MaxTokens,
                                _ => StopReason::Unknown(r),
                            });

                            let total_usage = usage.map(|u| Usage {
                                prompt_tokens: u.input_tokens,
                                completion_tokens: u.output_tokens,
                                total_tokens: u.input_tokens + u.output_tokens,
                                reasoning_tokens: None,
                            });

                            return futures::future::ready(Some(Ok(StreamChunk {
                                delta_content: None,
                                delta_tool_calls: Vec::new(),
                                delta_reasoning: None,
                                done: true,
                                usage: total_usage,
                                stop_reason,
                            })));
                        }
                        StreamEvent::MessageStop => {
                            return futures::future::ready(Some(Ok(StreamChunk {
                                delta_content: None,
                                delta_tool_calls: Vec::new(),
                                delta_reasoning: None,
                                done: true,
                                usage: None,
                                stop_reason: Some(StopReason::EndTurn),
                            })));
                        }
                        StreamEvent::Error { error } => {
                            return futures::future::ready(Some(Err(
                                ProviderError::StreamError(error.message)
                            )));
                        }
                        _ => {} // ping, message_start, content_block_stop - ignore
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
            display_name: format!("Claude {}", self.model_id),
            max_context_tokens: 200_000,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            input_cost_per_mtok: 3.0,
            output_cost_per_mtok: 15.0,
            is_flash: false,
        }
    }
}