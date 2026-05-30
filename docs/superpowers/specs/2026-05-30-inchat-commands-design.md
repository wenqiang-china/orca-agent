# In-Chat Commands Design

## Overview

Expand Orca Agent's in-chat command system from 3 commands to 16, covering session management, information queries, and conversation control.

## Architecture

Extract command handling into a dedicated `commands.rs` module inside `orca-cli`. The main loop calls `CommandHandler::try_handle(input, &mut agent)` — returns `Some(output)` if a command was matched, `None` if the input should be sent to the model.

### New file: `crates/orca-cli/src/commands.rs`

```rust
pub struct CommandHandler { ... }

impl CommandHandler {
    /// Returns Some(output) if input is a /command, None otherwise
    pub fn try_handle(input: &str, agent: &mut Agent) -> Option<String>;
}
```

### Changes to `crates/orca-cli/src/main.rs`

Replace the inline `/checkpoint` and `/cost` if-blocks (lines 228-237) with a single call:

```rust
if let Some(output) = CommandHandler::try_handle(input, &mut agent) {
    println!("{}", output);
    continue;
}
```

The `quit`/`exit`/`q` check stays in main.rs since it breaks the loop (it's a control flow concern, not a command).

---

## Command Reference

### Information Queries

**`/help`** — List all commands with one-line descriptions. Grouped by category (Query / Session / Control).

**`/tools`** — Iterate `agent.tool_executor().registry()` and print each tool's name + description. Output as a table.

**`/model`** — Print current config values:
```
Provider: openai
Model: gpt-4o
Base URL: (default)
```

**`/context`** — Print context window stats from `agent.session()`:
```
Context usage: 12,345 / 128,000 tokens (9.6%)
Compress threshold: 75%
Iteration: 14 / 200
Loop guard: active
```

**`/cost`** — (existing) Print total cost and iteration count.

**`/history`** — Print a summary of recent conversation turns. Show last 10 messages with role + truncated content (first 80 chars). Format:
```
[1] user: Refactor the auth module to use JWT
[2] assistant: I'll help you refactor the auth module...
[3] tool(read_file): {"path":"src/auth.rs"}
...
```

**`/system`** — (no args) Print current system prompt.

### Session Management

**`/checkpoint`** — (existing) Create manual checkpoint with description "manual checkpoint".

**`/clear`** — Clear conversation history. Before clearing, prompt confirmation:
```
Clear conversation? This will remove all messages. (y/n)
```
On confirmation: `agent.clear_conversation()`. Agent needs a new method that empties `session.conversation.messages` while preserving session ID, checkpoint manager, etc.

**`/save [filename]`** — Export conversation to JSON file. Default filename: `.orca-sessions/<session-id>.json`. If filename arg provided, use that path. Serialize `session.conversation.messages` to JSON.

**`/load <filename>`** — Load conversation from JSON file. Replace current `session.conversation.messages`. Parse JSON back into `Vec<Message>`.

**`/undo`** — Remove the last user message and its corresponding assistant response. Walk `session.conversation.messages` backwards, remove messages until the conversation is restored to before the last user turn.

### Conversation Control

**`/compact`** — Force context compression by calling `agent.seam_mgr().compress(&mut agent.session_mut().conversation.messages)`. Print stats:
```
Compressed 4 messages, saved ~2,100 tokens
```

**`/system <text>`** — Update system prompt: `agent.set_system_prompt(text)`. Agent needs a new method. Print confirmation:
```
System prompt updated (142 chars)
```

---

## Agent API Extensions

The `Agent` struct in `orca-core/src/agent.rs` needs these new public methods:

```rust
/// Clear all conversation messages
pub fn clear_conversation(&mut self) {
    self.session.conversation.messages.clear();
}

/// Update the system prompt
pub fn set_system_prompt(&mut self, prompt: String) {
    self.system_prompt = prompt;
}

/// Get the current system prompt
pub fn system_prompt(&self) -> &str {
    &self.system_prompt
}

/// Get access to session (mutable)
pub fn session_mut(&mut self) -> &mut Session {
    &mut self.session
}

/// Get access to tool executor for registry inspection
pub fn tool_executor(&self) -> &ToolExecutor {
    &self.tool_executor
}

/// Force context compression
pub fn compress_context(&mut self) -> (usize, usize) {
    let before = self.session.conversation.messages.len();
    self.seam_mgr.compress(&mut self.session.conversation.messages);
    let after = self.session.conversation.messages.len();
    (before, after)
}
```

---

## Implementation Notes

- Commands are case-sensitive (`/help` works, `/HELP` does not)
- Unknown `/xxx` commands print: "Unknown command: /xxx. Type /help for available commands."
- `/save` and `/load` create `.orca-sessions/` directory if it doesn't exist
- `/clear` requires confirmation to prevent accidental data loss
- `/undo` silently succeeds if there are no messages to undo
- `/compact` reports 0 if no messages were compressed
- `/history` defaults to last 10 messages, configurable via `/history 20`
