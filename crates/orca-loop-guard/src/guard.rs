use std::collections::{HashMap, VecDeque};
use orca_utils::message::ToolCall;

/// Default sliding window size
const DEFAULT_WINDOW_SIZE: usize = 6;
/// Default repetition threshold
const DEFAULT_REPETITION_THRESHOLD: usize = 3;
/// Default max consecutive failures before blocking
const DEFAULT_MAX_CONSECUTIVE_FAILURES: u32 = 5;

/// Action to take when repetition is detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepetitionAction {
    /// Allow the call
    Allow,
    /// Suppress silently (don't send to model)
    SuppressSilently,
    /// Suppress but inject a correction prompt
    SuppressWithCorrection(String),
    /// Block and escalate
    Block(String),
}

/// Entry in the sliding window
#[derive(Debug, Clone)]
struct WindowEntry {
    tool_name: String,
    arguments_hash: String,
    is_mutation: bool,
}

/// Loop guard that detects and prevents repetitive tool calls
pub struct LoopGuard {
    /// Sliding window of recent tool calls
    window: VecDeque<WindowEntry>,
    /// Window size
    window_size: usize,
    /// Number of repetitions before triggering
    repetition_threshold: usize,
    /// Consecutive failure counts per tool
    failure_counts: HashMap<String, u32>,
    /// Max consecutive failures before blocking
    max_consecutive_failures: u32,
    /// Total suppressed calls
    suppressed_count: u32,
    /// Whether the guard is currently in block mode
    blocked: bool,
    /// Reason for blocking
    block_reason: Option<String>,
}

impl Default for LoopGuard {
    fn default() -> Self {
        Self {
            window: VecDeque::new(),
            window_size: DEFAULT_WINDOW_SIZE,
            repetition_threshold: DEFAULT_REPETITION_THRESHOLD,
            failure_counts: HashMap::new(),
            max_consecutive_failures: DEFAULT_MAX_CONSECUTIVE_FAILURES,
            suppressed_count: 0,
            blocked: false,
            block_reason: None,
        }
    }
}

impl LoopGuard {
    pub fn new(window_size: usize, repetition_threshold: usize, max_consecutive_failures: u32) -> Self {
        Self {
            window_size,
            repetition_threshold,
            max_consecutive_failures,
            ..Default::default()
        }
    }

    /// Check if a tool call should be allowed, suppressed, or blocked
    pub fn check(&mut self, call: &ToolCall) -> RepetitionAction {
        // If already blocked, reject everything
        if self.blocked {
            return RepetitionAction::Block(
                self.block_reason.clone().unwrap_or_else(|| "loop guard blocked".to_string())
            );
        }

        let is_mutation = self.is_mutation_call(&call.name);
        let args_hash = orca_utils::hash::sha256_str(&call.arguments.to_string());

        // If this is a mutation call, clear read-only entries from window
        // (mutations advance state, so read-only repetitions are no longer loops)
        if is_mutation {
            self.clear_readonly_from_window();
        }

        // Count repetitions in window
        let repetition_count = self.window.iter().filter(|entry| {
            entry.tool_name == call.name && entry.arguments_hash == args_hash
        }).count();

        // Add to window
        let entry = WindowEntry {
            tool_name: call.name.clone(),
            arguments_hash: args_hash,
            is_mutation,
        };
        self.window.push_back(entry);

        // Trim window
        while self.window.len() > self.window_size {
            self.window.pop_front();
        }

        // Check mutation repetition (more dangerous)
        // Mutations trigger at threshold - 1 total calls
        if is_mutation && repetition_count >= self.repetition_threshold.saturating_sub(2) {
            self.suppressed_count += 1;
            tracing::warn!(
                tool = %call.name,
                count = repetition_count + 1,
                "repetitive mutation call detected"
            );
            return RepetitionAction::SuppressWithCorrection(format!(
                "You have called `{}` with the same arguments {} times in a row. \
                 This appears to be a loop. Try a different approach or explain what you're trying to accomplish.",
                call.name, repetition_count + 1
            ));
        }

        // Check read-only repetition
        if !is_mutation && repetition_count >= self.repetition_threshold.saturating_sub(1) {
            self.suppressed_count += 1;
            tracing::warn!(
                tool = %call.name,
                count = repetition_count + 1,
                "repetitive read-only call detected"
            );
            if self.suppressed_count > 10 {
                return RepetitionAction::SuppressSilently;
            }
            return RepetitionAction::SuppressWithCorrection(format!(
                "You've called `{}` with the same arguments {} times. \
                 The result won't change. Please try a different tool or approach.",
                call.name, repetition_count + 1
            ));
        }

        // Reset suppressed count on non-repetitive call
        if repetition_count == 0 {
            self.suppressed_count = 0;
        }

        RepetitionAction::Allow
    }

    /// Record a tool execution failure
    pub fn record_failure(&mut self, tool_name: &str) {
        let count = self.failure_counts.entry(tool_name.to_string()).or_insert(0);
        *count += 1;

        if *count >= self.max_consecutive_failures {
            self.blocked = true;
            self.block_reason = Some(format!(
                "Tool `{}` has failed {} consecutive times. Aborting to prevent infinite loop.",
                tool_name, count
            ));
            tracing::error!(
                tool = tool_name,
                count = count,
                "tool has exceeded max consecutive failures, blocking loop guard"
            );
        }
    }

    /// Record a tool execution success (resets failure count)
    pub fn record_success(&mut self, tool_name: &str) {
        self.failure_counts.insert(tool_name.to_string(), 0);
    }

    /// Clear read-only entries from the window when a mutation happens
    fn clear_readonly_from_window(&mut self) {
        self.window.retain(|entry| entry.is_mutation);
    }

    /// Determine if a tool call is a mutation (write operation)
    fn is_mutation_call(&self, name: &str) -> bool {
        matches!(
            name,
            "write_file" | "edit_file" | "execute_shell" | "create_file" | "delete_file"
                | "write" | "edit" | "create" | "delete" | "move" | "rename"
        )
    }

    /// Check if the guard is blocked
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Get the block reason
    pub fn block_reason(&self) -> Option<&str> {
        self.block_reason.as_deref()
    }

    /// Reset the guard to initial state
    pub fn reset(&mut self) {
        self.window.clear();
        self.failure_counts.clear();
        self.suppressed_count = 0;
        self.blocked = false;
        self.block_reason = None;
    }

    /// Get current window contents (for diagnostics)
    pub fn window_snapshot(&self) -> Vec<(&str, &str)> {
        self.window.iter().map(|e| (e.tool_name.as_str(), e.arguments_hash.as_str())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_call(name: &str, args: serde_json::Value) -> ToolCall {
        ToolCall {
            id: "test-1".to_string(),
            name: name.to_string(),
            arguments: args,
        }
    }

    #[test]
    fn test_no_repetition() {
        let mut guard = LoopGuard::default();
        let call = make_call("read_file", json!({"path": "/a.txt"}));
        assert_eq!(guard.check(&call), RepetitionAction::Allow);
    }

    #[test]
    fn test_readonly_repetition_detected() {
        let mut guard = LoopGuard::new(6, 3, 5);
        let call = make_call("read_file", json!({"path": "/a.txt"}));

        assert_eq!(guard.check(&call), RepetitionAction::Allow); // 1st
        assert_eq!(guard.check(&call), RepetitionAction::Allow); // 2nd
        let action = guard.check(&call); // 3rd - triggers
        assert!(matches!(action, RepetitionAction::SuppressWithCorrection(_)));
    }

    #[test]
    fn test_mutation_repetition_detected_earlier() {
        let mut guard = LoopGuard::new(6, 3, 5);
        let call = make_call("write_file", json!({"path": "/a.txt", "content": "x"}));

        assert_eq!(guard.check(&call), RepetitionAction::Allow); // 1st
        let action = guard.check(&call); // 2nd - triggers for mutations (threshold-1)
        assert!(matches!(action, RepetitionAction::SuppressWithCorrection(_)));
    }

    #[test]
    fn test_different_args_not_counted_as_repetition() {
        let mut guard = LoopGuard::new(6, 3, 5);
        for i in 0..5 {
            let call = make_call("read_file", json!({"path": format!("/{}.txt", i)}));
            assert_eq!(guard.check(&call), RepetitionAction::Allow);
        }
    }

    #[test]
    fn test_consecutive_failures_block() {
        let mut guard = LoopGuard::new(6, 3, 3);
        guard.record_failure("bad_tool");
        guard.record_failure("bad_tool");
        guard.record_failure("bad_tool");
        assert!(guard.is_blocked());
        assert!(guard.block_reason().unwrap().contains("bad_tool"));
    }

    #[test]
    fn test_success_resets_failure_count() {
        let mut guard = LoopGuard::new(6, 3, 3);
        guard.record_failure("tool");
        guard.record_failure("tool");
        guard.record_success("tool");
        guard.record_failure("tool");
        assert!(!guard.is_blocked());
    }

    #[test]
    fn test_reset() {
        let mut guard = LoopGuard::new(6, 3, 3);
        guard.record_failure("tool");
        guard.record_failure("tool");
        guard.record_failure("tool");
        assert!(guard.is_blocked());
        guard.reset();
        assert!(!guard.is_blocked());
    }

    #[test]
    fn test_mutation_clears_readonly_window() {
        let mut guard = LoopGuard::new(6, 3, 5);
        // Add some read-only calls
        let read = make_call("read_file", json!({"path": "/a.txt"}));
        guard.check(&read);
        guard.check(&read);

        // Mutation clears read-only entries
        let write = make_call("write_file", json!({"path": "/a.txt", "content": "x"}));
        guard.check(&write);

        // The read-only entries should have been cleared
        let snapshot = guard.window_snapshot();
        assert!(snapshot.iter().all(|(name, _)| *name == "write_file"));
    }
}
