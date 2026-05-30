use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An anchor preserves critical context across compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    pub id: String,
    pub anchor_type: AnchorType,
    pub content: String,
    pub source_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnchorType {
    /// User's explicit instruction or goal
    UserGoal,
    /// A decision that was made
    Decision,
    /// A constraint the user specified
    Constraint,
    /// An important fact discovered
    KeyFact,
    /// Error or issue to remember
    ErrorContext,
}

/// Keeps track of user anchors that must be preserved across compression
pub struct AnchorKeeper {
    anchors: Vec<Anchor>,
    max_anchors: usize,
}

impl AnchorKeeper {
    pub fn new(max_anchors: usize) -> Self {
        Self {
            anchors: Vec::new(),
            max_anchors,
        }
    }

    /// Add a new anchor
    pub fn add(&mut self, anchor_type: AnchorType, content: impl Into<String>) -> &Anchor {
        let anchor = Anchor {
            id: uuid::Uuid::new_v4().to_string(),
            anchor_type,
            content: content.into(),
            source_message_id: None,
            created_at: Utc::now(),
        };
        self.anchors.push(anchor);

        // Trim if over limit, keeping most recent
        if self.anchors.len() > self.max_anchors {
            let drain_count = self.anchors.len() - self.max_anchors;
            self.anchors.drain(0..drain_count);
        }

        self.anchors.last().unwrap()
    }

    /// Remove an anchor by ID
    pub fn remove(&mut self, id: &str) {
        self.anchors.retain(|a| a.id != id);
    }

    /// Get all anchors
    pub fn all(&self) -> &[Anchor] {
        &self.anchors
    }

    /// Get anchors of a specific type
    pub fn by_type(&self, anchor_type: &AnchorType) -> Vec<&Anchor> {
        self.anchors.iter().filter(|a| &a.anchor_type == anchor_type).collect()
    }

    /// Format anchors as a context block for injection into messages
    pub fn format_as_context(&self) -> String {
        if self.anchors.is_empty() {
            return String::new();
        }

        let mut out = String::from("## Preserved Context\n\n");
        for anchor in &self.anchors {
            let label = match anchor.anchor_type {
                AnchorType::UserGoal => "GOAL",
                AnchorType::Decision => "DECISION",
                AnchorType::Constraint => "CONSTRAINT",
                AnchorType::KeyFact => "FACT",
                AnchorType::ErrorContext => "ERROR",
            };
            out.push_str(&format!("- [{}] {}\n", label, anchor.content));
        }
        out
    }

    pub fn count(&self) -> usize {
        self.anchors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve() {
        let mut keeper = AnchorKeeper::new(100);
        keeper.add(AnchorType::UserGoal, "implement feature X");
        assert_eq!(keeper.count(), 1);
        assert_eq!(keeper.by_type(&AnchorType::UserGoal).len(), 1);
    }

    #[test]
    fn test_max_anchors_trim() {
        let mut keeper = AnchorKeeper::new(3);
        keeper.add(AnchorType::UserGoal, "goal 1");
        keeper.add(AnchorType::UserGoal, "goal 2");
        keeper.add(AnchorType::UserGoal, "goal 3");
        keeper.add(AnchorType::UserGoal, "goal 4");
        assert_eq!(keeper.count(), 3);
        assert_eq!(keeper.all()[0].content, "goal 2"); // oldest trimmed
    }

    #[test]
    fn test_format_context() {
        let mut keeper = AnchorKeeper::new(100);
        keeper.add(AnchorType::UserGoal, "build a CLI");
        keeper.add(AnchorType::Constraint, "must be async");
        let ctx = keeper.format_as_context();
        assert!(ctx.contains("[GOAL]"));
        assert!(ctx.contains("[CONSTRAINT]"));
    }

    #[test]
    fn test_remove_anchor() {
        let mut keeper = AnchorKeeper::new(100);
        let anchor = keeper.add(AnchorType::KeyFact, "something");
        let id = anchor.id.clone();
        keeper.remove(&id);
        assert_eq!(keeper.count(), 0);
    }
}