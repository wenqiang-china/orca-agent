use chrono::{DateTime, Utc};
use orca_capacity::CanonicalState;
use orca_utils::message::Conversation;
use uuid::Uuid;

/// A session represents a single agent conversation
pub struct Session {
    pub id: String,
    pub conversation: Conversation,
    pub state: CanonicalState,
    pub total_cost_usd: f64,
    pub iteration_count: u32,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation: Conversation::new(model_id),
            state: CanonicalState::new(),
            total_cost_usd: 0.0,
            iteration_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn add_cost(&mut self, cost: f64) {
        self.total_cost_usd += cost;
    }

    pub fn increment_iteration(&mut self) {
        self.iteration_count += 1;
    }

    pub fn context_usage_ratio(&self, max_tokens: usize) -> f64 {
        let estimated_tokens = self.conversation.total_chars() / 4; // rough estimate
        estimated_tokens as f64 / max_tokens as f64
    }
}