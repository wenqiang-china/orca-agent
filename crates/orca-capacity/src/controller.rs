use crate::state::CanonicalState;
use serde::{Deserialize, Serialize};

/// Checkpoint type for evaluation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checkpoint {
    /// Before sending request to model — evaluate coherence
    PreRequest,
    /// After tool execution — verify state consistency
    PostTool,
    /// On error escalation — decide intervention level
    ErrorEscalation,
}

/// Intervention decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intervention {
    /// No intervention needed
    None,
    /// Warning message to include in context
    Warn(String),
    /// Replan: adjust goals
    Replan,
    /// Checkpoint restart: restore from last checkpoint
    CheckpointRestart,
    /// Abort the session
    Abort(String),
}

/// Configuration for the capacity controller
#[derive(Debug, Clone)]
pub struct CapacityConfig {
    /// Max open loops before warning
    pub max_open_loops: usize,
    /// Max active goals before warning
    pub max_active_goals: usize,
    /// Max consecutive errors before escalating
    pub max_consecutive_errors: u32,
    /// Max iteration count before warning
    pub max_iterations: u32,
    /// Context usage threshold (0.0-1.0) before warning
    pub context_usage_warn_threshold: f64,
    /// Context usage threshold for checkpoint restart
    pub context_usage_restart_threshold: f64,
}

impl Default for CapacityConfig {
    fn default() -> Self {
        Self {
            max_open_loops: 10,
            max_active_goals: 5,
            max_consecutive_errors: 3,
            max_iterations: 200,
            context_usage_warn_threshold: 0.75,
            context_usage_restart_threshold: 0.90,
        }
    }
}

/// Capacity controller that evaluates state at 3 checkpoints
pub struct CapacityController {
    config: CapacityConfig,
    consecutive_errors: u32,
    iteration_count: u32,
    current_context_usage: f64,
}

impl CapacityController {
    pub fn new(config: CapacityConfig) -> Self {
        Self {
            config,
            consecutive_errors: 0,
            iteration_count: 0,
            current_context_usage: 0.0,
        }
    }

    /// Evaluate the current state at a checkpoint
    pub fn evaluate(&mut self, state: &CanonicalState, checkpoint: Checkpoint) -> Intervention {
        match checkpoint {
            Checkpoint::PreRequest => self.evaluate_pre_request(state),
            Checkpoint::PostTool => self.evaluate_post_tool(state),
            Checkpoint::ErrorEscalation => self.evaluate_error_escalation(state),
        }
    }

    fn evaluate_pre_request(&mut self, state: &CanonicalState) -> Intervention {
        // Check context usage
        if self.current_context_usage >= self.config.context_usage_restart_threshold {
            return Intervention::CheckpointRestart;
        }

        if self.current_context_usage >= self.config.context_usage_warn_threshold {
            return Intervention::Warn(format!(
                "Context usage at {:.0}%. Consider wrapping up current work.",
                self.current_context_usage * 100.0
            ));
        }

        // Check open loops
        let critical_loops = state.open_loops.iter()
            .filter(|l| l.priority >= crate::state::LoopPriority::Critical)
            .count();
        if critical_loops > 0 {
            return Intervention::Warn(format!(
                "{} critical open loops remain unaddressed.",
                critical_loops
            ));
        }

        if state.open_loops.len() > self.config.max_open_loops {
            return Intervention::Replan;
        }

        // Check active goals
        if state.active_goal_count() > self.config.max_active_goals {
            return Intervention::Warn(format!(
                "Too many active goals ({}). Consider completing some before starting new ones.",
                state.active_goal_count()
            ));
        }

        // Check iteration count
        if self.iteration_count >= self.config.max_iterations {
            return Intervention::Abort(format!(
                "Exceeded maximum iterations ({}). Aborting.",
                self.config.max_iterations
            ));
        }

        if self.iteration_count >= self.config.max_iterations * 80 / 100 {
            return Intervention::Warn(format!(
                "Approaching iteration limit ({}/{}).",
                self.iteration_count, self.config.max_iterations
            ));
        }

        Intervention::None
    }

    fn evaluate_post_tool(&self, state: &CanonicalState) -> Intervention {
        // After tool execution, check if all goals are now done
        if state.all_goals_done() && state.open_loops.is_empty() {
            tracing::info!("all goals completed and no open loops");
        }

        // Check for too many facts (could indicate stalling)
        if state.facts.len() > 100 {
            return Intervention::Warn(
                "Accumulated many facts. Consider synthesizing findings.".to_string()
            );
        }

        Intervention::None
    }

    fn evaluate_error_escalation(&mut self, _state: &CanonicalState) -> Intervention {
        self.consecutive_errors += 1;

        if self.consecutive_errors >= self.config.max_consecutive_errors {
            return Intervention::CheckpointRestart;
        }

        if self.consecutive_errors > self.config.max_consecutive_errors / 2 {
            return Intervention::Warn(format!(
                "Consecutive errors: {}. Consider changing approach.",
                self.consecutive_errors
            ));
        }

        Intervention::None
    }

    /// Record a successful operation (resets error counter)
    pub fn record_success(&mut self) {
        self.consecutive_errors = 0;
        self.iteration_count += 1;
    }

    /// Record an iteration (even on failure)
    pub fn record_iteration(&mut self) {
        self.iteration_count += 1;
    }

    /// Update context usage (0.0-1.0)
    pub fn set_context_usage(&mut self, usage: f64) {
        self.current_context_usage = usage.clamp(0.0, 1.0);
    }

    /// Get current iteration count
    pub fn iteration_count(&self) -> u32 {
        self.iteration_count
    }

    /// Reset the controller
    pub fn reset(&mut self) {
        self.consecutive_errors = 0;
        self.iteration_count = 0;
        self.current_context_usage = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CanonicalState;

    #[test]
    fn test_normal_operation() {
        let mut ctrl = CapacityController::new(CapacityConfig::default());
        let state = CanonicalState::new();
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::PreRequest), Intervention::None));
    }

    #[test]
    fn test_context_usage_warning() {
        let mut ctrl = CapacityController::new(CapacityConfig::default());
        let state = CanonicalState::new();
        ctrl.set_context_usage(0.80);
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::PreRequest), Intervention::Warn(_)));
    }

    #[test]
    fn test_context_usage_restart() {
        let mut ctrl = CapacityController::new(CapacityConfig::default());
        let state = CanonicalState::new();
        ctrl.set_context_usage(0.95);
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::PreRequest), Intervention::CheckpointRestart));
    }

    #[test]
    fn test_consecutive_errors_escalate() {
        let mut ctrl = CapacityController::new(CapacityConfig {
            max_consecutive_errors: 3,
            ..Default::default()
        });
        let state = CanonicalState::new();

        // First two errors give warnings
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::ErrorEscalation), Intervention::None));
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::ErrorEscalation), Intervention::Warn(_)));

        // Third error triggers restart
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::ErrorEscalation), Intervention::CheckpointRestart));
    }

    #[test]
    fn test_max_iterations_abort() {
        let mut ctrl = CapacityController::new(CapacityConfig {
            max_iterations: 5,
            ..Default::default()
        });
        let state = CanonicalState::new();

        for _ in 0..5 {
            ctrl.record_iteration();
        }
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::PreRequest), Intervention::Abort(_)));
    }

    #[test]
    fn test_success_resets_errors() {
        let mut ctrl = CapacityController::new(CapacityConfig {
            max_consecutive_errors: 3,
            ..Default::default()
        });

        let state = CanonicalState::new();
        ctrl.evaluate(&state, Checkpoint::ErrorEscalation);
        ctrl.evaluate(&state, Checkpoint::ErrorEscalation);
        ctrl.record_success();
        // Should not escalate after reset
        assert!(matches!(ctrl.evaluate(&state, Checkpoint::ErrorEscalation), Intervention::None));
    }
}