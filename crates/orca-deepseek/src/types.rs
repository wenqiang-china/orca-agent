use serde::{Deserialize, Serialize};

/// DeepSeek API request format
#[derive(Debug, Clone, Serialize)]
pub struct DeepSeekRequest {
    pub model: String,
    pub messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<DeepSeekTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<DeepSeekToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: DeepSeekFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeepSeekTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: DeepSeekToolFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeepSeekToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// DeepSeek API response format (non-streaming)
#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekResponse {
    pub id: String,
    pub choices: Vec<DeepSeekChoice>,
    pub usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekChoice {
    pub message: DeepSeekMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    #[serde(default)]
    pub prompt_cache_hit_tokens: Option<usize>,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: Option<usize>,
}

/// DeepSeek SSE stream chunk
#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekStreamChunk {
    pub id: String,
    pub choices: Vec<DeepSeekStreamChoice>,
    pub usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekStreamChoice {
    pub delta: DeepSeekDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<DeepSeekToolCallDelta>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekToolCallDelta {
    pub index: usize,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<DeepSeekFunctionDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekFunctionDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}
