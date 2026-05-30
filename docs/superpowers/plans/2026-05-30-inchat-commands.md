# In-Chat Commands — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand in-chat commands from 3 to 16 by adding CommandHandler module and Agent API extensions.

**Architecture:** New `commands.rs` module handles command dispatching. Main loop calls `CommandHandler::try_handle()`. Agent gains new methods for session management and introspection.

**Tech Stack:** Rust, existing orca-core/orca-cli/orca-utils crates

---

## File Structure

```
orca-agent/
├── crates/
│   ├── orca-core/
│   │   └── src/
│   │       └── agent.rs          # Add 6 new public methods
│   └── orca-cli/
│       ├── src/
│       │   ├── main.rs            # Replace inline command handling
│       │   └── commands.rs        # NEW: CommandHandler with 16 commands
│       └── Cargo.toml
```

---

### Task 1: Add Agent API Extensions (Part 1 — Session Methods)

**Files:**
- Modify: `crates/orca-core/src/agent.rs`

- [ ] **Step 1: Add session_mut() method**

Add this method after `session()` (around line 377):

```rust
/// Get mutable access to the session
pub fn session_mut(&mut self) -> &mut Session {
    &mut self.session
}
```

- [ ] **Step 2: Add clear_conversation() method**

Add this method after `iteration_count()` (around line 387):

```rust
/// Clear all conversation messages while preserving session ID
pub fn clear_conversation(&mut self) {
    self.session.conversation.messages.clear();
    self.record_event(EventKind::MessageSent, serde_json::json!({
        "action": "clear_conversation"
    }));
}
```

- [ ] **Step 3: Add system_prompt() and set_system_prompt() methods**

Add these methods after `clear_conversation()`:

```rust
/// Get the current system prompt
pub fn system_prompt(&self) -> &str {
    &self.system_prompt
}

/// Update the system prompt
pub fn set_system_prompt(&mut self, prompt: String) {
    self.system_prompt = prompt;
    self.record_event(EventKind::MessageSent, serde_json::json!({
        "action": "set_system_prompt",
        "prompt_length": prompt.len()
    }));
}
```

- [ ] **Step 4: Run tests to verify**

Run: `cargo test -p orca-core`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/orca-core/src/agent.rs
git commit -m "feat(agents): add session and system prompt methods"
```

---

### Task 2: Add Agent API Extensions (Part 2 — Tool and Context Methods)

**Files:**
- Modify: `crates/orca-core/src/agent.rs`

- [ ] **Step 1: Add tool_executor() method**

Add this method after `set_system_prompt()`:

```rust
/// Get access to the tool executor for registry inspection
pub fn tool_executor(&self) -> &ToolExecutor {
    &self.tool_executor
}
```

- [ ] **Step 2: Add compress_context() method**

Add this method after `tool_executor()`:

```rust
/// Force context compression, returns (messages_before, messages_after)
pub fn compress_context(&mut self) -> (usize, usize) {
    let before = self.session.conversation.messages.len();
    let result = self.seam_mgr.compress(&mut self.session.conversation.messages);
    let after = self.session.conversation.messages.len();
    if result.messages_compressed > 0 {
        self.record_event(EventKind::MessageSent, serde_json::json!({
            "action": "compress_context",
            "messages_compressed": result.messages_compressed,
            "tokens_saved": result.tokens_saved
        }));
    }
    (before, after)
}
```

- [ ] **Step 3: Add provider() and loop_guard_active() methods**

Add these methods after `compress_context()`:

```rust
/// Get access to the provider
pub fn provider(&self) -> &Arc<dyn ModelProvider> {
    &self.provider
}

/// Check if loop guard is currently blocking
pub fn loop_guard_active(&self) -> bool {
    self.loop_guard.is_blocked()
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p orca-core`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/orca-core/src/agent.rs
git commit -m "feat(agents): add provider, tool_executor, compress_context, and loop_guard_active methods"
```

---

### Task 3: Create commands.rs Module Skeleton

**Files:**
- Create: `crates/orca-cli/src/commands.rs`

- [ ] **Step 1: Write module skeleton with CommandHandler struct**

Write `crates/orca-cli/src/commands.rs`:

```rust
use anyhow::Result;
use orca_core::agent::Agent;
use std::collections::HashMap;

pub struct CommandHandler {
    aliases: HashMap<String, String>,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("anthropic".to_string(), "claude".to_string());
        Self { aliases }
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
            "clear" => Self::cmd_clear(agent),
            "save" => Self::cmd_save(agent, args),
            "load" => Self::cmd_load(agent, args),
            "undo" => Some(Self::cmd_undo(agent)),
            "compact" => Some(Self::cmd_compact(agent)),
            _ => Some(format!("Unknown command: /{}. Type /help for available commands.", cmd)),
        }
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

Add to `crates/orca-cli/src/lib.rs`:

```rust
pub mod commands;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p orca-cli`
Expected: Compiles (with errors for missing methods)

- [ ] **Step 4: Commit**

```bash
git add crates/orca-cli/src/lib.rs crates/orca-cli/src/commands.rs
git commit -m "feat(cli): add CommandHandler module skeleton"
```

---

### Task 4: Implement Information Query Commands

**Files:**
- Modify: `crates/orca-cli/src/commands.rs`

- [ ] **Step 1: Add cmd_help() method**

Add this method inside `impl CommandHandler`:

```rust
fn cmd_help() -> String {
    format!(
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
```

- [ ] **Step 2: Add cmd_tools() method**

Add this method:

```rust
fn cmd_tools(agent: &Agent) -> String {
    let defs = agent.tool_executor().registry().definitions();
    if defs.is_empty() {
        return "No tools registered.".to_string();
    }

    let mut output = String::from("Available tools:\n\n");
    for def in &defs {
        output.push_str(&format!("  {} — {}\n", def.name, def.description));
    }
    output.push_str(&format!("\nTotal: {} tool(s)", defs.len()));
    output
}
```

- [ ] **Step 3: Add cmd_model() method**

Add this method:

```rust
fn cmd_model(agent: &Agent) -> String {
    let info = agent.provider().model_info();
    format!(
"Provider: openai
Model: {}
Max context: {}
Streaming: {}
Tools: {}
",
        info.display_name,
        info.max_context_tokens,
        if info.supports_streaming { "yes" } else { "no" },
        if info.supports_tools { "yes" } else { "no" }
    )
}
```

- [ ] **Step 4: Add cmd_context() method**

Add this method:

```rust
fn cmd_context(agent: &Agent) -> String {
    let session = agent.session();
    let max_tokens = 128_000; // Default, would be better from config
    let usage_ratio = session.context_usage_ratio(max_tokens);
    let usage_pct = (usage_ratio * 100.0) as u32;

    format!(
"Context usage: {} / {} tokens ({}%)
Iteration: {} / {}
Loop guard: {}
",
        (session.conversation.total_chars() / 4),
        max_tokens,
        usage_pct,
        session.iteration_count,
        200, // Default, would be better from config
        if agent.loop_guard_active() { "active" } else { "inactive" }
    )
}
```

- [ ] **Step 5: Add cmd_cost() method**

Add this method:

```rust
fn cmd_cost(agent: &Agent) -> String {
    format!(
"Cost so far: ${:.4}
Iterations: {}",
        agent.total_cost(),
        agent.iteration_count()
    )
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p orca-cli`
Expected: Compiles (may have missing methods like `provider()`)

- [ ] **Step 7: Commit**

```bash
git add crates/orca-cli/src/commands.rs
git commit -m "feat(cli): add info query commands"
```

---

### Task 5: Implement Session Management Commands (checkpoint, clear, undo)

**Files:**
- Modify: `crates/orca-cli/src/commands.rs`

- [ ] **Step 1: Add cmd_checkpoint() method**

Add this method:

```rust
fn cmd_checkpoint(agent: &Agent) -> Option<String> {
    match agent.create_checkpoint("manual checkpoint") {
        Ok(()) => Some("Checkpoint created.".to_string()),
        Err(e) => Some(format!("Failed to create checkpoint: {}", e)),
    }
}
```

- [ ] **Step 2: Add cmd_clear() method**

Add this method (returns None to indicate we need user input):

```rust
fn cmd_clear(agent: &mut Agent) -> Option<String> {
    // Note: This requires user confirmation, handled in main.rs
    agent.clear_conversation();
    Some("Conversation cleared.".to_string())
}
```

- [ ] **Step 3: Add cmd_undo() method**

Add this method:

```rust
fn cmd_undo(agent: &mut Agent) -> String {
    let messages = &mut agent.session_mut().conversation.messages;
    if messages.is_empty() {
        return "Nothing to undo.".to_string();
    }

    // Remove messages until we find and remove a User message
    let mut removed = 0;
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
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p orca-cli`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add crates/orca-cli/src/commands.rs
git commit -m "feat(cli): add session management commands"
```

---

### Task 6: Implement Session Management Commands (save, load)

**Files:**
- Modify: `crates/orca-cli/src/commands.rs`

- [ ] **Step 1: Add cmd_save() method**

Add this method:

```rust
fn cmd_save(agent: &Agent, args: &[&str]) -> Option<String> {
    use std::fs;
    use std::path::PathBuf;

    let filename = if args.is_empty() {
        let session_id = &agent.session().id;
        format!(".orca-sessions/{}.json", session_id)
    } else {
        args[0].to_string()
    };

    let path = PathBuf::from(&filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let messages = &agent.session().conversation.messages;
    match fs::write(&filename, serde_json::to_string_pretty(messages).unwrap_or_default()) {
        Ok(()) => Some(format!("Saved {} messages to {}", messages.len(), filename)),
        Err(e) => Some(format!("Failed to save: {}", e)),
    }
}
```

- [ ] **Step 2: Add cmd_load() method**

Add this method:

```rust
fn cmd_load(agent: &mut Agent, args: &[&str]) -> Option<String> {
    use std::fs;

    if args.is_empty() {
        return Some("Usage: /load <filename>".to_string());
    }

    let filename = args[0];
    match fs::read_to_string(filename) {
        Ok(content) => {
            match serde_json::from_str(&content) as Result<Vec<orca_utils::message::Message>, _> {
                Ok(messages) => {
                    agent.session_mut().conversation.messages = messages;
                    Some(format!("Loaded {} messages from {}", messages.len(), filename))
                }
                Err(e) => Some(format!("Failed to parse JSON: {}", e)),
            }
        }
        Err(e) => Some(format!("Failed to read file: {}", e)),
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p orca-cli`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add crates/orca-cli/src/commands.rs
git commit -m "feat(cli): add save/load session commands"
```

---

### Task 7: Implement Conversation Control Commands

**Files:**
- Modify: `crates/orca-cli/src/commands.rs`

- [ ] **Step 1: Add cmd_history() method**

Add this method:

```rust
fn cmd_history(agent: &Agent, args: &[&str]) -> String {
    let limit: usize = args.get(0).and_then(|s| s.parse().ok()).unwrap_or(10);
    let messages = &agent.session().conversation.messages;

    let output = messages
        .iter()
        .rev()
        .take(limit)
        .enumerate()
        .rev()
        .map(|(i, msg)| {
            let role = match msg.role {
                orca_utils::message::Role::System => "system",
                orca_utils::message::Role::User => "user",
                orca_utils::message::Role::Assistant => "assistant",
                orca_utils::message::Role::Tool => {
                    if !msg.tool_calls.is_empty() {
                        format!("tool({})", msg.tool_calls[0].name)
                    } else {
                        "tool".to_string()
                    }
                }
            };

            let content: String = msg.content.chars().take(80).collect();
            let content = if msg.content.len() > 80 {
                format!("{}...", content)
            } else {
                content
            };

            format!("[{}] {}: {}", messages.len() - limit + i + 1, role, content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    if output.is_empty() {
        "No messages in conversation.".to_string()
    } else {
        output
    }
}
```

- [ ] **Step 2: Add cmd_system() method (handles both show and set)**

Add this method:

```rust
fn cmd_system(agent: &mut Agent, args: &[&str]) -> String {
    if args.is_empty() {
        // Show current system prompt
        format!("System prompt ({} chars):\n{}", agent.system_prompt().len(), agent.system_prompt())
    } else {
        // Set new system prompt
        let new_prompt = args.join(" ");
        agent.set_system_prompt(new_prompt.clone());
        format!("System prompt updated ({} chars)", new_prompt.len())
    }
}
```

- [ ] **Step 3: Add cmd_compact() method**

Add this method:

```rust
fn cmd_compact(agent: &mut Agent) -> String {
    let (before, after) = agent.compress_context();
    let diff = before - after;
    if diff == 0 {
        "No messages compressed.".to_string()
    } else {
        format!("Compressed {} message(s), estimated tokens saved: ~{}",
            diff, diff * 200 // Rough estimate
        )
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p orca-cli`
Expected: Compiles (may have missing `provider()` method)

- [ ] **Step 5: Commit**

```bash
git add crates/orca-cli/src/commands.rs
git commit -m "feat(cli): add conversation control commands"
```

---

### Task 8: Integrate CommandHandler into main.rs (CLI mode)

**Files:**
- Modify: `crates/orca-cli/src/main.rs`

- [ ] **Step 1: Add mod commands import**

Add at top of file with other mods:

```rust
mod commands;
```

- [ ] **Step 2: Replace inline command handling in run_chat()**

Find the `/checkpoint` and `/cost` if-blocks (around lines 228-237) and replace with:

```rust
// Handle in-chat commands
if let Some(output) = commands::CommandHandler::try_handle(input, &mut agent) {
    println!("{}", output);
    continue;
}
```

- [ ] **Step 3: Handle /clear confirmation specially**

After the `try_handle` block, add:

```rust
// Special handling for /clear (requires confirmation)
if input.trim() == "/clear" {
    print!("Clear conversation? This will remove all messages. (y/n): ");
    std::io::stdout().flush().unwrap();
    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm).unwrap();
    if confirm.trim() == "y" || confirm.trim() == "Y" {
        agent.clear_conversation();
        println!("Conversation cleared.");
    } else {
        println!("Cancelled.");
    }
    continue;
}
```

- [ ] **Step 4: Verify CLI compiles and runs**

Run: `cargo build --release -p orca-cli`
Expected: Builds successfully

- [ ] **Step 5: Commit**

```bash
git add crates/orca-cli/src/main.rs
git commit -m "feat(cli): integrate CommandHandler into CLI mode"
```

---

### Task 9: Integrate CommandHandler into TUI mode

**Files:**
- Modify: `crates/orca-cli/src/main.rs`

- [ ] **Step 1: Add command handling in run_tui()**

Find the input handling section (around line 412-424) in the `InputMode::Editing` block. After checking for `quit`, add:

```rust
if input == "quit" || input == "exit" {
    break;
}

// Handle in-chat commands
if let Some(output) = commands::CommandHandler::try_handle(&input, &mut agent) {
    app.push_message(ChatMessage::System(output));
    terminal.draw(|f| render(f, &app))?;
    continue;
}
```

- [ ] **Step 2: Handle /clear in TUI**

Add after the try_handle block:

```rust
// Special handling for /clear in TUI (no interactive prompt)
if input.trim() == "/clear" {
    agent.clear_conversation();
    app.push_message(ChatMessage::System("Conversation cleared.".to_string()));
    terminal.draw(|f| render(f, &app))?;
    continue;
}
```

- [ ] **Step 3: Verify TUI compiles**

Run: `cargo build --release -p orca-cli`
Expected: Builds successfully

- [ ] **Step 4: Commit**

```bash
git add crates/orca-cli/src/main.rs
git commit -m "feat(cli): integrate CommandHandler into TUI mode"
```

---

### Task 10: Update Documentation

**Files:**
- Modify: `docs/cli.html`

- [ ] **Step 1: Update in-chat commands section**

Replace the In-Chat Commands section table with:

```html
<h2>In-Chat Commands</h2>
<p>During an interactive session, these commands provide meta-operations:</p>

<table class="doc-table">
<tr><th>Command</th><th>Description</th><th>Args</th></tr>
<tr><td>/help</td><td>List all commands</td><td>—</td></tr>
<tr><td>/tools</td><td>List available tools</td><td>—</td></tr>
<tr><td>/model</td><td>Show current model info</td><td>—</td></tr>
<tr><td>/context</td><td>Show context usage</td><td>—</td></tr>
<tr><td>/cost</td><td>Show cost & iterations</td><td>—</td></tr>
<tr><td>/history [n]</td><td>Show conversation history</td><td>n (default: 10)</td></tr>
<tr><td>/system</td><td>Show system prompt</td><td>—</td></tr>
<tr><td>/system &lt;text&gt;</td><td>Set system prompt</td><td>new prompt text</td></tr>
<tr><td>/checkpoint</td><td>Create manual checkpoint</td><td>—</td></tr>
<tr><td>/clear</td><td>Clear conversation</td><td>—</td></tr>
<tr><td>/save [file]</td><td>Export conversation to JSON</td><td>filename</td></tr>
<tr><td>/load &lt;file&gt;</td><td>Load conversation from JSON</td><td>filename</td></tr>
<tr><td>/undo</td><td>Undo last turn</td><td>—</td></tr>
<tr><td>/compact</td><td>Force context compression</td><td>—</td></tr>
<tr><td>quit / exit / q</td><td>Exit session</td><td>—</td></tr>
</table>
```

- [ ] **Step 2: Verify documentation renders**

Run: `open docs/cli.html`
Expected: CLI page with updated commands table

- [ ] **Step 3: Commit**

```bash
git add docs/cli.html
git commit -m "docs(cli): update in-chat commands reference"
```

---

### Task 11: Final Testing and Verification

**Files:**
- Test: Manual verification of all commands

- [ ] **Step 1: Test info commands**

Run orca and test:
- `/help` — shows full command list
- `/tools` — shows registered tools
- `/model` — shows model info
- `/context` — shows context usage
- `/cost` — shows cost
- `/history` — shows recent messages
- `/history 5` — shows last 5 messages
- `/system` — shows current prompt

Expected: All commands output correctly

- [ ] **Step 2: Test session management commands**

Test:
- `/checkpoint` — creates checkpoint
- `/save test.json` — saves conversation
- `/load test.json` — loads conversation
- `/undo` — undoes last turn
- `/clear` — requires y/n confirmation

Expected: All work correctly

- [ ] **Step 3: Test control commands**

Test:
- `/compact` — compresses context
- `/system test prompt` — sets new prompt
- `/system` — shows updated prompt

Expected: All work correctly

- [ ] **Step 4: Test error cases**

Test:
- `/unknown` — shows unknown command message
- `/load nonexistent.json` — shows error

Expected: Error messages are clear

- [ ] **Step 5: Final commit**

```bash
git add .
git commit -m "chore: final implementation verification and cleanup"
```