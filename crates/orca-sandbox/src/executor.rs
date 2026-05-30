use crate::policy::{NetworkPolicy, SandboxPolicy};
use crate::sandbox_trait::{ExecutionResult, Sandbox, SandboxError, SandboxProfile};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;

/// Default sandboxed executor using tokio::process with policy enforcement
/// On macOS: could use Seatbelt profiles; on Linux: Landlock/bwrap
/// For now, uses application-level enforcement (command filtering, timeout, output limits)
#[derive(Default)]
pub struct SandboxedExecutor {
    /// Enforce OS-level sandbox when available
    #[allow(dead_code)]
    enforce_os_sandbox: bool,
}

impl SandboxedExecutor {
    pub fn new(enforce_os_sandbox: bool) -> Self {
        Self { enforce_os_sandbox }
    }

    /// Truncate output to max_bytes, appending a truncation notice if needed
    fn truncate_output(output: &str, max_bytes: usize) -> String {
        if output.len() <= max_bytes {
            return output.to_string();
        }
        let truncated = &output[..max_bytes];
        format!("{}... [truncated, {} total bytes]", truncated, output.len())
    }
}

#[async_trait]
impl Sandbox for SandboxedExecutor {
    fn create_profile(&self, policy: SandboxPolicy, working_dir: PathBuf) -> SandboxProfile {
        SandboxProfile {
            working_dir,
            env_vars: Vec::new(),
            policy,
        }
    }

    async fn execute(&self, profile: &SandboxProfile, cmd: &str) -> Result<ExecutionResult, SandboxError> {
        // Check policy
        if !profile.policy.is_command_allowed(cmd) {
            return Err(SandboxError::CommandBlocked(cmd.to_string()));
        }

        // Enforce timeout
        let timeout = profile.policy.timeout.min(profile.policy.max_timeout);

        let start = Instant::now();

        // Build command
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(cmd)
            .current_dir(&profile.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set environment variables
        for (key, value) in &profile.env_vars {
            command.env(key, value);
        }

        // Clear potentially dangerous env vars
        command.env_remove("LD_PRELOAD");
        command.env_remove("DYLD_INSERT_LIBRARIES");

        // Network policy via env (informational for now)
        if profile.policy.network_policy == NetworkPolicy::Denied {
            command.env("ORCA_NETWORK_DENIED", "1");
        }

        // Execute with timeout
        match tokio::time::timeout(timeout, command.output()).await {
            Ok(Ok(output)) => {
                let duration = start.elapsed();
                let stdout = Self::truncate_output(
                    &String::from_utf8_lossy(&output.stdout),
                    profile.policy.max_output_bytes,
                );
                let stderr = Self::truncate_output(
                    &String::from_utf8_lossy(&output.stderr),
                    profile.policy.max_output_bytes,
                );
                let exit_code = output.status.code();

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    duration,
                    was_killed: false,
                })
            }
            Ok(Err(e)) => Err(SandboxError::ExecutionFailed(e.to_string())),
            Err(_) => Ok(ExecutionResult {
                stdout: String::new(),
                stderr: format!("command timed out after {:?}", timeout),
                exit_code: None,
                duration: timeout,
                was_killed: true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_execution() {
        let executor = SandboxedExecutor::default();
        let policy = SandboxPolicy::default();
        let profile = executor.create_profile(policy, PathBuf::from("/tmp"));

        let result = executor.execute(&profile, "echo hello").await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_blocked_command() {
        let executor = SandboxedExecutor::default();
        let policy = SandboxPolicy::default();
        let profile = executor.create_profile(policy, PathBuf::from("/tmp"));

        let err = executor.execute(&profile, "rm -rf /").await.unwrap_err();
        assert!(matches!(err, SandboxError::CommandBlocked(_)));
    }

    #[tokio::test]
    async fn test_timeout() {
        let executor = SandboxedExecutor::default();
        let policy = SandboxPolicy {
            timeout: Duration::from_millis(500),
            max_timeout: Duration::from_millis(500),
            ..Default::default()
        };
        let profile = executor.create_profile(policy, PathBuf::from("/tmp"));

        let result = executor.execute(&profile, "sleep 10").await.unwrap();
        assert!(result.was_killed);
    }

    #[tokio::test]
    async fn test_exit_code() {
        let executor = SandboxedExecutor::default();
        let policy = SandboxPolicy::default();
        let profile = executor.create_profile(policy, PathBuf::from("/tmp"));

        let result = executor.execute(&profile, "exit 42").await.unwrap();
        assert_eq!(result.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_output_truncation() {
        let executor = SandboxedExecutor::default();
        let policy = SandboxPolicy {
            max_output_bytes: 50,
            ..Default::default()
        };
        let profile = executor.create_profile(policy, PathBuf::from("/tmp"));

        let result = executor
            .execute(&profile, "python3 -c \"print('x' * 1000)\"")
            .await
            .unwrap();
        assert!(result.stdout.len() < 200); // Should be truncated
    }
}