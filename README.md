# Orca Agent

A high-performance coding agent built in Rust, featuring multi-provider support, OS-level sandboxing, semantic capacity control, and checkpoint-based session recovery.

## Features

- **Multi-Provider Support** - DeepSeek, Anthropic (Claude), OpenAI, and any OpenAI-compatible third-party provider
- **OS-Level Sandboxing** - Command timeout, output truncation, dangerous command blocking
- **Semantic Capacity Control** - CanonicalState tracking with 3-checkpoint evaluation
- **Context Management** - Layered archiving with anchor preservation across compression
- **Loop Detection** - Self-correcting loop guard with mutation-aware window clearing
- **Tool Fault Tolerance** - 5-step fuzzy name resolution + 6-stage JSON argument repair
- **Checkpoint System** - Save/restore session state with automatic checkpoint management
- **TUI Mode** - Rich terminal UI with ratatui
- **MCP Client** - Model Context Protocol support for external tool servers

## Quick Start

### Build

```bash
cargo build --release
```

### Run

```bash
# Interactive chat
./target/release/orca chat

# TUI mode
./target/release/orca chat --tui

# With specific provider/model
./target/release/orca chat --provider deepseek --model deepseek-chat

# List available models
./target/release/orca models

# Show configuration
./target/release/orca config show
```

### Set API Keys

```bash
export DEEPSEEK_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"
export OPENAI_API_KEY="your-key"
```

## Provider Configuration

### Built-in Providers

| Provider | CLI name | Models |
|----------|----------|--------|
| DeepSeek | `deepseek` | deepseek-chat, deepseek-reasoner |
| Anthropic | `anthropic` / `claude` | claude-sonnet-4-20250514, claude-opus-4-20250514 |
| OpenAI | `openai` | gpt-4o, gpt-4o-mini, gpt-4.1, o1, o3, o3-mini |

### OpenAI-Compatible Third-Party Providers

Any provider that implements the OpenAI API format is supported via the `custom` or `openai-compatible` provider type.

**CLI one-off usage:**

```bash
# Ollama (local)
cargo run -- chat --provider openai --model llama3 --base-url http://localhost:11434/v1

# Groq
cargo run -- chat --provider openai --model llama3-70b-8192 --base-url https://api.groq.com/openai

# Together AI
cargo run -- chat --provider openai --model meta-llama/Llama-3-70b-chat-hf --base-url https://api.together.xyz/v1

# OpenRouter
cargo run -- chat --provider openai --model anthropic/claude-3.5-sonnet --base-url https://openrouter.ai/api/v1

# Azure OpenAI
cargo run -- chat --provider openai --model gpt-4o --base-url https://your-resource.openai.azure.com/openai
```

**Config file** (`~/.config/orca/config.toml`):

```toml
provider = "custom"
model = "your-model-name"

[providers.custom]
api_key = "$YOUR_API_KEY"
base_url = "https://your-api-endpoint.com"
```

Supported third-party providers:

| Provider | base_url |
|----------|----------|
| Ollama | `http://localhost:11434/v1` |
| Groq | `https://api.groq.com/openai` |
| Together AI | `https://api.together.xyz/v1` |
| Cloudflare Workers AI | `https://api.cloudflare.com/client/v4/accounts/{id}/ai` |
| vLLM | `http://localhost:8000/v1` |
| LiteLLM | `http://localhost:4000` |
| Azure OpenAI | `https://{resource}.openai.azure.com/openai` |
| OpenRouter | `https://openrouter.ai/api/v1` |

## Configuration

Configuration file location: `~/.config/orca/config.toml`

```toml
provider = "deepseek"
model = "deepseek-chat"

[providers.deepseek]
api_key = "$DEEPSEEK_API_KEY"
# base_url = "https://api.deepseek.com"  # optional custom endpoint

[sandbox]
enabled = true
exec_timeout_secs = 120
max_exec_timeout_secs = 600
network_policy = "denied"  # denied | restricted | full

[budget]
max_session_budget_usd = 10.0
max_iterations = 200
rate_limit_per_minute = 60

[context]
max_context_tokens = 128000
compress_threshold = 0.75
max_checkpoints = 10
use_flash_summary = true

log_level = "info"
```

## CLI Commands

```bash
# Chat
orca chat                          # Start interactive session
orca chat --tui                    # Start TUI mode
orca chat -p "explain this code"   # One-shot query
orca chat --provider openai --model gpt-4o --base-url http://localhost:11434/v1

# Configuration
orca config show                   # Display current config
orca config set key value          # Update config value
orca config reset                  # Reset to defaults

# Models
orca models                        # List available models

# Checkpoints
orca resume <checkpoint-id>        # Resume from checkpoint
```

In-chat commands:
- `/checkpoint` - Create manual checkpoint
- `/cost` - Show cost and iteration count
- `quit` / `exit` / `q` - Exit session

## Architecture

```
orca (CLI + TUI)
├── orca-core          # Agent loop, session management
│   ├── orca-providers # Unified provider trait + routing
│   │   ├── orca-deepseek   # DeepSeek provider
│   │   ├── orca-anthropic  # Anthropic/Claude provider
│   │   └── orca-openai     # OpenAI provider
│   ├── orca-tools     # Tool executor + registry
│   │   ├── orca-sandbox      # OS-level sandboxing
│   │   ├── orca-name-resolver # Fuzzy tool name matching
│   │   └── orca-arg-repair   # JSON argument repair
│   ├── orca-capacity  # Semantic state + checkpoints
│   ├── orca-seam      # Layered context archiving
│   ├── orca-checkpoint # Session persistence
│   ├── orca-loop-guard # Repetition detection
│   └── orca-events    # SQLite event store
├── orca-tui           # ratatui rendering
├── orca-mcp           # MCP client
└── orca-config        # TOML configuration
```

## Built-in Tools

| Tool | Description | Sandbox |
|------|-------------|---------|
| `read_file` | Read file contents | No |
| `write_file` | Write content to file | No |
| `glob` | Find files by pattern | No |
| `grep` | Search file contents by regex | No |
| `execute_shell` | Execute shell commands | Yes |
| `git` | Git operations (status, diff, log, add, commit, etc.) | No |
| `web_fetch` | Fetch URL content | Yes |
| `web_search` | Web search via DuckDuckGo | Yes |

## Key Components

### CanonicalState (Capacity Control)

Tracks semantic state across the session:
- **Goals** - Active/completed/failed objectives
- **Constraints** - Hard rules that must not be violated
- **Facts** - Known information with provenance
- **Open Loops** - Pending work items with priority

Three evaluation checkpoints:
1. **PreRequest** - Before model call (context usage, goal count, iteration limits)
2. **PostTool** - After tool execution (state consistency)
3. **ErrorEscalation** - On errors (consecutive failure tracking)

### LoopGuard

Detects and prevents repetitive tool calls:
- Sliding window analysis (default: 6 calls)
- Mutation-aware clearing (writes reset read-only tracking)
- Graduated response: Allow → SuppressWithCorrection → SuppressSilently → Block
- Per-tool consecutive failure tracking with auto-block

### Seam Manager (Context Archiving)

Four-layer compression system:
- **Recent** (0-10min) - No compression
- **Warm** (10-30min) - 50% compression, max 2000 chars
- **Cool** (30min-2hr) - 30% compression, max 500 chars
- **Cold** (2hr+) - 10% compression, max 200 chars

User anchors (goals, decisions, constraints) are preserved across compression.

### Tool Name Resolution

5-step fuzzy matching:
1. Exact match
2. Case-insensitive (`Read_File` → `read_file`)
3. Hyphen normalization (`read-file` → `read_file`)
4. CamelCase conversion (`readFile` → `read_file`)
5. Prefix fuzzy (`read` → `read_file`)

Legacy format support: `[TOOL_CALL]`, XML `<tool>`, JSON code blocks

### Argument Repair

6-stage JSON repair pipeline:
1. Direct parse attempt
2. Control character removal
3. Trailing comma removal
4. Bracket balancing
5. Extra closure stripping
6. Double-encoded JSON detection

## Test Coverage

65 tests across all crates, all passing:

| Crate | Tests |
|-------|-------|
| orca-utils | 2 |
| orca-config | 2 |
| orca-events | 3 |
| orca-name-resolver | 10 |
| orca-arg-repair | 8 |
| orca-capacity | 7 |
| orca-seam | 3 |
| orca-checkpoint | 5 |
| orca-sandbox | 5 |
| orca-loop-guard | 8 |
| orca-tools | 4 |
| orca-providers | 1 |

## Development

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p orca-tools

# Build release
cargo build --release

# Check for warnings
cargo clippy --workspace

# Format code
cargo fmt --workspace
```

## Project Structure

```
orca-agent/
├── Cargo.toml          # Workspace definition
├── README.md           # This file
├── crates/
│   ├── orca-cli/       # CLI entry point (binary)
│   ├── orca-tui/       # TUI rendering (ratatui)
│   ├── orca-core/      # Agent loop + session
│   ├── orca-providers/  # Provider trait + router
│   ├── orca-deepseek/   # DeepSeek provider
│   ├── orca-anthropic/  # Anthropic provider
│   ├── orca-openai/     # OpenAI provider
│   ├── orca-tools/      # Tool executor + builtins
│   ├── orca-sandbox/    # OS-level sandbox
│   ├── orca-name-resolver/ # Fuzzy name matching
│   ├── orca-arg-repair/ # JSON argument repair
│   ├── orca-capacity/   # Semantic state tracking
│   ├── orca-seam/       # Context archiving
│   ├── orca-checkpoint/ # Session persistence
│   ├── orca-loop-guard/ # Loop detection
│   ├── orca-events/     # Event logging
│   ├── orca-mcp/        # MCP client
│   ├── orca-config/     # Configuration
│   └── orca-utils/      # Shared utilities
└── tools/
    ├── filesystem/      # Filesystem tool (stub)
    ├── git/             # Git tool (stub)
    ├── lsp/             # LSP tool (stub)
    ├── shell/           # Shell tool (stub)
    └── web/             # Web tool (stub)
```

## License

MIT
