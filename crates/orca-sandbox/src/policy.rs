use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Network access policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkPolicy {
    Denied,
    Restricted(Vec<String>), // Allowed domains
    Full,
}

/// Sandbox execution policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Read-only paths (system paths are always read-only)
    pub read_only_paths: Vec<PathBuf>,
    /// Writable paths (project root is always writable)
    pub writable_paths: Vec<PathBuf>,
    /// Network access policy
    pub network_policy: NetworkPolicy,
    /// Command execution timeout
    pub timeout: Duration,
    /// Maximum timeout (hard limit)
    pub max_timeout: Duration,
    /// Allowed commands (empty = all allowed, subject to other constraints)
    pub allowed_commands: Vec<String>,
    /// Blocked commands (always denied)
    pub blocked_commands: Vec<String>,
    /// Maximum output size in bytes
    pub max_output_bytes: usize,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            read_only_paths: vec![
                PathBuf::from("/"),
                PathBuf::from("/usr"),
                PathBuf::from("/System"),
                PathBuf::from("/Library"),
            ],
            writable_paths: Vec::new(),
            network_policy: NetworkPolicy::Denied,
            timeout: Duration::from_secs(120),
            max_timeout: Duration::from_secs(600),
            allowed_commands: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                ":(){:|:&};:".to_string(),
            ],
            max_output_bytes: 1024 * 1024, // 1 MiB
        }
    }
}

impl SandboxPolicy {
    /// Create a permissive policy for development use
    pub fn permissive() -> Self {
        Self {
            network_policy: NetworkPolicy::Full,
            timeout: Duration::from_secs(300),
            ..Default::default()
        }
    }

    /// Create a strict policy for untrusted code
    pub fn strict() -> Self {
        Self {
            network_policy: NetworkPolicy::Denied,
            timeout: Duration::from_secs(60),
            max_output_bytes: 512 * 1024,
            ..Default::default()
        }
    }

    /// Check if a command is allowed by the policy
    pub fn is_command_allowed(&self, cmd: &str) -> bool {
        let cmd_name = cmd.split_whitespace().next().unwrap_or("");

        // Check blocked list
        for blocked in &self.blocked_commands {
            if cmd.starts_with(blocked) || cmd_name == *blocked {
                return false;
            }
        }

        // If allowed list is empty, everything not blocked is allowed
        if self.allowed_commands.is_empty() {
            return true;
        }

        // Check allowed list
        self.allowed_commands.iter().any(|a| cmd_name == *a || cmd.starts_with(a))
    }
}