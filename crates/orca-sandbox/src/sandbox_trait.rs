use crate::policy::SandboxPolicy;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

/// Result of a sandboxed execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub was_killed: bool,
}

impl ExecutionResult {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: Some(0),
            duration: Duration::ZERO,
            was_killed: false,
        }
    }

    pub fn failure(stderr: impl Into<String>, exit_code: i32) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code: Some(exit_code),
            duration: Duration::ZERO,
            was_killed: false,
        }
    }

    pub fn timeout() -> Self {
        Self {
            stdout: String::new(),
            stderr: "command timed out".to_string(),
            exit_code: None,
            duration: Duration::ZERO,
            was_killed: true,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0) && !self.was_killed
    }
}

/// A sandbox profile defines the environment for execution
#[derive(Debug, Clone)]
pub struct SandboxProfile {
    pub working_dir: PathBuf,
    pub env_vars: Vec<(String, String)>,
    pub policy: SandboxPolicy,
}

/// Cross-platform sandbox trait
#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Create a sandbox profile for execution
    fn create_profile(&self, policy: SandboxPolicy, working_dir: PathBuf) -> SandboxProfile;

    /// Execute a command in the sandbox
    async fn execute(&self, profile: &SandboxProfile, cmd: &str) -> Result<ExecutionResult, SandboxError>;
}

/// Sandbox errors
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("command blocked by policy: {0}")]
    CommandBlocked(String),
    #[error("sandbox setup failed: {0}")]
    SetupFailed(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
}