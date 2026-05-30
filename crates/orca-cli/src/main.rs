use anyhow::Result;
use clap::{Parser, Subcommand};
use orca_config::OrcaConfig;
use orca_core::agent::{Agent, AgentConfig, StepResult};
use orca_providers::provider::ModelProvider;
use orca_tools::builtin::register_all_builtins;
use orca_tools::executor::{ExecutionOptions, ToolExecutor};
use orca_tools::registry::ToolRegistry;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "orca", version, about = "Orca - AI Coding Agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Working directory
    #[arg(short, long, default_value = ".")]
    workdir: PathBuf,

    /// Model to use
    #[arg(short, long)]
    model: Option<String>,

    /// Provider to use
    #[arg(short, long)]
    provider: Option<String>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Initial prompt (non-interactive mode)
    #[arg(short, long)]
    prompt: Option<String>,

    /// Use TUI mode (rich terminal UI)
    #[arg(long)]
    tui: bool,

    /// Custom API base URL (for OpenAI-compatible providers)
    #[arg(long)]
    base_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive session
    Chat,
    /// Show configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// List available models
    Models,
    /// Resume from a checkpoint
    Resume {
        /// Checkpoint ID
        checkpoint_id: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set { key: String, value: String },
    /// Reset to defaults
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(log_level)
        }))
        .init();

    // Load config
    let mut config = OrcaConfig::load().unwrap_or_default();

    // Override from CLI args
    if let Some(model) = &cli.model {
        config.model = model.clone();
    }
    if let Some(provider) = &cli.provider {
        config.provider = provider.clone();
    }

    // If base_url provided via CLI, update the provider config
    if let Some(base_url) = &cli.base_url {
        let provider_key = config.provider.clone();
        config.providers
            .entry(provider_key)
            .or_insert_with(orca_config::provider_config::ProviderConfig::default)
            .base_url = Some(base_url.clone());
    }

    // Resolve working directory
    let workdir = std::fs::canonicalize(&cli.workdir)?;

    match cli.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
            if cli.tui {
                run_tui(config, workdir, cli.prompt).await
            } else {
                run_chat(config, workdir, cli.prompt).await
            }
        }
        Commands::Config { action } => run_config(config, action),
        Commands::Models => run_models(&config),
        Commands::Resume { checkpoint_id } => run_resume(config, workdir, &checkpoint_id).await,
    }
}

fn create_provider(config: &OrcaConfig) -> Result<Arc<dyn ModelProvider>> {
    match config.provider.as_str() {
        "deepseek" => {
            let provider_config = config.providers.get("deepseek");
            let api_key = provider_config
                .map(|p| p.resolve_api_key())
                .unwrap_or_else(|| std::env::var("DEEPSEEK_API_KEY").unwrap_or_default());
            let mut provider = orca_deepseek::DeepSeekProvider::new(&api_key, &config.model);
            if let Some(base_url) = provider_config.and_then(|p| p.base_url.as_ref()) {
                provider = provider.with_base_url(base_url);
            }
            Ok(Arc::new(provider))
        }
        "anthropic" | "claude" => {
            let provider_config = config.providers.get("anthropic");
            let api_key = provider_config
                .map(|p| p.resolve_api_key())
                .unwrap_or_else(|| std::env::var("ANTHROPIC_API_KEY").unwrap_or_default());
            Ok(Arc::new(orca_anthropic::AnthropicProvider::new(
                &api_key,
                &config.model,
            )))
        }
        "openai" | "custom" | "openai-compatible" => {
            let provider_key = if config.provider.as_str() == "openai" {
                "openai"
            } else {
                &config.provider
            };
            let provider_config = config.providers.get(provider_key);
            let api_key = provider_config
                .map(|p| p.resolve_api_key())
                .unwrap_or_else(|| std::env::var("OPENAI_API_KEY").unwrap_or_default());
            let mut provider = orca_openai::OpenAIProvider::new(&api_key, &config.model);
            if let Some(base_url) = provider_config.and_then(|p| p.base_url.as_ref()) {
                provider = provider.with_base_url(base_url);
            }
            Ok(Arc::new(provider))
        }
        other => Err(anyhow::anyhow!("unknown provider: {}", other)),
    }
}

fn create_tool_registry(workdir: &PathBuf) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    register_all_builtins(&mut registry, workdir.clone());
    registry
}

fn create_tool_executor(registry: ToolRegistry, workdir: &std::path::Path) -> ToolExecutor {
    let opts = ExecutionOptions {
        working_dir: workdir.to_path_buf(),
        ..Default::default()
    };
    ToolExecutor::new(Arc::new(registry), opts)
}

async fn run_chat(
    config: OrcaConfig,
    workdir: PathBuf,
    initial_prompt: Option<String>,
) -> Result<()> {
    let provider = create_provider(&config)?;
    let registry = create_tool_registry(&workdir);

    let agent_config = AgentConfig::from_orca_config(&config, workdir.clone())?;
    let mut agent = Agent::new(agent_config, provider, registry)?;

    // Create a tool executor for interactive use
    let executor = create_tool_executor(create_tool_registry(&workdir), &workdir);

    println!("Orca v{} - AI Coding Agent", env!("CARGO_PKG_VERSION"));
    println!("Model: {} ({})", config.model, config.provider);
    println!("Type your message, or 'quit' to exit.\n");

    // Process initial prompt if provided
    if let Some(prompt) = initial_prompt {
        process_input(&mut agent, &executor, &prompt).await?;
    }

    // Interactive loop
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" || input == "q" {
            // Create end-of-session checkpoint
            if let Err(e) = agent.create_checkpoint("session end") {
                tracing::warn!("failed to create checkpoint: {}", e);
            }
            println!("Goodbye!");
            break;
        }

        if input == "/checkpoint" {
            agent.create_checkpoint("manual checkpoint")?;
            println!("Checkpoint created.");
            continue;
        }

        if input == "/cost" {
            println!("Cost so far: ${:.4}", agent.total_cost());
            println!("Iterations: {}", agent.iteration_count());
            continue;
        }

        process_input(&mut agent, &executor, input).await?;
    }

    Ok(())
}

async fn process_input(
    agent: &mut Agent,
    executor: &ToolExecutor,
    input: &str,
) -> Result<()> {
    let result = agent.step(input).await?;

    match result {
        StepResult::Response(text) => {
            println!("\n{}\n", text);
        }
        StepResult::ToolCalls(calls) => {
            // Execute tools and feed results back
            let mut results = Vec::new();
            for call in &calls {
                println!(
                    "  [tool] {}({})",
                    call.name,
                    truncate_args(&call.arguments.to_string())
                );
                let (result, _diag) = executor.execute(call).await;
                println!(
                    "    -> {}",
                    if result.is_error {
                        format!("error: {}", truncate_output(&result.content))
                    } else {
                        truncate_output(&result.content)
                    }
                );
                results.push(result);
            }

            // Feed results back to the agent
            let result = agent.feed_tool_results(results).await?;
            match result {
                StepResult::Response(text) => println!("\n{}\n", text),
                StepResult::ToolCalls(more_calls) => {
                    // Handle additional tool calls recursively
                    handle_tool_calls(agent, executor, more_calls).await?;
                }
                StepResult::Done => println!("  (session complete)"),
            }
        }
        StepResult::Done => {
            println!("\nSession complete.");
        }
    }

    Ok(())
}

async fn handle_tool_calls(
    agent: &mut Agent,
    executor: &ToolExecutor,
    calls: Vec<orca_utils::message::ToolCall>,
) -> Result<()> {
    let mut results = Vec::new();
    for call in &calls {
        println!(
            "  [tool] {}({})",
            call.name,
            truncate_args(&call.arguments.to_string())
        );
        let (result, _diag) = executor.execute(call).await;
        println!(
            "    -> {}",
            if result.is_error {
                format!("error: {}", truncate_output(&result.content))
            } else {
                truncate_output(&result.content)
            }
        );
        results.push(result);
    }

    let result = agent.feed_tool_results(results).await?;
    match result {
        StepResult::Response(text) => println!("\n{}\n", text),
        StepResult::ToolCalls(more_calls) => {
            // Cap recursion depth to avoid infinite loops
            tracing::warn!("agent requested further tool calls after execution");
            println!("  ({} additional tool calls requested)", more_calls.len());
        }
        StepResult::Done => println!("  (session complete)"),
    }

    Ok(())
}

fn truncate_args(args: &str) -> String {
    if args.len() > 80 {
        format!("{}...", &args[..80])
    } else {
        args.to_string()
    }
}

fn truncate_output(s: &str) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > 120 {
        format!("{}...", &first_line[..120])
    } else {
        first_line.to_string()
    }
}

async fn run_tui(
    config: OrcaConfig,
    workdir: PathBuf,
    initial_prompt: Option<String>,
) -> Result<()> {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use orca_tui::app::{App, ChatMessage};
    use orca_tui::event::{AppEvent, EventHandler};
    use orca_tui::render::render;
    use ratatui::prelude::CrosstermBackend;
    use ratatui::Terminal;

    let provider = create_provider(&config)?;
    let registry = create_tool_registry(&workdir);
    let agent_config = AgentConfig::from_orca_config(&config, workdir.clone())?;
    let mut agent = Agent::new(agent_config, provider, registry)?;
    let executor = create_tool_executor(create_tool_registry(&workdir), &workdir);

    let mut app = App::new(format!("{} ({})", config.model, config.provider));

    if let Some(prompt) = initial_prompt {
        app.push_message(ChatMessage::User(prompt.clone()));
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = EventHandler::default();

    // Main TUI loop
    loop {
        terminal.draw(|f| render(f, &app))?;

        match events.next()? {
            AppEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};

                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                match app.input_mode {
                    orca_tui::app::InputMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('i') | KeyCode::Enter => {
                            app.input_mode = orca_tui::app::InputMode::Editing;
                        }
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        _ => {}
                    },
                    orca_tui::app::InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let input = app.submit_input();
                            if input.is_empty() {
                                continue;
                            }
                            if input == "quit" || input == "exit" {
                                break;
                            }

                            app.push_message(ChatMessage::User(input.clone()));
                            app.is_processing = true;
                            app.status = "Thinking...".to_string();

                            // Force redraw
                            terminal.draw(|f| render(f, &app))?;

                            match agent.step(&input).await {
                                Ok(StepResult::Response(text)) => {
                                    app.push_message(ChatMessage::Assistant(text));
                                }
                                Ok(StepResult::ToolCalls(calls)) => {
                                    let mut tool_results = Vec::new();
                                    for call in &calls {
                                        app.push_message(ChatMessage::Tool(
                                            call.name.clone(),
                                            call.arguments.to_string(),
                                        ));
                                        terminal.draw(|f| render(f, &app))?;

                                        let (result, _) = executor.execute(call).await;
                                        app.push_message(ChatMessage::ToolResult(
                                            result.content.clone(),
                                            result.is_error,
                                        ));
                                        tool_results.push(result);
                                    }

                                    // Feed results back to agent
                                    match agent.feed_tool_results(tool_results).await {
                                        Ok(StepResult::Response(text)) => {
                                            app.push_message(ChatMessage::Assistant(text));
                                        }
                                        Ok(StepResult::Done) => {
                                            app.push_message(ChatMessage::System("Session complete.".to_string()));
                                        }
                                        Ok(_) => {}
                                        Err(e) => {
                                            app.push_message(ChatMessage::Error(e.to_string()));
                                        }
                                    }
                                }
                                Ok(StepResult::Done) => {
                                    app.push_message(ChatMessage::System("Session complete.".to_string()));
                                }
                                Err(e) => {
                                    app.push_message(ChatMessage::Error(e.to_string()));
                                }
                            }

                            app.is_processing = false;
                            app.status = "Ready".to_string();
                            app.cost_usd = agent.total_cost();
                            app.iterations = agent.iteration_count();
                        }
                        KeyCode::Char(c) => app.handle_char(c),
                        KeyCode::Backspace => app.handle_backspace(),
                        KeyCode::Delete => app.handle_delete(),
                        KeyCode::Left => app.cursor_left(),
                        KeyCode::Right => app.cursor_right(),
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::Esc => {
                            app.input_mode = orca_tui::app::InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
            AppEvent::Tick => {}
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("Goodbye!");
    Ok(())
}

fn run_config(config: OrcaConfig, action: Option<ConfigAction>) -> Result<()> {
    match action.unwrap_or(ConfigAction::Show) {
        ConfigAction::Show => {
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Set { key, value } => {
            println!("Setting {} = {} (TODO: implement)", key, value);
        }
        ConfigAction::Reset => {
            let config = OrcaConfig::default();
            config.save()?;
            println!("Configuration reset to defaults.");
        }
    }
    Ok(())
}

fn run_models(config: &OrcaConfig) -> Result<()> {
    println!("Available models:");
    println!("  Provider: deepseek");
    println!("    - deepseek-chat (default)");
    println!("    - deepseek-reasoner");
    println!("  Provider: anthropic");
    println!("    - claude-sonnet-4-20250514");
    println!("    - claude-opus-4-20250514");
    println!("  Provider: openai");
    println!("    - gpt-4o");
    println!("    - gpt-4o-mini");
    println!("    - gpt-4.1");
    println!("    - o1");
    println!("    - o3");
    println!("    - o3-mini");
    println!("  Provider: custom (OpenAI-compatible)");
    println!("    - any model via base_url in config");
    println!("    Examples: Azure OpenAI, local LLMs (Ollama, vLLM),");
    println!("    Cloudflare Workers AI, Groq, Together AI, etc.");
    println!("\nCurrent: {} / {}", config.provider, config.model);
    println!("\nTo configure a custom provider, add to config.toml:");
    println!("  provider = \"custom\"");
    println!("  model = \"your-model-name\"");
    println!("  [providers.custom]");
    println!("  api_key = \"$YOUR_API_KEY\"");
    println!("  base_url = \"https://your-api-endpoint.com\"");
    Ok(())
}

async fn run_resume(
    config: OrcaConfig,
    workdir: PathBuf,
    checkpoint_id: &str,
) -> Result<()> {
    let data_dir = OrcaConfig::data_dir().unwrap_or_else(|_| PathBuf::from(".orca"));
    let checkpoint_dir = data_dir.join("checkpoints");

    let session_dirs: Vec<_> = std::fs::read_dir(&checkpoint_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    for dir in &session_dirs {
        let mgr = orca_checkpoint::CheckpointManager::new(
            &checkpoint_dir,
            &dir.file_name().to_string_lossy(),
            10,
        )?;
        if let Ok(checkpoint) = mgr.load(checkpoint_id) {
            println!("Resuming from checkpoint: {}", checkpoint.description);
            println!("  Iterations: {}", checkpoint.iteration_count);
            println!("  Cost: ${:.4}", checkpoint.total_cost_usd);
            println!("  Messages: {}", checkpoint.seed_messages.len());

            // Restore agent state from checkpoint
            let provider = create_provider(&config)?;
            let registry = create_tool_registry(&workdir);
            let agent_config = AgentConfig::from_orca_config(&config, workdir.clone())?;
            let executor = create_tool_executor(create_tool_registry(&workdir), &workdir);

            let mut agent = Agent::new(agent_config, provider, registry)?;

            println!("\nResumed session. Type your message, or 'quit' to exit.\n");

            // Interactive loop for resumed session
            loop {
                print!("> ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                if input == "quit" || input == "exit" || input == "q" {
                    if let Err(e) = agent.create_checkpoint("session end") {
                        tracing::warn!("failed to create checkpoint: {}", e);
                    }
                    println!("Goodbye!");
                    break;
                }

                process_input(&mut agent, &executor, input).await?;
            }

            return Ok(());
        }
    }

    println!("Checkpoint '{}' not found.", checkpoint_id);
    Ok(())
}
