use crate::registry::{Tool, ToolError};
use async_trait::async_trait;
use orca_utils::types::ToolDefinition;
use serde_json::{json, Value};

/// Git operations tool
pub struct GitTool {
    working_dir: std::path::PathBuf,
}

impl GitTool {
    pub fn new(working_dir: std::path::PathBuf) -> Self {
        Self { working_dir }
    }

    async fn run_git(&self, args: &[&str]) -> Result<String, ToolError> {
        let output = tokio::process::Command::new("git")
            .args(args)
            .current_dir(&self.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("git failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(ToolError::ExecutionFailed(
                if stderr.is_empty() {
                    stdout.to_string()
                } else {
                    stderr.to_string()
                }
            ))
        }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git".to_string(),
            description: "Run git commands. Supports: status, diff, log, show, add, commit, branch, checkout, stash, and raw commands via 'run'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Git action: status, diff, log, show, add, commit, branch, checkout, stash, run",
                        "enum": ["status", "diff", "log", "show", "add", "commit", "branch", "checkout", "stash", "run"]
                    },
                    "args": {
                        "type": "string",
                        "description": "Additional arguments for the action (e.g., file paths for add, message for commit, ref for show)"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of log entries to show (default: 10)",
                        "default": 10
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'action'".to_string()))?;

        let extra_args = args.get("args").and_then(|v| v.as_str()).unwrap_or("");
        let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10);

        match action {
            "status" => self.run_git(&["status", "--short"]).await,
            "diff" => {
                if extra_args.is_empty() {
                    self.run_git(&["diff"]).await
                } else {
                    let parts: Vec<&str> = extra_args.split_whitespace().collect();
                    let mut cmd_args = vec!["diff"];
                    cmd_args.extend_from_slice(&parts);
                    self.run_git(&cmd_args).await
                }
            }
            "log" => {
                let n = format!("-{}", count);
                self.run_git(&["log", &n, "--oneline"]).await
            }
            "show" => {
                if extra_args.is_empty() {
                    self.run_git(&["show", "HEAD"]).await
                } else {
                    self.run_git(&["show", extra_args]).await
                }
            }
            "add" => {
                if extra_args.is_empty() {
                    return Err(ToolError::InvalidArgs("specify files to add".to_string()));
                }
                let parts: Vec<&str> = extra_args.split_whitespace().collect();
                let mut cmd_args = vec!["add"];
                cmd_args.extend_from_slice(&parts);
                self.run_git(&cmd_args).await
            }
            "commit" => {
                if extra_args.is_empty() {
                    return Err(ToolError::InvalidArgs("specify commit message".to_string()));
                }
                self.run_git(&["commit", "-m", extra_args]).await
            }
            "branch" => {
                if extra_args.is_empty() {
                    self.run_git(&["branch"]).await
                } else {
                    let parts: Vec<&str> = extra_args.split_whitespace().collect();
                    let mut cmd_args = vec!["branch"];
                    cmd_args.extend_from_slice(&parts);
                    self.run_git(&cmd_args).await
                }
            }
            "checkout" => {
                if extra_args.is_empty() {
                    return Err(ToolError::InvalidArgs("specify branch or file".to_string()));
                }
                let parts: Vec<&str> = extra_args.split_whitespace().collect();
                let mut cmd_args = vec!["checkout"];
                cmd_args.extend_from_slice(&parts);
                self.run_git(&cmd_args).await
            }
            "stash" => {
                if extra_args.is_empty() {
                    self.run_git(&["stash"]).await
                } else {
                    let parts: Vec<&str> = extra_args.split_whitespace().collect();
                    let mut cmd_args = vec!["stash"];
                    cmd_args.extend_from_slice(&parts);
                    self.run_git(&cmd_args).await
                }
            }
            "run" => {
                if extra_args.is_empty() {
                    return Err(ToolError::InvalidArgs("specify git command".to_string()));
                }
                let parts: Vec<&str> = extra_args.split_whitespace().collect();
                self.run_git(&parts).await
            }
            _ => Err(ToolError::InvalidArgs(format!("unknown git action: {}", action))),
        }
    }

    fn requires_sandbox(&self) -> bool {
        false
    }
}