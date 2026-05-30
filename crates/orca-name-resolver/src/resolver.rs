use orca_utils::types::ToolDefinition;
use orca_utils::message::ToolCall;

/// Resolution match with confidence level
#[derive(Debug, Clone)]
pub struct ResolvedTool {
    pub tool: ToolDefinition,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchType {
    Exact,
    CaseInsensitive,
    HyphenNormalized,
    CamelToSnake,
    PrefixFuzzy,
}

/// 5-step tool name resolver
pub struct ToolNameResolver {
    tools: Vec<ToolDefinition>,
    /// Pre-computed lowercase names for fast lookup
    lowercase_names: Vec<(String, usize)>,
    /// Pre-computed hyphen-normalized names
    hyphen_names: Vec<(String, usize)>,
    /// Pre-computed snake_case names
    snake_names: Vec<(String, usize)>,
}

impl ToolNameResolver {
    pub fn new(tools: Vec<ToolDefinition>) -> Self {
        let lowercase_names: Vec<(String, usize)> = tools
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.to_lowercase(), i))
            .collect();

        let hyphen_names: Vec<(String, usize)> = tools
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.replace('-', "_"), i))
            .collect();

        let snake_names: Vec<(String, usize)> = tools
            .iter()
            .enumerate()
            .map(|(i, t)| (camel_to_snake_case(&t.name), i))
            .collect();

        Self {
            tools,
            lowercase_names,
            hyphen_names,
            snake_names,
        }
    }

    /// Resolve a tool name through the 5-step ladder
    pub fn resolve(&self, name: &str) -> Option<ResolvedTool> {
        // Step 1: Exact match
        if let Some(idx) = self.tools.iter().position(|t| t.name == name) {
            return Some(ResolvedTool {
                tool: self.tools[idx].clone(),
                match_type: MatchType::Exact,
            });
        }

        // Step 2: Case-insensitive
        let lower = name.to_lowercase();
        if let Some(idx) = self.lowercase_names.iter().find(|(n, _)| *n == lower).map(|(_, i)| *i) {
            return Some(ResolvedTool {
                tool: self.tools[idx].clone(),
                match_type: MatchType::CaseInsensitive,
            });
        }

        // Step 3: Hyphen normalization (read-file → read_file)
        let normalized = name.replace('-', "_");
        if let Some(idx) = self.hyphen_names.iter().find(|(n, _)| *n == normalized).map(|(_, i)| *i) {
            return Some(ResolvedTool {
                tool: self.tools[idx].clone(),
                match_type: MatchType::HyphenNormalized,
            });
        }

        // Step 4: CamelCase → snake_case
        let snake = camel_to_snake_case(name);
        if let Some(idx) = self.snake_names.iter().find(|(n, _)| *n == snake).map(|(_, i)| *i) {
            return Some(ResolvedTool {
                tool: self.tools[idx].clone(),
                match_type: MatchType::CamelToSnake,
            });
        }

        // Step 5: Prefix fuzzy match (longest prefix match)
        let lower = name.to_lowercase();
        let mut best_match: Option<(usize, usize)> = None; // (tool_idx, prefix_len)
        for (tool_name, idx) in &self.lowercase_names {
            if tool_name.starts_with(&lower) || lower.starts_with(tool_name) {
                let match_len = tool_name.len().min(lower.len());
                if best_match.map_or(true, |(_, best_len)| match_len > best_len) {
                    best_match = Some((*idx, match_len));
                }
            }
        }

        if let Some((idx, _)) = best_match {
            return Some(ResolvedTool {
                tool: self.tools[idx].clone(),
                match_type: MatchType::PrefixFuzzy,
            });
        }

        None
    }

    /// Resolve a tool call, repairing the name if needed
    pub fn resolve_tool_call(&self, call: &ToolCall) -> Option<(ToolCall, MatchType)> {
        self.resolve(&call.name).map(|resolved| {
            (
                ToolCall {
                    id: call.id.clone(),
                    name: resolved.tool.name.clone(),
                    arguments: call.arguments.clone(),
                },
                resolved.match_type,
            )
        })
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// Convert CamelCase to snake_case
fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.extend(ch.to_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: String::new(),
            parameters: serde_json::json!({}),
        }
    }

    fn make_resolver() -> ToolNameResolver {
        ToolNameResolver::new(vec![
            make_tool("read_file"),
            make_tool("write_file"),
            make_tool("execute_shell"),
            make_tool("search_code"),
            make_tool("edit_file"),
        ])
    }

    #[test]
    fn test_exact_match() {
        let resolver = make_resolver();
        let result = resolver.resolve("read_file").unwrap();
        assert_eq!(result.match_type, MatchType::Exact);
    }

    #[test]
    fn test_case_insensitive() {
        let resolver = make_resolver();
        let result = resolver.resolve("Read_File").unwrap();
        assert_eq!(result.match_type, MatchType::CaseInsensitive);
    }

    #[test]
    fn test_hyphen_normalization() {
        let resolver = make_resolver();
        let result = resolver.resolve("read-file").unwrap();
        assert_eq!(result.match_type, MatchType::HyphenNormalized);
    }

    #[test]
    fn test_camel_to_snake() {
        let resolver = make_resolver();
        let result = resolver.resolve("readFile").unwrap();
        assert_eq!(result.match_type, MatchType::CamelToSnake);
    }

    #[test]
    fn test_prefix_fuzzy() {
        let resolver = make_resolver();
        let result = resolver.resolve("read").unwrap();
        assert_eq!(result.match_type, MatchType::PrefixFuzzy);
        assert_eq!(result.tool.name, "read_file");
    }

    #[test]
    fn test_no_match() {
        let resolver = make_resolver();
        assert!(resolver.resolve("totally_unknown_tool").is_none());
    }

    #[test]
    fn test_camel_to_snake_conversion() {
        assert_eq!(camel_to_snake_case("readFile"), "read_file");
        assert_eq!(camel_to_snake_case("executeShellCommand"), "execute_shell_command");
        assert_eq!(camel_to_snake_case("simple"), "simple");
        assert_eq!(camel_to_snake_case("ABC"), "a_b_c");
    }
}