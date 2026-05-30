use crate::provider::{ModelProvider, ProviderError, ChunkStream};
use crate::types::{ChatRequest, ChatResponse, ModelInfo};
use orca_utils::types::ModelId;
use std::collections::HashMap;
use std::sync::Arc;

/// Routing strategy for selecting providers
#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Always use a fixed model
    Fixed(ModelId),
    /// Use cheapest available model
    CostOptimized,
    /// Use highest quality model
    QualityOptimized,
    /// Adaptive: try primary, fallback to specified model
    Adaptive { fallback: ModelId },
}

/// Routes requests to the appropriate provider
pub struct ProviderRouter {
    providers: HashMap<ModelId, Arc<dyn ModelProvider>>,
    strategy: RoutingStrategy,
    default_model: ModelId,
}

impl ProviderRouter {
    pub fn new(strategy: RoutingStrategy, default_model: ModelId) -> Self {
        Self {
            providers: HashMap::new(),
            strategy,
            default_model,
        }
    }

    /// Register a provider for a model
    pub fn register(&mut self, model_id: ModelId, provider: Arc<dyn ModelProvider>) {
        self.providers.insert(model_id, provider);
    }

    /// Resolve which model to use based on strategy
    pub fn resolve_model(&self) -> &ModelId {
        match &self.strategy {
            RoutingStrategy::Fixed(id) => id,
            RoutingStrategy::CostOptimized => {
                // Pick cheapest model that's available
                self.providers
                    .keys()
                    .min_by_key(|id| {
                        let info = self.providers.get(id).unwrap().model_info();
                        (info.input_cost_per_mtok * 1000.0) as u64
                    })
                    .unwrap_or(&self.default_model)
            }
            RoutingStrategy::QualityOptimized => {
                // Pick most expensive (typically highest quality)
                self.providers
                    .keys()
                    .max_by_key(|id| {
                        let info = self.providers.get(id).unwrap().model_info();
                        (info.output_cost_per_mtok * 1000.0) as u64
                    })
                    .unwrap_or(&self.default_model)
            }
            RoutingStrategy::Adaptive { fallback } => {
                // Try default first, fallback is used externally
                if self.providers.contains_key(&self.default_model) {
                    &self.default_model
                } else {
                    fallback
                }
            }
        }
    }

    /// Get a provider for a specific model
    fn get_provider(&self, model_id: &ModelId) -> Result<Arc<dyn ModelProvider>, ProviderError> {
        self.providers
            .get(model_id)
            .cloned()
            .ok_or_else(|| ProviderError::ModelNotFound(model_id.to_string()))
    }

    /// Execute a non-streaming request with routing
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model_id = self.resolve_model().clone();
        let provider = self.get_provider(&model_id)?;
        provider.chat(request).await
    }

    /// Execute a streaming request with routing
    pub async fn chat_stream(&self, request: ChatRequest) -> Result<ChunkStream, ProviderError> {
        let model_id = self.resolve_model().clone();
        let provider = self.get_provider(&model_id)?;
        provider.chat_stream(request).await
    }

    /// Execute request with fallback on error
    pub async fn chat_with_fallback(
        &self,
        request: ChatRequest,
        fallback_model: &ModelId,
    ) -> Result<ChatResponse, ProviderError> {
        let primary_id = self.resolve_model().clone();
        match self.get_provider(&primary_id)?.chat(request.clone()).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                tracing::warn!("primary provider failed: {e}, falling back to {fallback_model}");
                self.get_provider(fallback_model)?.chat(request).await
            }
        }
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        self.providers.values().map(|p| p.model_info()).collect()
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ModelInfo, Usage, StopReason};
    use async_trait::async_trait;

    // A mock provider for testing
    struct MockProvider {
        info: ModelInfo,
    }

    #[async_trait]
    impl ModelProvider for MockProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse {
                content: "test".to_string(),
                tool_calls: vec![],
                usage: Usage::default(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
            })
        }

        async fn chat_stream(&self, _request: ChatRequest) -> Result<ChunkStream, ProviderError> {
            unimplemented!()
        }

        fn model_info(&self) -> ModelInfo {
            self.info.clone()
        }
    }

    #[test]
    fn test_routing_strategy_fixed() {
        let mut router = ProviderRouter::new(
            RoutingStrategy::Fixed(ModelId::from("test-model")),
            ModelId::from("test-model"),
        );

        let info = ModelInfo {
            id: ModelId::from("test-model"),
            display_name: "Test Model".to_string(),
            max_context_tokens: 128000,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            input_cost_per_mtok: 0.0,
            output_cost_per_mtok: 0.0,
            is_flash: false,
        };

        router.register(
            ModelId::from("test-model"),
            Arc::new(MockProvider { info }),
        );

        assert_eq!(*router.resolve_model(), ModelId::from("test-model"));
    }
}