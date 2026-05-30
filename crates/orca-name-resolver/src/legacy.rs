use orca_utils::message::ToolCall;
use serde_json::Value;
use uuid::Uuid;

/// Parses legacy tool call formats that some models emit
pub struct LegacyParser;

impl LegacyParser {
    /// Try to parse tool calls from raw model output
    /// Supports: [TOOL_CALL], XML, arrow syntax, CLI-style
    pub fn parse(content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();

        calls.extend(Self::parse_bracket_format(content));
        calls.extend(Self::parse_xml_format(content));
        calls.extend(Self::parse_json_blocks(content));

        calls
    }

    /// Parse [TOOL_CALL] format:
    /// [TOOL_CALL] tool_name({"arg": "value"})
    fn parse_bracket_format(content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("[TOOL_CALL]") {
                let rest = rest.trim();
                // Find tool name and args
                if let Some(paren_pos) = rest.find('(') {
                    let name = rest[..paren_pos].trim().to_string();
                    let args_str = rest[paren_pos + 1..].trim_end_matches(')').trim();
                    let args = serde_json::from_str::<Value>(args_str)
                        .unwrap_or(serde_json::json!({}));
                    calls.push(ToolCall {
                        id: Uuid::new_v4().to_string(),
                        name,
                        arguments: args,
                    });
                }
            }
        }
        calls
    }

    /// Parse XML format:
    /// <tool name="tool_name"><arg name="key">value</arg></tool>
    fn parse_xml_format(content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        let re = regex::Regex::new(r#"<tool\s+name="([^"]+)">(.*?)</tool>"#).unwrap();

        for cap in re.captures_iter(content) {
            let name = cap[1].to_string();
            let body = &cap[2];
            let mut args = serde_json::Map::new();
            let arg_re = regex::Regex::new(r#"<arg\s+name="([^"]+)">([^<]*)</arg>"#).unwrap();
            for arg_cap in arg_re.captures_iter(body) {
                args.insert(
                    arg_cap[1].to_string(),
                    Value::String(arg_cap[2].to_string()),
                );
            }
            calls.push(ToolCall {
                id: Uuid::new_v4().to_string(),
                name,
                arguments: Value::Object(args),
            });
        }
        calls
    }

    /// Parse JSON code blocks:
    /// ```json
    /// {"name": "tool_name", "arguments": {...}}
    /// ```
    fn parse_json_blocks(content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        let re = regex::Regex::new(r"```json\s*\n(.*?)\n\s*```").unwrap();

        for cap in re.captures_iter(content) {
            if let Ok(value) = serde_json::from_str::<Value>(&cap[1]) {
                if let Some(obj) = value.as_object() {
                    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                        let args = obj.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new()));
                        calls.push(ToolCall {
                            id: Uuid::new_v4().to_string(),
                            name: name.to_string(),
                            arguments: args,
                        });
                    }
                }
            }
        }
        calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_format() {
        let content = r#"Let me read the file.
[TOOL_CALL] read_file({"path": "/tmp/test.txt"})
"#;
        let calls = LegacyParser::parse(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_xml_format() {
        let content = r#"I'll search for it.
<tool name="search_code"><arg name="query">TODO</arg></tool>"#;
        let calls = LegacyParser::parse(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search_code");
    }

    #[test]
    fn test_json_block_format() {
        let content = r#"Here's what I'll do:
```json
{"name": "write_file", "arguments": {"path": "/tmp/out.txt", "content": "hello"}}
```"#;
        let calls = LegacyParser::parse(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "write_file");
    }
}