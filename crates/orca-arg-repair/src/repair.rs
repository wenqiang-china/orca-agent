use serde_json::Value;

/// Maximum input size for repair (1 MiB)
const MAX_INPUT_SIZE: usize = 1024 * 1024;
/// Maximum bracket balancing iterations
const MAX_BRACKET_ITERATIONS: usize = 50;

/// Result of argument repair
#[derive(Debug, Clone)]
pub enum RepairResult {
    /// Input was valid JSON, no repair needed
    Valid(Value),
    /// Input was repaired successfully
    Repaired(Value),
    /// Could not repair, returned fallback empty object
    Fallback(Value),
}

impl RepairResult {
    pub fn into_value(self) -> Value {
        match self {
            Self::Valid(v) | Self::Repaired(v) | Self::Fallback(v) => v,
        }
    }

    pub fn was_repaired(&self) -> bool {
        matches!(self, Self::Repaired(_))
    }
}

/// 5-stage JSON argument repairer
pub struct ArgRepairer {
    max_input_size: usize,
    max_bracket_iterations: usize,
}

impl Default for ArgRepairer {
    fn default() -> Self {
        Self {
            max_input_size: MAX_INPUT_SIZE,
            max_bracket_iterations: MAX_BRACKET_ITERATIONS,
        }
    }
}

impl ArgRepairer {
    pub fn new(max_input_size: usize, max_bracket_iterations: usize) -> Self {
        Self {
            max_input_size,
            max_bracket_iterations,
        }
    }

    /// Run the 5-stage repair pipeline on raw argument string
    pub fn repair(&self, input: &str) -> RepairResult {
        // Size guard
        if input.len() > self.max_input_size {
            tracing::warn!(
                "input exceeds max size ({} > {})",
                input.len(),
                self.max_input_size
            );
            return RepairResult::Fallback(Value::Object(serde_json::Map::new()));
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            return RepairResult::Valid(Value::Object(serde_json::Map::new()));
        }

        // Stage 1: Try direct parse
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            // Check for double-encoded JSON (string value containing JSON)
            if let Some(inner) = self.unwrap_double_encoded_value(&value) {
                return RepairResult::Repaired(inner);
            }
            return RepairResult::Valid(value);
        }

        let mut s = trimmed.to_string();

        // Stage 2: Remove control characters (except \n, \r, \t which are valid in JSON strings)
        s = self.remove_control_chars(&s);

        // Stage 3: Remove trailing commas
        s = self.remove_trailing_commas(&s);

        // Stage 4: Balance brackets
        s = self.balance_brackets(&s);

        // Stage 5: Strip extra closures at end
        s = self.strip_extra_closures(&s);

        // Try parse after repairs
        if let Ok(value) = serde_json::from_str::<Value>(&s) {
            return RepairResult::Repaired(value);
        }

        // Stage 6: Double-encoded JSON detection (JSON within JSON)
        if let Some(unwrapped) = self.unwrap_double_encoded(&s) {
            return RepairResult::Repaired(unwrapped);
        }

        // Try one more time after double-decode attempt
        if let Ok(value) = serde_json::from_str::<Value>(&s) {
            return RepairResult::Repaired(value);
        }

        tracing::warn!("could not repair JSON arguments, using fallback");
        RepairResult::Fallback(Value::Object(serde_json::Map::new()))
    }

    /// Stage 2: Remove non-printable control characters
    fn remove_control_chars(&self, s: &str) -> String {
        s.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect()
    }

    // Stage 3: Remove trailing commas before } or ]
    fn remove_trailing_commas(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();

        for i in 0..len {
            if chars[i] == ',' {
                // Look ahead for closing bracket (skipping whitespace)
                let has_trailing_close = chars[i + 1..]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| *c == '}' || *c == ']');

                if has_trailing_close {
                    continue; // Skip the trailing comma
                }
            }
            result.push(chars[i]);
        }
        result
    }

    /// Stage 4: Balance opening and closing brackets
    fn balance_brackets(&self, s: &str) -> String {
        let mut result = s.to_string();

        for _ in 0..self.max_bracket_iterations {
            let open_braces = result.matches('{').count();
            let close_braces = result.matches('}').count();
            let open_brackets = result.matches('[').count();
            let close_brackets = result.matches(']').count();

            let needs_close_brace = open_braces as i32 - close_braces as i32;
            let needs_close_bracket = open_brackets as i32 - close_brackets as i32;

            if needs_close_brace <= 0 && needs_close_bracket <= 0 {
                break;
            }

            // Close brackets before braces (inner before outer)
            if needs_close_bracket > 0 {
                result.push(']');
            } else if needs_close_brace > 0 {
                result.push('}');
            }
        }

        result
    }

    // Stage 5: Strip extra closing brackets/braces at the end
    fn strip_extra_closures(&self, s: &str) -> String {
        let mut result = s.to_string();

        // Remove trailing }, ], or combinations
        while result.ends_with('}') || result.ends_with(']') {
            let open_braces = result.matches('{').count();
            let close_braces = result.matches('}').count();
            let open_brackets = result.matches('[').count();
            let close_brackets = result.matches(']').count();

            if close_braces > open_braces
                && let Some(pos) = result.rfind('}')
            {
                result.remove(pos);
                continue;
            }
            if close_brackets > open_brackets
                && let Some(pos) = result.rfind(']')
            {
                result.remove(pos);
                continue;
            }
            break;
        }

        result
    }

    /// Stage 6: Detect and unwrap double-encoded JSON
    /// e.g., "{\"key\": \"value\"}" → {"key": "value"}
    fn unwrap_double_encoded(&self, s: &str) -> Option<Value> {
        // Check if the string looks like a JSON string containing JSON
        let trimmed = s.trim();
        if trimmed.starts_with("\"{") && trimmed.ends_with("}\"") {
            // Try to parse as a JSON string first
            if let Ok(inner_str) = serde_json::from_str::<String>(trimmed)
                && let Ok(value) = serde_json::from_str::<Value>(&inner_str)
            {
                return Some(value);
            }
        }

        // Also handle: the value is a string that contains valid JSON
        if let Ok(value) = serde_json::from_str::<Value>(trimmed)
            && let Some(s) = value.as_str()
            && let Ok(inner) = serde_json::from_str::<Value>(s)
        {
            return Some(inner);
        }

        None
    }

    /// Check if a parsed JSON value is actually a double-encoded string
    /// e.g., Value::String("{\"key\": \"value\"}") → Value::Object({"key": "value"})
    fn unwrap_double_encoded_value(&self, value: &Value) -> Option<Value> {
        if let Some(s) = value.as_str() {
            // Try to parse the string content as JSON
            if let Ok(inner) = serde_json::from_str::<Value>(s) {
                return Some(inner);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_json() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair(r#"{"path": "/tmp/test.txt"}"#);
        assert!(!result.was_repaired());
        let v = result.into_value();
        assert_eq!(v["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_trailing_comma() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair(r#"{"path": "/tmp/test.txt",}"#);
        assert!(result.was_repaired());
        let v = result.into_value();
        assert_eq!(v["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_missing_closing_brace() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair(r#"{"path": "/tmp/test.txt""#);
        let v = result.into_value();
        assert_eq!(v["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_extra_closing_braces() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair(r#"{"path": "/tmp/test.txt"}}"#);
        let v = result.into_value();
        assert_eq!(v["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_empty_input() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair("");
        assert!(!result.was_repaired());
    }

    #[test]
    fn test_double_encoded_json() {
        let repairer = ArgRepairer::default();
        let double = serde_json::to_string(&r#"{"key": "value"}"#).unwrap();
        let result = repairer.repair(&double);
        let v = result.into_value();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn test_control_chars() {
        let repairer = ArgRepairer::default();
        let input = "{\x01\"path\": \x02\"/tmp/test.txt\"\x03}";
        let result = repairer.repair(input);
        let v = result.into_value();
        assert_eq!(v["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_nested_trailing_commas() {
        let repairer = ArgRepairer::default();
        let result = repairer.repair(r#"{"a": [1, 2, 3,], "b": {"c": 1,}}"#);
        let v = result.into_value();
        assert_eq!(v["a"][0], 1);
        assert_eq!(v["b"]["c"], 1);
    }
}
