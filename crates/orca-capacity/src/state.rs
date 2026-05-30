use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CanonicalState: semantic-level state tracking for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalState {
    /// Current goals the agent is working towards
    pub goals: Vec<Goal>,
    /// Hard constraints that must not be violated
    pub constraints: Vec<Constraint>,
    /// Known facts (key-value with provenance)
    pub facts: HashMap<String, Fact>,
    /// Unclosed loops (pending work, unanswered questions)
    pub open_loops: Vec<OpenLoop>,
    /// Timestamp of last update
    pub updated_at: DateTime<Utc>,
}

/// A goal the agent is pursuing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub status: GoalStatus,
    pub sub_goals: Vec<String>, // IDs of child goals
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

/// A hard constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub description: String,
    pub source: ConstraintSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintSource {
    UserSpecified,
    SystemDefault,
    Derived,
}

/// A known fact with provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub key: String,
    pub value: serde_json::Value,
    pub source: String,       // Where this fact came from
    pub confidence: f64,      // 0.0 - 1.0
    pub established_at: DateTime<Utc>,
}

/// An unclosed loop (pending work item)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLoop {
    pub id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub priority: LoopPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoopPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for CanonicalState {
    fn default() -> Self {
        Self {
            goals: Vec::new(),
            constraints: Vec::new(),
            facts: HashMap::new(),
            open_loops: Vec::new(),
            updated_at: Utc::now(),
        }
    }
}

impl CanonicalState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a goal
    pub fn add_goal(&mut self, description: impl Into<String>) -> &Goal {
        let id = uuid::Uuid::new_v4().to_string();
        self.goals.push(Goal {
            id: id.clone(),
            description: description.into(),
            status: GoalStatus::Pending,
            sub_goals: Vec::new(),
            parent_id: None,
            created_at: Utc::now(),
            completed_at: None,
        });
        self.updated_at = Utc::now();
        self.goals.last().unwrap()
    }

    /// Update goal status
    pub fn update_goal_status(&mut self, goal_id: &str, status: GoalStatus) {
        if let Some(goal) = self.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.status = status.clone();
            if status == GoalStatus::Completed || status == GoalStatus::Failed {
                goal.completed_at = Some(Utc::now());
            }
        }
        self.updated_at = Utc::now();
    }

    /// Add a fact
    pub fn add_fact(&mut self, key: impl Into<String>, value: serde_json::Value, source: impl Into<String>) {
        let key = key.into();
        self.facts.insert(key.clone(), Fact {
            key,
            value,
            source: source.into(),
            confidence: 1.0,
            established_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    /// Add an open loop
    pub fn add_open_loop(&mut self, description: impl Into<String>, priority: LoopPriority) {
        self.open_loops.push(OpenLoop {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            created_at: Utc::now(),
            priority,
        });
        self.updated_at = Utc::now();
    }

    /// Close an open loop
    pub fn close_loop(&mut self, loop_id: &str) {
        self.open_loops.retain(|l| l.id != loop_id);
        self.updated_at = Utc::now();
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, description: impl Into<String>, source: ConstraintSource) {
        self.constraints.push(Constraint {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            source,
        });
        self.updated_at = Utc::now();
    }

    /// Count active (non-completed, non-failed) goals
    pub fn active_goal_count(&self) -> usize {
        self.goals.iter().filter(|g| matches!(g.status, GoalStatus::Pending | GoalStatus::InProgress)).count()
    }

    /// Check if all goals are completed
    pub fn all_goals_done(&self) -> bool {
        !self.goals.is_empty() && self.goals.iter().all(|g| matches!(g.status, GoalStatus::Completed | GoalStatus::Failed))
    }

    /// Get a summary of current state
    pub fn summary(&self) -> String {
        let active = self.active_goal_count();
        let completed = self.goals.iter().filter(|g| g.status == GoalStatus::Completed).count();
        let failed = self.goals.iter().filter(|g| g.status == GoalStatus::Failed).count();
        let loops = self.open_loops.len();
        let facts = self.facts.len();
        let constraints = self.constraints.len();

        format!(
            "Goals: {} active, {} completed, {} failed | Constraints: {} | Facts: {} | Open loops: {}",
            active, completed, failed, constraints, facts, loops
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_add_and_complete_goal() {
        let mut state = CanonicalState::new();
        let goal = state.add_goal("implement feature X");
        let goal_id = goal.id.clone();

        assert_eq!(state.active_goal_count(), 1);
        assert!(!state.all_goals_done());

        state.update_goal_status(&goal_id, GoalStatus::Completed);
        assert_eq!(state.active_goal_count(), 0);
        assert!(state.all_goals_done());
    }

    #[test]
    fn test_facts() {
        let mut state = CanonicalState::new();
        state.add_fact("language", json!("Rust"), "user");
        assert_eq!(state.facts.len(), 1);
        assert_eq!(state.facts["language"].value, json!("Rust"));
    }

    #[test]
    fn test_open_loops() {
        let mut state = CanonicalState::new();
        state.add_open_loop("need to add error handling", LoopPriority::High);
        assert_eq!(state.open_loops.len(), 1);

        let loop_id = state.open_loops[0].id.clone();
        state.close_loop(&loop_id);
        assert_eq!(state.open_loops.len(), 0);
    }

    #[test]
    fn test_summary() {
        let mut state = CanonicalState::new();
        state.add_goal("test");
        state.add_constraint("no unsafe", ConstraintSource::UserSpecified);
        state.add_fact("x", json!(42), "test");
        let s = state.summary();
        assert!(s.contains("1 active"));
        assert!(s.contains("Constraints: 1"));
    }
}