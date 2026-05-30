use crate::registry::{Tool, ToolError};
use async_trait::async_trait;
use orca_utils::types::ToolDefinition;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Read a file from disk
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path' argument".to_string()))?;

        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to read {}: {}", path, e)))
    }

    fn requires_sandbox(&self) -> bool {
        false
    }
}

/// Write content to a file
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file, creating parent directories if needed"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to write to"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path' argument".to_string()))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'content' argument".to_string()))?;

        // Create parent directories
        if let Some(parent) = PathBuf::from(path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("failed to create dirs: {}", e)))?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to write {}: {}", path, e)))?;

        Ok(format!("Wrote {} bytes to {}", content.len(), path))
    }

    fn requires_sandbox(&self) -> bool {
        false
    }
}

/// List files matching a glob pattern
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., '**/*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory (default: current directory)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pattern' argument".to_string()))?;

        let base = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let full_pattern = if base == "." {
            pattern.to_string()
        } else {
            format!("{}/{}", base.trim_end_matches('/'), pattern)
        };

        let paths = glob::glob(&full_pattern)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid glob pattern: {}", e)))?;

        let mut results = Vec::new();
        for entry in paths {
            match entry {
                Ok(path) => results.push(path.display().to_string()),
                Err(e) => tracing::warn!("glob error: {}", e),
            }
        }

        if results.is_empty() {
            Ok("No files found matching pattern.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn requires_sandbox(&self) -> bool {
        false
    }
}

/// Search for text patterns in files
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for a pattern in files".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (regex supported)"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search in"
                    },
                    "glob": {
                        "type": "string",
                        "description": "File glob filter (e.g., '*.rs')"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pattern'".to_string()))?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let re = regex::Regex::new(pattern)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid regex: {}", e)))?;

        let mut results = Vec::new();
        let walker = walkdir::WalkDir::new(path).max_depth(10);

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }

            // Check glob filter
            if let Some(glob_filter) = args.get("glob").and_then(|v| v.as_str()) {
                let file_name = entry.file_name().to_string_lossy();
                if let Ok(pattern) = glob::Pattern::new(glob_filter)
                    && !pattern.matches(&file_name)
                {
                    continue;
                }
            }

            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        results.push(format!(
                            "{}:{}: {}",
                            entry.path().display(),
                            line_num + 1,
                            line.trim()
                        ));
                    }
                }
            }

            if results.len() > 500 {
                results.push("... (results truncated)".to_string());
                break;
            }
        }

        if results.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn requires_sandbox(&self) -> bool {
        false
    }
}

/// Register all built-in tools
pub fn register_builtins(registry: &mut crate::registry::ToolRegistry) {
    use std::sync::Arc;
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(GlobTool));
    registry.register(Arc::new(GrepTool));
}

/// Register all built-in tools (including shell, git, web)
pub fn register_all_builtins(registry: &mut crate::registry::ToolRegistry, working_dir: std::path::PathBuf) {
    use std::sync::Arc;
    // File tools
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(GlobTool));
    registry.register(Arc::new(GrepTool));
    // Shell tool
    registry.register(Arc::new(crate::shell::ShellTool));
    // Git tool
    registry.register(Arc::new(crate::git::GitTool::new(working_dir)));
    // Web tools
    registry.register(Arc::new(crate::web::WebFetchTool::default()));
    registry.register(Arc::new(crate::web::WebSearchTool::default()));
}
