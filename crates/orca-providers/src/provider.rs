use crate::types::{ChatRequest, ChatResponse, ModelInfo, StreamChunk};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Type alias for a stream of chunks
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>;

/// Errors from providers
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("authentication failed: {0}")]
    AuthError(String),

    #[error("rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("request too large: {0}")]
    RequestTooLarge(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("stream error: {0}")]
    StreamError(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("provider error: {0}")]
    Other(String),
}

/// Unified interface for all model providers
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Send a non-streaming chat request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// Send a streaming chat request
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChunkStream, ProviderError>;

    /// Get info about the current model
    fn model_info(&self) -> ModelInfo;

    /// Check if this provider supports reasoning_content
    fn supports_reasoning(&self) -> bool {
        false
    }
}