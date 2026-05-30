use orca_core::agent::Agent;
use std::collections::HashMap;

pub struct CommandHandler {
    _aliases: HashMap<String, String>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("anthropic".to_string(), "claude".to_string());
        Self { _aliases: aliases }
    }

    /// Returns Some(output) if input is a /command, None otherwise
    pub fn try_handle(input: &str, agent: &mut Agent) -> Option<String> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "help" => Some(Self::cmd_help()),
            "tools" => Some(Self::cmd_tools(agent)),
            "model" => Some(Self::cmd_model(agent)),
            "context" => Some(Self::cmd_context(agent)),
            "cost" => Some(Self::cmd_cost(agent)),
            "history" => Some(Self::cmd_history(agent, args)),
            "system" => Some(Self::cmd_system(agent, args)),
            "checkpoint" => Self::cmd_checkpoint(agent),
            "clear" => Some(Self::cmd_clear(agent)),
            "save" => Self::cmd_save(agent, args),
            "load" => Self::cmd_load(agent, args),
            "undo" => Some(Self::cmd_undo(agent)),
            "compact" => Some(Self::cmd_compact(agent)),
            _ => Some(format!(
                "Unknown command: /{}. Type /help for available commands.",
                cmd
            )),
        }
    }

    fn cmd_help() -> String {
        String::from(
"Orca Agent In-Chat Commands

Information Queries:
  /help         Show this help message
  /tools        List available tools
  /model        Show current model configuration
  /context      Show context window usage
  /cost         Show total cost and iteration count
  /history [n]  Show conversation history (default: 10)
  /system       Show current system prompt

Session Management:
  /checkpoint   Create a manual checkpoint
  /clear        Clear conversation history
  /save [file]  Export conversation to JSON file
  /load <file>  Load conversation from JSON file
  /undo         Undo last conversation turn

Conversation Control:
  /compact      Force context compression
  /system <txt> Set system prompt text

Type any message to send it to the model.
"
        )
    }

    fn cmd_tools(agent: &Agent) -> String {
        let defs = agent.tool_executor().registry().definitions();
        if defs.is_empty() {
            return "No tools registered.".to_string();
        }

        let mut output = String::from("Available tools:\n\n");
        for def in &defs {
            output.push_str(&format!("  {} - {}\n", def.name, def.description));
        }
        output.push_str(&format!("\nTotal: {} tool(s)", defs.len()));
        output
    }

    fn cmd_model(agent: &Agent) -> String {
        let info = agent.provider().model_info();
        format!(
            "Model: {}\nMax context: {}\nStreaming: {}\nTools: {}",
            info.display_name,
            info.max_context_tokens,
            if info.supports_streaming { "yes" } else { "no" },
            if info.supports_tools { "yes" } else { "no" }
        )
    }

    fn cmd_context(agent: &Agent) -> String {
        let session = agent.session();
        let usage_ratio = session.context_usage_ratio(128_000);
        let usage_pct = (usage_ratio * 100.0) as u32;

        format!(
            "Context usage: {} / {} tokens ({}%)\nIteration: {}\nLoop guard: {}",
            session.conversation.total_chars() / 4,
            128_000,
            usage_pct,
            session.iteration_count,
            if agent.loop_guard_active() { "active" } else { "inactive" }
        )
    }

    fn cmd_cost(agent: &Agent) -> String {
        format!(
            "Cost so far: ${:.4}\nIterations: {}",
            agent.total_cost(),
            agent.iteration_count()
        )
    }

    fn cmd_history(agent: &Agent, args: &[&str]) -> String {
        let limit: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(10);
        let messages = &agent.session().conversation.messages;

        if messages.is_empty() {
            return "No messages in conversation.".to_string();
        }

        let start = messages.len().saturating_sub(limit);
        let output: Vec<String> = messages
            .iter()
            .skip(start)
            .enumerate()
            .map(|(i, msg)| {
                let role = match msg.role {
                    orca_utils::message::Role::System => "system".to_string(),
                    orca_utils::message::Role::User => "user".to_string(),
                    orca_utils::message::Role::Assistant => "assistant".to_string(),
                    orca_utils::message::Role::Tool => {
                        if let Some(first) = msg.tool_calls.first() {
                            format!("tool({})", first.name)
                        } else {
                            "tool".to_string()
                        }
                    }
                };
                let content: String = msg.content.chars().take(80).collect();
                let suffix = if msg.content.len() > 80 { "..." } else { "" };
                format!("[{}] {}: {}{}", start + i + 1, role, content, suffix)
            })
            .collect();

        output.join("\n")
    }

    fn cmd_system(agent: &mut Agent, args: &[&str]) -> String {
        if args.is_empty() {
            format!(
                "System prompt ({} chars):\n{}",
                agent.system_prompt().len(),
                agent.system_prompt()
            )
        } else {
            let new_prompt = args.join(" ");
            let len = new_prompt.len();
            agent.set_system_prompt(new_prompt);
            format!("System prompt updated ({} chars)", len)
        }
    }

    fn cmd_checkpoint(agent: &Agent) -> Option<String> {
        match agent.create_checkpoint("manual checkpoint") {
            Ok(()) => Some("Checkpoint created.".to_string()),
            Err(e) => Some(format!("Failed to create checkpoint: {}", e)),
        }
    }

    fn cmd_clear(agent: &mut Agent) -> String {
        agent.clear_conversation();
        "Conversation cleared.".to_string()
    }

    fn cmd_save(agent: &Agent, args: &[&str]) -> Option<String> {
        use std::fs;
        use std::path::PathBuf;

        let filename = if args.is_empty() {
            format!(".orca-sessions/{}.json", agent.session().id)
        } else {
            args[0].to_string()
        };

        let path = PathBuf::from(&filename);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let messages = &agent.session().conversation.messages;
        match fs::write(
            &filename,
            serde_json::to_string_pretty(messages).unwrap_or_default(),
        ) {
            Ok(()) => Some(format!("Saved {} messages to {}", messages.len(), filename)),
            Err(e) => Some(format!("Failed to save: {}", e)),
        }
    }

    fn cmd_load(agent: &mut Agent, args: &[&str]) -> Option<String> {
        use std::fs;

        if args.is_empty() {
            return Some("Usage: /load <filename>".to_string());
        }

        let filename = args[0];
        match fs::read_to_string(filename) {
            Ok(content) => {
                match serde_json::from_str::<Vec<orca_utils::message::Message>>(&content) {
                    Ok(messages) => {
                        let count = messages.len();
                        agent.session_mut().conversation.messages = messages;
                        Some(format!("Loaded {} messages from {}", count, filename))
                    }
                    Err(e) => Some(format!("Failed to parse JSON: {}", e)),
                }
            }
            Err(e) => Some(format!("Failed to read file: {}", e)),
        }
    }

    fn cmd_undo(agent: &mut Agent) -> String {
        let messages = &mut agent.session_mut().conversation.messages;
        if messages.is_empty() {
            return "Nothing to undo.".to_string();
        }

        let mut removed = 0;
        // Remove trailing assistant/tool messages first
        while let Some(msg) = messages.last() {
            match msg.role {
                orca_utils::message::Role::User => {
                    messages.pop();
                    removed += 1;
                    break;
                }
                _ => {
                    messages.pop();
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            format!("Undone {} message(s).", removed)
        } else {
            "Nothing to undo.".to_string()
        }
    }

    fn cmd_compact(agent: &mut Agent) -> String {
        let (before, after) = agent.compress_context();
        let diff = before - after;
        if diff == 0 {
            "No messages compressed.".to_string()
        } else {
            format!("Compressed {} message(s)", diff)
        }
    }
}