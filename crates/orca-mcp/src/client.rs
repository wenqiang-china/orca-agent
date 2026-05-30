use crate::types::*;
use anyhow::{Context, Result};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

/// Type alias for the pending requests map
type PendingRequests = Arc<Mutex<Vec<(u64, oneshot::Sender<JsonRpcResponse>)>>>;

/// MCP client that communicates with an MCP server over stdio
pub struct McpClient {
    config: McpServerConfig,
    child: Option<Child>,
    stdin: Option<Arc<Mutex<tokio::process::ChildStdin>>>,
    pending: PendingRequests,
    next_id: AtomicU64,
    server_info: Option<InitializeResult>,
    tools: Vec<McpTool>,
    initialized: bool,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            child: None,
            stdin: None,
            pending: Arc::new(Mutex::new(Vec::new())),
            next_id: AtomicU64::new(1),
            server_info: None,
            tools: Vec::new(),
            initialized: false,
        }
    }

    /// Start the MCP server process and initialize
    pub async fn connect(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .context(format!("failed to start MCP server: {}", self.config.command))?;

        let stdin = child.stdin.take().context("failed to get stdin")?;
        let stdout = child.stdout.take().context("failed to get stdout")?;

        let stdin = Arc::new(Mutex::new(stdin));
        self.stdin = Some(stdin.clone());
        self.child = Some(child);

        // Start background reader for responses
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Try to parse as response (has id)
                        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed)
                            && let Some(id) = resp.id
                        {
                            let mut pending = pending.lock().await;
                            if let Some(pos) = pending.iter().position(|(pid, _)| *pid == id) {
                                let (_, sender) = pending.remove(pos);
                                let _ = sender.send(resp);
                            }
                        }
                        // Notifications are logged but not matched
                    }
                    Err(e) => {
                        tracing::error!("MCP read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Send initialize request (goes through the same send_request mechanism;
        // the background reader will receive the response and forward it via the channel)
        let resp = self
            .send_request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "orca",
                        "version": "0.1.0"
                    }
                })),
            )
            .await?;

        if let Some(err) = resp.error {
            anyhow::bail!("MCP initialize failed: {}", err.message);
        }
        if let Some(result) = resp.result {
            let init: InitializeResult = serde_json::from_value(result)
                .context("failed to parse MCP initialize result")?;
            tracing::info!(
                server = %init.serverInfo.name,
                version = %init.serverInfo.version,
                protocol = %init.protocolVersion,
                "MCP server initialized"
            );
            self.server_info = Some(init);
        }

        // Send initialized notification
        self.send_notification("notifications/initialized", None)
            .await?;

        // List tools
        self.refresh_tools().await?;

        self.initialized = true;
        Ok(())
    }

    /// Refresh the list of available tools
    pub async fn refresh_tools(&mut self) -> Result<()> {
        let resp = self.send_request("tools/list", None).await?;
        if let Some(err) = resp.error {
            anyhow::bail!("MCP tools/list failed: {}", err.message);
        }
        if let Some(result) = resp.result {
            let list: ListToolsResult =
                serde_json::from_value(result).context("failed to parse tools list")?;
            self.tools = list.tools;
            tracing::info!(count = self.tools.len(), "MCP tools refreshed");
        }
        Ok(())
    }

    /// List available tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Convert MCP tools to orca ToolDefinitions
    pub fn tool_definitions(&self) -> Vec<orca_utils::types::ToolDefinition> {
        self.tools
            .iter()
            .map(|t| orca_utils::types::ToolDefinition {
                name: format!("mcp_{}_{}", self.config.name, t.name),
                description: t.description.clone().unwrap_or_default(),
                parameters: t
                    .inputSchema
                    .clone()
                    .unwrap_or(serde_json::json!({"type": "object"})),
            })
            .collect()
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult> {
        // Strip the mcp prefix if present
        let tool_name = name
            .strip_prefix(&format!("mcp_{}_", self.config.name))
            .unwrap_or(name);

        let resp = self
            .send_request(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments
                })),
            )
            .await?;

        if let Some(err) = resp.error {
            anyhow::bail!("MCP tool call failed: {}", err.message);
        }

        let result: McpToolResult = resp
            .result
            .map(|r| {
                serde_json::from_value(r).unwrap_or(McpToolResult {
                    content: vec![],
                    isError: Some(true),
                })
            })
            .unwrap_or(McpToolResult {
                content: vec![],
                isError: Some(false),
            });

        Ok(result)
    }

    /// Whether the client is connected and initialized
    pub fn is_connected(&self) -> bool {
        self.initialized
    }

    /// Get server info
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    /// Send a JSON-RPC request and wait for the response via the background reader
    async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<JsonRpcResponse> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.push((id, tx));

        let mut json = serde_json::to_string(&request)?;
        json.push('\n');

        let stdin = self.stdin.as_ref().context("MCP client not connected")?;
        let mut stdin = stdin.lock().await;
        stdin.write_all(json.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin); // Release the lock before waiting on rx

        rx.await.context("MCP response channel closed")
    }

    /// Send a notification (no response expected)
    async fn send_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let mut json = serde_json::to_string(&notification)?;
        json.push('\n');

        let stdin = self.stdin.as_ref().context("MCP client not connected")?;
        let mut stdin = stdin.lock().await;
        stdin.write_all(json.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        self.initialized = false;
        self.stdin = None;
        if let Some(mut child) = self.child.take() {
            child.kill().await.ok();
        }
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Best effort: try to kill the child process
        if let Some(ref mut child) = self.child {
            // Can't await in Drop, just try to kill
            child.start_kill().ok();
        }
    }
}
