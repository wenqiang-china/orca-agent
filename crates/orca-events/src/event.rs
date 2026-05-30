use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A logged event in the agent's lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub session_id: String,
    pub kind: EventKind,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Types of events we track
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    SessionStart,
    SessionEnd,
    MessageSent,
    MessageReceived,
    ToolCallStart,
    ToolCallEnd,
    ToolCallError,
    CheckpointCreated,
    CheckpointRestored,
    CapacityWarning,
    LoopDetected,
    SandboxViolation,
    BudgetWarning,
    ModelSwitch,
    Error,
}

impl Event {
    pub fn new(session_id: impl Into<String>, kind: EventKind, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            kind,
            data,
            timestamp: Utc::now(),
        }
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionStart => write!(f, "session_start"),
            Self::SessionEnd => write!(f, "session_end"),
            Self::MessageSent => write!(f, "message_sent"),
            Self::MessageReceived => write!(f, "message_received"),
            Self::ToolCallStart => write!(f, "tool_call_start"),
            Self::ToolCallEnd => write!(f, "tool_call_end"),
            Self::ToolCallError => write!(f, "tool_call_error"),
            Self::CheckpointCreated => write!(f, "checkpoint_created"),
            Self::CheckpointRestored => write!(f, "checkpoint_restored"),
            Self::CapacityWarning => write!(f, "capacity_warning"),
            Self::LoopDetected => write!(f, "loop_detected"),
            Self::SandboxViolation => write!(f, "sandbox_violation"),
            Self::BudgetWarning => write!(f, "budget_warning"),
            Self::ModelSwitch => write!(f, "model_switch"),
            Self::Error => write!(f, "error"),
        }
    }
}
