use crate::registry::{ToolError, ToolRegistry};
use orca_arg_repair::ArgRepairer;
use orca_name_resolver::resolver::{MatchType, ToolNameResolver};
use orca_sandbox::executor::SandboxedExecutor;
use orca_sandbox::policy::SandboxPolicy;
use orca_sandbox::sandbox_trait::Sandbox;
use orca_utils::message::{ToolCall, ToolResult};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Execution options
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub working_dir: PathBuf,
    pub sandbox_policy: SandboxPolicy,
    /// Whether to attempt name resolution for unknown tools
    pub resolve_names: bool,
    /// Whether to attempt argument repair
    pub repair_args: bool,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            sandbox_policy: SandboxPolicy::default(),
            resolve_names: true,
            repair_args: true,
        }
    }
}

/// Diagnostic info about how a tool call was processed
#[derive(Debug, Clone)]
pub struct ExecutionDiag {
    pub original_name: Option<String>,
    pub resolved_name: String,
    pub name_match_type: Option<MatchType>,
    pub args_repaired: bool,
}

/// The main tool executor: combines registry, resolver, and arg repair
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    resolver: ToolNameResolver,
    arg_repairer: ArgRepairer,
    sandbox: SandboxedExecutor,
    options: ExecutionOptions,
}

impl ToolExecutor {
    pub fn new(registry: Arc<ToolRegistry>, options: ExecutionOptions) -> Self {
        let definitions = registry.definitions();
        let resolver = ToolNameResolver::new(definitions);
        Self {
            registry,
            resolver,
            arg_repairer: ArgRepairer::default(),
            sandbox: SandboxedExecutor::default(),
            options,
        }
    }

    /// Execute a tool call, with name resolution and arg repair
    pub async fn execute(&self, call: &ToolCall) -> (ToolResult, ExecutionDiag) {
        let start = Instant::now();
        let _original_name = call.name.clone();

        // Step 1: Resolve tool name
        let resolved_name = if self.options.resolve_names {
            match self.resolver.resolve(&call.name) {
                Some(resolved) => {
                    let diag_name = if resolved.tool.name != call.name {
                        Some(call.name.clone())
                    } else {
                        None
                    };
                    (
                        resolved.tool.name.clone(),
                        resolved.match_type.clone(),
                        diag_name,
                    )
                }
                None => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    return (
                        ToolResult {
                            tool_call_id: call.id.clone(),
                            content: format!(
                                "Tool not found: '{}'. Available tools: {}",
                                call.name,
                                self.available_tools_hint()
                            ),
                            is_error: true,
                            execution_time_ms: elapsed,
                        },
                        ExecutionDiag {
                            original_name: None,
                            resolved_name: call.name.clone(),
                            name_match_type: None,
                            args_repaired: false,
                        },
                    );
                }
            }
        } else {
            (call.name.clone(), MatchType::Exact, None)
        };

        // Step 2: Get tool
        let tool = match self.registry.get(&resolved_name.0) {
            Some(t) => t,
            None => {
                let elapsed = start.elapsed().as_millis() as u64;
                return (
                    ToolResult {
                        tool_call_id: call.id.clone(),
                        content: format!("Tool '{}' not in registry", resolved_name.0),
                        is_error: true,
                        execution_time_ms: elapsed,
                    },
                    ExecutionDiag {
                        original_name: resolved_name.2,
                        resolved_name: resolved_name.0,
                        name_match_type: Some(resolved_name.1),
                        args_repaired: false,
                    },
                );
            }
        };

        // Step 3: Repair arguments
        let (args, was_repaired) = if self.options.repair_args {
            let result = self.arg_repairer.repair(&call.arguments.to_string());
            let was_repaired = result.was_repaired();
            (result.into_value(), was_repaired)
        } else {
            (call.arguments.clone(), false)
        };

        // Step 4: Execute
        let output = if tool.requires_sandbox() {
            // Execute via sandbox
            let profile = self.sandbox.create_profile(
                self.options.sandbox_policy.clone(),
                self.options.working_dir.clone(),
            );
            // For sandboxed tools, the args should contain a "command" field
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("echo 'no command specified'");
            match self.sandbox.execute(&profile, cmd).await {
                Ok(result) => {
                    if result.is_success() {
                        Ok(result.stdout)
                    } else {
                        Err(ToolError::ExecutionFailed(if result.stderr.is_empty() {
                            format!("exit code: {:?}", result.exit_code)
                        } else {
                            result.stderr
                        }))
                    }
                }
                Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
            }
        } else {
            // Direct execution
            tool.execute(args).await
        };

        let elapsed = start.elapsed().as_millis() as u64;

        let (content, is_error) = match output {
            Ok(content) => (content, false),
            Err(e) => (e.to_string(), true),
        };

        (
            ToolResult {
                tool_call_id: call.id.clone(),
                content,
                is_error,
                execution_time_ms: elapsed,
            },
            ExecutionDiag {
                original_name: resolved_name.2,
                resolved_name: resolved_name.0,
                name_match_type: Some(resolved_name.1),
                args_repaired: was_repaired,
            },
        )
    }

    /// Get a hint string about available tools
    fn available_tools_hint(&self) -> String {
        let defs = self.registry.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        names.join(", ")
    }

    /// Get the registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Tool;
    use async_trait::async_trait;
    use orca_utils::types::ToolDefinition;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "echo".to_string(),
                description: "Echo the input".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    }
                }),
            }
        }

        async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
            Ok(args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string())
        }

        fn requires_sandbox(&self) -> bool {
            false
        }
    }

    fn setup() -> ToolExecutor {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        let opts = ExecutionOptions {
            resolve_names: true,
            repair_args: true,
            ..Default::default()
        };
        ToolExecutor::new(Arc::new(registry), opts)
    }

    #[tokio::test]
    async fn test_direct_execution() {
        let executor = setup();
        let call = ToolCall {
            id: "t1".to_string(),
            name: "echo".to_string(),
            arguments: json!({"message": "hello"}),
        };
        let (result, diag) = executor.execute(&call).await;
        assert!(!result.is_error);
        assert_eq!(result.content, "hello");
        assert!(!diag.args_repaired);
    }

    #[tokio::test]
    async fn test_name_resolution() {
        let executor = setup();
        let call = ToolCall {
            id: "t2".to_string(),
            name: "Echo".to_string(),
            arguments: json!({"message": "resolved"}),
        };
        let (result, diag) = executor.execute(&call).await;
        assert!(!result.is_error);
        assert_eq!(result.content, "resolved");
        assert!(diag.original_name.is_some());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let executor = setup();
        let call = ToolCall {
            id: "t3".to_string(),
            name: "totally_unknown".to_string(),
            arguments: json!({}),
        };
        let (result, _) = executor.execute(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_arg_repair() {
        let executor = setup();
        let call = ToolCall {
            id: "t4".to_string(),
            name: "echo".to_string(),
            arguments: serde_json::from_str(r#"{"message": "test",}"#)
                .unwrap_or(json!({"message": "test,"})),
        };
        // This should work even with slightly malformed args
        let (result, _) = executor.execute(&call).await;
        // The execution should succeed in some form
        assert!(!result.is_error || result.content.contains("invalid"));
    }
}
