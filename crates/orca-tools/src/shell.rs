use crate::registry::{Tool, ToolError};
use async_trait::async_trait;
use orca_utils::types::ToolDefinition;
use serde_json::{json, Value};

/// Execute a shell command (this tool is always sandboxed)
pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "execute_shell".to_string(),
            description: "Execute a shell command and return its output. Commands run in a sandboxed environment with a timeout.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 120, max: 600)",
                        "default": 120
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'command' argument".to_string()))?;

        let timeout_secs = args.get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(120)
            .min(600);

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            output
        ).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                if exit_code == 0 {
                    if stdout.is_empty() {
                        "(command completed successfully, no output)".to_string()
                    } else {
                        stdout.to_string()
                    }
                } else {
                    format!("exit code: {}\nstdout: {}\nstderr: {}", exit_code, stdout, stderr)
                }
            }
            Ok(Err(e)) => return Err(ToolError::ExecutionFailed(format!("failed to execute: {}", e))),
            Err(_) => return Err(ToolError::Timeout(format!("command timed out after {}s", timeout_secs))),
        };

        // Truncate very long output
        if output.len() > 50_000 {
            Ok(format!("{}...\n\n[output truncated, {} total chars]", &output[..50_000], output.len()))
        } else {
            Ok(output)
        }
    }

    fn requires_sandbox(&self) -> bool {
        true
    }
}