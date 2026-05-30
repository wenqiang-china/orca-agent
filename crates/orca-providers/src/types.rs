use orca_utils::types::ModelId;
use serde::{Deserialize, Serialize};

/// A chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<orca_utils::message::Message>,
    pub tools: Vec<orca_utils::types::ToolDefinition>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
}

/// A complete (non-streamed) chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<orca_utils::message::ToolCall>,
    pub usage: Usage,
    pub stop_reason: StopReason,
    /// For models that support reasoning_content (DeepSeek R1)
    pub reasoning: Option<String>,
}

/// A single chunk in a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Delta content (appended to running content)
    pub delta_content: Option<String>,
    /// Delta tool call (partial)
    pub delta_tool_calls: Vec<DeltaToolCall>,
    /// Reasoning delta (for DeepSeek R1)
    pub delta_reasoning: Option<String>,
    /// Whether this is the final chunk
    pub done: bool,
    /// Usage info (only on final chunk for some providers)
    pub usage: Option<Usage>,
    /// Stop reason (only on final chunk)
    pub stop_reason: Option<StopReason>,
}

/// Partial tool call in a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaToolCall {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    /// Partial arguments string (append to accumulate)
    pub arguments_delta: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    /// Reasoning tokens (for DeepSeek R1)
    pub reasoning_tokens: Option<usize>,
}

impl Usage {
    pub fn cost_usd(&self, input_cost_per_mtok: f64, output_cost_per_mtok: f64) -> f64 {
        let input_cost = (self.prompt_tokens as f64 / 1_000_000.0) * input_cost_per_mtok;
        let output_cost = (self.completion_tokens as f64 / 1_000_000.0) * output_cost_per_mtok;
        input_cost + output_cost
    }
}

/// Why the model stopped generating
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    /// Model finished naturally (end_turn / stop)
    EndTurn,
    /// Model requested tool calls
    ToolUse,
    /// Hit max_tokens limit
    MaxTokens,
    /// Content was filtered
    ContentFilter,
    /// Unknown/other reason
    Unknown(String),
}

/// Information about a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: ModelId,
    pub display_name: String,
    pub max_context_tokens: usize,
    pub max_output_tokens: usize,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub is_flash: bool,
}

impl ModelInfo {
    pub fn cost_estimate(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_mtok;
        let output = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_mtok;
        input + output
    }
}