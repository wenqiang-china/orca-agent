use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A saved checkpoint of conversation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    pub id: String,
    pub session_id: String,
    /// Compressed/archived messages (summary + recent context)
    pub seed_messages: Vec<orca_utils::message::Message>,
    /// Goals that have been completed
    pub completed_goals: Vec<CompletedGoal>,
    /// Goals still in progress
    pub active_goals: Vec<String>,
    /// Preserved anchor context
    pub anchor_context: String,
    /// Serialized canonical state snapshot
    pub state_snapshot: serde_json::Value,
    /// Iteration count at checkpoint time
    pub iteration_count: u32,
    /// Total cost at checkpoint time
    pub total_cost_usd: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Human-readable description
    pub description: String,
}

/// A completed goal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedGoal {
    pub description: String,
    pub completed_at: DateTime<Utc>,
    pub result_summary: String,
}

/// Lightweight summary of a checkpoint (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSummary {
    pub id: String,
    pub description: String,
    pub message_count: usize,
    pub active_goals: usize,
    pub completed_goals: usize,
    pub iteration_count: u32,
    pub total_cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

impl From<&CheckpointData> for CheckpointSummary {
    fn from(data: &CheckpointData) -> Self {
        Self {
            id: data.id.clone(),
            description: data.description.clone(),
            message_count: data.seed_messages.len(),
            active_goals: data.active_goals.len(),
            completed_goals: data.completed_goals.len(),
            iteration_count: data.iteration_count,
            total_cost_usd: data.total_cost_usd,
            created_at: data.created_at,
        }
    }
}

/// Restored session state from a checkpoint
#[derive(Debug, Clone)]
pub struct RestoredSession {
    /// Messages to seed the new conversation with
    pub seed_messages: Vec<orca_utils::message::Message>,
    /// Previously completed work
    pub completed_goals: Vec<CompletedGoal>,
    /// Anchor context string
    pub anchor_context: String,
    /// Serialized state to restore
    pub state_snapshot: serde_json::Value,
    /// Iteration count to resume from
    pub iteration_count: u32,
    /// Cost to resume from
    pub total_cost_usd: f64,
}
