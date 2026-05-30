use crate::session::Session;
use anyhow::Result;
use orca_capacity::{CapacityController, Checkpoint as CapCheckpoint, Intervention};
use orca_capacity::controller::CapacityConfig;
use orca_checkpoint::checkpoint::CheckpointData;
use orca_checkpoint::manager::CheckpointManager;
use orca_config::OrcaConfig;
use orca_events::{Event, EventKind, EventStore};
use orca_loop_guard::{LoopGuard, RepetitionAction};
use orca_providers::provider::ModelProvider;
use orca_providers::types::{ChatRequest, StopReason};
use orca_sandbox::policy::SandboxPolicy;
use orca_seam::SeamManager;
use orca_tools::executor::{ExecutionOptions, ToolExecutor};
use orca_tools::registry::ToolRegistry;
use orca_utils::message::{Message, Role, ToolCall, ToolResult};
use std::path::PathBuf;
use std::sync::Arc;
use tracing;

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub working_dir: PathBuf,
    pub data_dir: PathBuf,
    pub sandbox_policy: SandboxPolicy,
    pub max_context_tokens: usize,
    pub compress_threshold: f64,
    pub max_checkpoints: usize,
    pub max_iterations: u32,
    pub max_budget_usd: f64,
}

impl AgentConfig {
    pub fn from_orca_config(config: &OrcaConfig, working_dir: PathBuf) -> Result<Self> {
        let data_dir = OrcaConfig::data_dir()?;
        Ok(Self {
            system_prompt: String::new(),
            working_dir,
            data_dir,
            sandbox_policy: SandboxPolicy::default(),
            max_context_tokens: config.context.max_context_tokens,
            compress_threshold: config.context.compress_threshold,
            max_checkpoints: config.context.max_checkpoints,
            max_iterations: config.budget.max_iterations,
            max_budget_usd: config.budget.max_session_budget_usd,
        })
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are Orca, an AI coding assistant. Be helpful, accurate, and concise.".to_string(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            data_dir: PathBuf::from(".orca"),
            sandbox_policy: SandboxPolicy::default(),
            max_context_tokens: 128_000,
            compress_threshold: 0.75,
            max_checkpoints: 10,
            max_iterations: 200,
            max_budget_usd: 10.0,
        }
    }
}

/// The main agent, orchestrating all components
pub struct Agent {
    config: AgentConfig,
    session: Session,
    provider: Arc<dyn ModelProvider>,
    tool_executor: ToolExecutor,
    loop_guard: LoopGuard,
    capacity_ctrl: CapacityController,
    seam_mgr: SeamManager,
    checkpoint_mgr: Option<CheckpointManager>,
    event_store: Option<EventStore>,
    system_prompt: String,
}

/// Result of a single agent step
#[derive(Debug)]
pub enum StepResult {
    /// Agent produced a final text response
    Response(String),
    /// Agent wants to call tools (caller should execute and feed back results)
    ToolCalls(Vec<ToolCall>),
    /// Agent wants to stop
    Done,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn ModelProvider>,
        registry: ToolRegistry,
    ) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let model_info = provider.model_info();
        let model_id = model_info.id.to_string();

        let tool_executor = {
            let opts = ExecutionOptions {
                working_dir: config.working_dir.clone(),
                sandbox_policy: config.sandbox_policy.clone(),
                resolve_names: true,
                repair_args: true,
            };
            ToolExecutor::new(Arc::new(registry), opts)
        };

        let loop_guard = LoopGuard::default();
        let capacity_ctrl = CapacityController::new(CapacityConfig {
            max_iterations: config.max_iterations,
            context_usage_warn_threshold: config.compress_threshold,
            context_usage_restart_threshold: config.compress_threshold + 0.15,
            ..Default::default()
        });
        let seam_mgr = SeamManager::new(50);

        // Set up checkpoint manager
        let checkpoint_dir = config.data_dir.join("checkpoints");
        let checkpoint_mgr = CheckpointManager::new(&checkpoint_dir, &session_id, config.max_checkpoints).ok();

        // Set up event store
        let event_dir = config.data_dir.join("events");
        std::fs::create_dir_all(&event_dir).ok();
        let event_store = EventStore::open(&event_dir.join("events.db")).ok();

        Ok(Self {
            config: config.clone(),
            session: Session::new(&model_id),
            provider,
            tool_executor,
            loop_guard,
            capacity_ctrl,
            seam_mgr,
            checkpoint_mgr,
            event_store,
            system_prompt: config.system_prompt,
        })
    }

    /// Send a user message and get the next step
    pub async fn step(&mut self, user_input: &str) -> Result<StepResult> {
        // Add user message
        let user_msg = Message::user(user_input);
        self.session.conversation.push(user_msg);

        // Record event
        self.record_event(EventKind::MessageSent, serde_json::json!({
            "role": "user",
            "content_len": user_input.len()
        }));

        // Run the agent loop until we get a response or tool calls
        self.run_until_response().await
    }

    /// Feed tool results back to the agent
    pub async fn feed_tool_results(&mut self, results: Vec<ToolResult>) -> Result<StepResult> {
        for result in &results {
            // Track failures in loop guard
            if result.is_error {
                self.loop_guard.record_failure("tool");
            } else {
                self.loop_guard.record_success("tool");
            }

            let msg = Message::tool_result(result.clone());
            self.session.conversation.push(msg);
        }

        self.record_event(EventKind::ToolCallEnd, serde_json::json!({
            "count": results.len()
        }));

        self.run_until_response().await
    }

    /// Internal: run the model until it produces text or tool calls
    async fn run_until_response(&mut self) -> Result<StepResult> {
        loop {
            // Check budget
            if self.session.total_cost_usd >= self.config.max_budget_usd {
                return Ok(StepResult::Done);
            }

            if self.session.iteration_count >= self.config.max_iterations {
                return Ok(StepResult::Done);
            }

            // Check loop guard
            if self.loop_guard.is_blocked() {
                tracing::warn!("loop guard blocked: {}", self.loop_guard.block_reason().unwrap_or("unknown"));
                return Ok(StepResult::Done);
            }

            // Evaluate capacity
            let usage = self.session.context_usage_ratio(self.config.max_context_tokens);
            self.capacity_ctrl.set_context_usage(usage);

            let intervention = self.capacity_ctrl.evaluate(&self.session.state, CapCheckpoint::PreRequest);
            match intervention {
                Intervention::None => {}
                Intervention::Warn(msg) => {
                    tracing::warn!("capacity warning: {}", msg);
                }
                Intervention::CheckpointRestart => {
                    tracing::info!("checkpoint restart requested");
                    self.create_checkpoint("auto checkpoint before restart")?;
                    // Could restore from checkpoint here, for now just warn
                }
                Intervention::Replan => {
                    tracing::info!("replan requested");
                }
                Intervention::Abort(reason) => {
                    tracing::error!("abort: {}", reason);
                    return Ok(StepResult::Done);
                }
            }

            // Compress context if needed
            if usage >= self.config.compress_threshold {
                let result = self.seam_mgr.compress(&mut self.session.conversation.messages);
                if result.messages_compressed > 0 {
                    tracing::info!(
                        compressed = result.messages_compressed,
                        tokens_saved = result.tokens_saved,
                        "context compressed"
                    );
                }
            }

            // Build request
            let request = ChatRequest {
                messages: self.session.conversation.messages.clone(),
                tools: self.tool_executor.registry().definitions(),
                max_tokens: Some(8192),
                temperature: None,
                system_prompt: Some(self.system_prompt.clone()),
            };

            // Call model
            let response = match self.provider.chat(request).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("model call failed: {}", e);
                    self.record_event(EventKind::Error, serde_json::json!({"error": e.to_string()}));
                    return Err(e.into());
                }
            };

            self.session.iteration_count += 1;
            self.capacity_ctrl.record_success();

            // Estimate cost
            let model_info = self.provider.model_info();
            let cost = response.usage.cost_usd(model_info.input_cost_per_mtok, model_info.output_cost_per_mtok);
            self.session.add_cost(cost);

            self.record_event(EventKind::MessageReceived, serde_json::json!({
                "stop_reason": format!("{:?}", response.stop_reason),
                "usage": {
                    "prompt_tokens": response.usage.prompt_tokens,
                    "completion_tokens": response.usage.completion_tokens,
                    "cost_usd": cost
                }
            }));

            // Add assistant message
            let assistant_msg = Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: Role::Assistant,
                content: response.content.clone(),
                tool_calls: response.tool_calls.clone(),
                tool_call_id: None,
                timestamp: chrono::Utc::now(),
            };
            self.session.conversation.push(assistant_msg);

            // Process response
            match response.stop_reason {
                StopReason::EndTurn | StopReason::MaxTokens => {
                    if response.tool_calls.is_empty() {
                        return Ok(StepResult::Response(response.content));
                    }
                    // Has tool calls, return them
                    return Ok(StepResult::ToolCalls(response.tool_calls));
                }
                StopReason::ToolUse => {
                    // Check loop guard for each tool call
                    let mut allowed_calls = Vec::new();
                    for call in &response.tool_calls {
                        match self.loop_guard.check(call) {
                            RepetitionAction::Allow => allowed_calls.push(call.clone()),
                            RepetitionAction::SuppressWithCorrection(msg) => {
                                tracing::warn!("suppressing tool call: {}", msg);
                                // Add correction as a user message
                                self.session.conversation.push(Message::system(&msg));
                            }
                            RepetitionAction::SuppressSilently => {
                                tracing::warn!("silently suppressing tool call: {}", call.name);
                            }
                            RepetitionAction::Block(reason) => {
                                tracing::error!("blocking tool call: {}", reason);
                                return Ok(StepResult::Done);
                            }
                        }
                    }

                    if allowed_calls.is_empty() {
                        // All calls suppressed, give the model a chance to respond differently
                        self.session.conversation.push(Message::system(
                            "All your recent tool calls have been suppressed due to repetition. \
                             Please try a completely different approach."
                        ));
                        continue; // Loop back to call model again
                    }

                    return Ok(StepResult::ToolCalls(allowed_calls));
                }
                StopReason::ContentFilter => {
                    tracing::warn!("content filter triggered");
                    return Ok(StepResult::Done);
                }
                StopReason::Unknown(reason) => {
                    tracing::warn!("unknown stop reason: {}", reason);
                    if !response.tool_calls.is_empty() {
                        return Ok(StepResult::ToolCalls(response.tool_calls));
                    }
                    return Ok(StepResult::Response(response.content));
                }
            }
        }
    }

    /// Create a checkpoint of the current session
    pub fn create_checkpoint(&self, description: &str) -> Result<()> {
        if let Some(ref mgr) = self.checkpoint_mgr {
            let checkpoint = CheckpointData {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: self.session.id.clone(),
                seed_messages: self.session.conversation.messages.clone(),
                completed_goals: self.session.state.goals.iter()
                    .filter(|g| g.status == orca_capacity::GoalStatus::Completed)
                    .map(|g| orca_checkpoint::checkpoint::CompletedGoal {
                        description: g.description.clone(),
                        completed_at: g.completed_at.unwrap_or(chrono::Utc::now()),
                        result_summary: String::new(),
                    })
                    .collect(),
                active_goals: self.session.state.goals.iter()
                    .filter(|g| matches!(g.status, orca_capacity::GoalStatus::Pending | orca_capacity::GoalStatus::InProgress))
                    .map(|g| g.description.clone())
                    .collect(),
                anchor_context: self.seam_mgr.anchors().format_as_context(),
                state_snapshot: serde_json::to_value(&self.session.state).unwrap_or_default(),
                iteration_count: self.session.iteration_count,
                total_cost_usd: self.session.total_cost_usd,
                created_at: chrono::Utc::now(),
                description: description.to_string(),
            };
            mgr.save(&checkpoint)?;
            self.record_event(EventKind::CheckpointCreated, serde_json::json!({
                "checkpoint_id": checkpoint.id,
                "description": description
            }));
        }
        Ok(())
    }

    /// Get the current session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get total cost so far
    pub fn total_cost(&self) -> f64 {
        self.session.total_cost_usd
    }

    /// Get iteration count
    pub fn iteration_count(&self) -> u32 {
        self.session.iteration_count
    }

    /// Get mutable access to the session
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Clear all conversation messages while preserving session ID
    pub fn clear_conversation(&mut self) {
        self.session.conversation.messages.clear();
        self.record_event(EventKind::MessageSent, serde_json::json!({
            "action": "clear_conversation"
        }));
    }

    /// Get the current system prompt
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Update the system prompt
    pub fn set_system_prompt(&mut self, prompt: String) {
        let prompt_len = prompt.len();
        self.system_prompt = prompt;
        self.record_event(EventKind::MessageSent, serde_json::json!({
            "action": "set_system_prompt",
            "prompt_length": prompt_len
        }));
    }

    /// Record an event
    fn record_event(&self, kind: EventKind, data: serde_json::Value) {
        if let Some(ref store) = self.event_store {
            let event = Event::new(&self.session.id, kind, data);
            if let Err(e) = store.record(&event) {
                tracing::warn!("failed to record event: {}", e);
            }
        }
    }
}