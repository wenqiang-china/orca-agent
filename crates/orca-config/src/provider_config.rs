use serde::{Deserialize, Serialize};

/// Configuration for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API key (or env var reference like "$DEEPSEEK_API_KEY")
    pub api_key: String,
    /// Base URL for the API
    #[serde(default)]
    pub base_url: Option<String>,
    /// Available models with this provider
    #[serde(default)]
    pub models: Vec<ModelConfig>,
    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,
}

/// Configuration for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model ID (e.g., "deepseek-chat", "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name
    #[serde(default)]
    pub name: Option<String>,
    /// Max output tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Cost per 1M input tokens (USD)
    #[serde(default)]
    pub input_cost_per_mtok: f64,
    /// Cost per 1M output tokens (USD)
    #[serde(default)]
    pub output_cost_per_mtok: f64,
    /// Whether this model supports reasoning_content
    #[serde(default)]
    pub supports_reasoning: bool,
    /// Whether this model is a "flash" model (used for cheap summarization)
    #[serde(default)]
    pub is_flash: bool,
}

fn default_max_concurrent() -> usize { 4 }
fn default_request_timeout() -> u64 { 120 }
fn default_max_tokens() -> usize { 8192 }

impl ProviderConfig {
    /// Resolve API key from env var if it starts with $
    pub fn resolve_api_key(&self) -> String {
        if let Some(var_name) = self.api_key.strip_prefix('$') {
            std::env::var(var_name).unwrap_or_default()
        } else {
            self.api_key.clone()
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            models: Vec::new(),
            max_concurrent: 4,
            request_timeout_secs: 120,
        }
    }
}