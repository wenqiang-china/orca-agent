use crate::registry::{Tool, ToolError};
use async_trait::async_trait;
use orca_utils::types::ToolDefinition;
use serde_json::{json, Value};

/// Fetch content from a URL
pub struct WebFetchTool {
    client: reqwest::Client,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("orca-agent/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch content from a URL. Returns the response body as text. Supports HTML, JSON, plain text.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "Maximum characters to return (default: 10000)",
                        "default": 10000
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let url = args.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'url'".to_string()))?;

        let max_chars = args.get("max_chars")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10_000);

        // Validate URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidArgs("only http/https URLs are supported".to_string()));
        }

        let resp = self.client.get(url).send().await
            .map_err(|e| ToolError::ExecutionFailed(format!("request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(ToolError::ExecutionFailed(format!("HTTP {}: {}", status, status.canonical_reason().unwrap_or("error"))));
        }

        let body = resp.text().await
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to read body: {}", e)))?;

        if body.len() <= max_chars {
            Ok(body)
        } else {
            Ok(format!("{}...\n\n[truncated, {} total chars]", &body[..max_chars], body.len()))
        }
    }

    fn requires_sandbox(&self) -> bool {
        true
    }
}

/// Search the web (using a search engine API - stub for now)
pub struct WebSearchTool {
    client: reqwest::Client,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("orca-agent/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information. Returns relevant results with titles, URLs, and snippets.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query'".to_string()))?;

        // Use DuckDuckGo HTML search as a simple fallback
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));

        let resp = self.client.get(&url).send().await
            .map_err(|e| ToolError::ExecutionFailed(format!("search request failed: {}", e)))?;

        let body = resp.text().await
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to read search results: {}", e)))?;

        // Simple HTML parsing to extract results
        let mut results = Vec::new();
        let mut lines = body.lines();
        while let Some(line) = lines.next() {
            if line.contains("result__a") {
                // Extract title and URL
                if let Some(title_start) = line.find("result__a") {
                    let after = &line[title_start..];
                    if let Some(href_start) = after.find("href=\"") {
                        let href = &after[href_start + 6..];
                        if let Some(href_end) = href.find('"') {
                            let url = &href[..href_end];
                            // Get text content
                            if let Some(text_start) = after.find('>') {
                                let text = &after[text_start + 1..];
                                if let Some(text_end) = text.find('<') {
                                    let title = &text[..text_end];
                                    results.push(format!("- {} ({})", title.trim(), url));
                                }
                            }
                        }
                    }
                }
            }
            if results.len() >= args.get("num_results").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5) {
                break;
            }
        }

        if results.is_empty() {
            Ok("No search results found. Try rephrasing your query.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn requires_sandbox(&self) -> bool {
        true
    }
}