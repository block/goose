mod builder;
mod completion;
mod export;
mod input;
mod output;
mod prompt;
mod task_execution_display;
mod thinking;

use crate::session::task_execution_display::{
    format_task_execution_notification, TASK_EXECUTION_NOTIFICATION_TYPE,
};
use goose::conversation::Conversation;
use std::io::Write;
use std::str::FromStr;

pub use self::export::message_to_markdown;
pub use builder::{build_session, SessionBuilderConfig, SessionSettings};
use console::Color;
use goose::agents::AgentEvent;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::Permission;
use goose::permission::PermissionConfirmation;
use goose::providers::base::Provider;
use goose::utils::safe_truncate;

use anyhow::{Context, Result};
use completion::GooseCompleter;
use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::types::RetryConfig;
use goose::agents::{Agent, SessionConfig, MANUAL_COMPACT_TRIGGER};
use goose::config::{Config, GooseMode};
use goose::providers::pricing::initialize_pricing_cache;
use goose::session::SessionManager;
use input::InputResult;
use rmcp::model::PromptMessage;
use rmcp::model::ServerNotification;
use rmcp::model::{ErrorCode, ErrorData};

use goose::config::paths::Paths;
use goose::conversation::message::{Message, MessageContent};
use rand::{distributions::Alphanumeric, Rng};
use rustyline::EditMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio;
use tokio_util::sync::CancellationToken;
use tracing::warn;

#[derive(Serialize, Deserialize, Debug)]
struct JsonOutput {
    messages: Vec<Message>,
    metadata: JsonMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonMetadata {
    total_tokens: Option<i32>,
    status: String,
}

pub enum RunMode {
    Normal,
    Plan,
}

pub struct CliSession {
    agent: Agent,
    messages: Conversation,
    session_id: String,
    completion_cache: Arc<std::sync::RwLock<CompletionCache>>,
    debug: bool,
    run_mode: RunMode,
    scheduled_job_id: Option<String>, // ID of the scheduled job that triggered this session
    max_turns: Option<u32>,
    edit_mode: Option<EditMode>,
    retry_config: Option<RetryConfig>,
    output_format: String,
}

// Cache structure for completion data
struct CompletionCache {
    prompts: HashMap<String, Vec<String>>,
    prompt_info: HashMap<String, output::PromptInfo>,
    last_updated: Instant,
}

impl CompletionCache {
    fn new() -> Self {
        Self {
            prompts: HashMap::new(),
            prompt_info: HashMap::new(),
            last_updated: Instant::now(),
        }
    }
}

pub enum PlannerResponseType {
    Plan,
    ClarifyingQuestions,
}

/// Decide if the planner's reponse is a plan or a clarifying question
///
/// This function is called after the planner has generated a response
/// to the user's message. The response is either a plan or a clarifying
/// question.
pub async fn classify_planner_response(
    message_text: String,
    provider: Arc<dyn Provider>,
) -> Result<PlannerResponseType> {
    let prompt = format!("The text below is the output from an AI model which can either provide a plan or list of clarifying questions. Based on the text below, decide if the output is a \"plan\" or \"clarifying questions\".\n---\n{message_text}");

    // Generate the description
    let message = Message::user().with_text(&prompt);
    let (result, _usage) = provider
        .complete(
            "Reply only with the classification label: \"plan\" or \"clarifying questions\"",
            &[message],
            &[],
        )
        .await?;

    let predicted = result.as_concat_text();
    if predicted.to_lowercase().contains("plan") {
        Ok(PlannerResponseType::Plan)
    } else {
        Ok(PlannerResponseType::ClarifyingQuestions)
    }
}

impl CliSession {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        agent: Agent,
        session_id: String,
        debug: bool,
        scheduled_job_id: Option<String>,
        max_turns: Option<u32>,
        edit_mode: Option<EditMode>,
        retry_config: Option<RetryConfig>,
        output_format: String,
    ) -> Self {
        let messages = SessionManager::get_session(&session_id, true)
            .await
            .map(|session| session.conversation.unwrap_or_default())
            .unwrap();

        CliSession {
            agent,
            messages,
            session_id,
            completion_cache: Arc::new(std::sync::RwLock::new(CompletionCache::new())),
            debug,
            run_mode: RunMode::Normal,
            scheduled_job_id,
            max_turns,
            edit_mode,
            retry_config,
            output_format,
        }
    }

    pub fn session_id(&self) -> &String {
        &self.session_id
    }

    /// Add a stdio extension to the session
    ///
    /// # Arguments
    /// * `extension_command` - Full command string including environment variables
    ///   Format: "ENV1=val1 ENV2=val2 command args..."
    pub async fn add_extension(&mut self, extension_command: String) -> Result<()> {
        let mut parts: Vec<&str> = extension_command.split_whitespace().collect();
        let mut envs = HashMap::new();

        while let Some(part) = parts.first() {
            if !part.contains('=') {
                break;
            }
            let env_part = parts.remove(0);
            let (key, value) = env_part.split_once('=').unwrap();
            envs.insert(key.to_string(), value.to_string());
        }

        if parts.is_empty() {
            return Err(anyhow::anyhow!("No command provided in extension string"));
        }

        let cmd = parts.remove(0).to_string();
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::Stdio {
            name,
            cmd,
            args: parts.iter().map(|s| s.to_string()).collect(),
            envs: Envs::new(envs),
            env_keys: Vec::new(),
            description: goose::config::DEFAULT_EXTENSION_DESCRIPTION.to_string(),
            // TODO: should set timeout
            timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: None,
            available_tools: Vec::new(),
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))?;

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    /// Add a remote extension to the session
    ///
    /// # Arguments
    /// * `extension_url` - URL of the server
    pub async fn add_remote_extension(&mut self, extension_url: String) -> Result<()> {
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::Sse {
            name,
            uri: extension_url,
            envs: Envs::new(HashMap::new()),
            env_keys: Vec::new(),
            description: goose::config::DEFAULT_EXTENSION_DESCRIPTION.to_string(),
            // TODO: should set timeout
            timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: None,
            available_tools: Vec::new(),
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))?;

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    /// Add a streamable HTTP extension to the session
    ///
    /// # Arguments
    /// * `extension_url` - URL of the server
    pub async fn add_streamable_http_extension(&mut self, extension_url: String) -> Result<()> {
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::StreamableHttp {
            name,
            uri: extension_url,
            envs: Envs::new(HashMap::new()),
            env_keys: Vec::new(),
            headers: HashMap::new(),
            description: goose::config::DEFAULT_EXTENSION_DESCRIPTION.to_string(),
            // TODO: should set timeout
            timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: None,
            available_tools: Vec::new(),
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))?;

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    /// Add a builtin extension to the session
    ///
    /// # Arguments
    /// * `builtin_name` - Name of the builtin extension(s), comma separated
    pub async fn add_builtin(&mut self, builtin_name: String) -> Result<()> {
        for name in builtin_name.split(',') {
            let extension_name = name.trim().to_string();
            let config = ExtensionConfig::Builtin {
                name: extension_name,
                display_name: None,
                // TODO: should set a timeout
                timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
                bundled: None,
                description: name.trim().to_string(),
                available_tools: Vec::new(),
            };
            self.agent
                .add_extension(config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start builtin extension: {}", e))?;
        }

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    pub async fn list_prompts(
        &mut self,
        extension: Option<String>,
    ) -> Result<HashMap<String, Vec<String>>> {
        let prompts = self.agent.list_extension_prompts().await;

        // Early validation if filtering by extension
        if let Some(filter) = &extension {
            if !prompts.contains_key(filter) {
                return Err(anyhow::anyhow!("Extension '{}' not found", filter));
            }
        }

        // Convert prompts into filtered map of extension names to prompt names
        Ok(prompts
            .into_iter()
            .filter(|(ext, _)| extension.as_ref().is_none_or(|f| f == ext))
            .map(|(extension, prompt_list)| {
                let names = prompt_list.into_iter().map(|p| p.name).collect();
                (extension, names)
            })
            .collect())
    }

    pub async fn get_prompt_info(&mut self, name: &str) -> Result<Option<output::PromptInfo>> {
        let prompts = self.agent.list_extension_prompts().await;

        // Find which extension has this prompt
        for (extension, prompt_list) in prompts {
            if let Some(prompt) = prompt_list.iter().find(|p| p.name == name) {
                return Ok(Some(output::PromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    extension: Some(extension),
                }));
            }
        }

        Ok(None)
    }

    pub async fn get_prompt(&mut self, name: &str, arguments: Value) -> Result<Vec<PromptMessage>> {
        Ok(self.agent.get_prompt(name, arguments).await?.messages)
    }

    /// Process a single message and get the response
    pub(crate) async fn process_message(
        &mut self,
        message: Message,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let cancel_token = cancel_token.clone();
        self.push_message(message);
        self.process_agent_response(false, cancel_token).await?;
        Ok(())
    }

    /// Start an interactive session, optionally with an initial message
    pub async fn interactive(&mut self, prompt: Option<String>) -> Result<()> {
        // Process initial message if provided
        if let Some(prompt) = prompt {
            let msg = Message::user().with_text(&prompt);
            self.process_message(msg, CancellationToken::default())
                .await?;
        }

        // Initialize the completion cache
        self.update_completion_cache().await?;

        // Create a new editor with our custom completer
        let builder =
            rustyline::Config::builder().completion_type(rustyline::CompletionType::Circular);
        let builder = if let Some(edit_mode) = self.edit_mode {
            builder.edit_mode(edit_mode)
        } else {
            // Default to Emacs mode if no edit mode is set
            builder.edit_mode(EditMode::Emacs)
        };
        let config = builder.build();
        let mut editor =
            rustyline::Editor::<GooseCompleter, rustyline::history::DefaultHistory>::with_config(
                config,
            )?;

        // Set up the completer with a reference to the completion cache
        let completer = GooseCompleter::new(self.completion_cache.clone());
        editor.set_helper(Some(completer));

        let history_file = Paths::state_dir().join("history.txt");
        let old_history_file = Paths::config_dir().join("history.txt");

        if let Some(parent) = history_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let history_files = [&history_file, &old_history_file];
        let load_from = history_files.iter().find(|f| f.exists());

        if let Some(file) = load_from {
            if let Err(err) = editor.load_history(file) {
                eprintln!("Warning: Failed to load command history: {}", err);
            }
        }

        let save_history =
            |editor: &mut rustyline::Editor<GooseCompleter, rustyline::history::DefaultHistory>| {
                if let Err(err) = editor.save_history(&history_file) {
                    eprintln!("Warning: Failed to save command history: {}", err);
                } else if old_history_file.exists() {
                    if let Err(err) = std::fs::remove_file(&old_history_file) {
                        eprintln!("Warning: Failed to remove old history file: {}", err);
                    }
                }
            };

        output::display_greeting();
        loop {
            // Display context usage before each prompt
            self.display_context_usage().await?;

            match input::get_input(&mut editor)? {
                InputResult::Message(content) => {
                    match self.run_mode {
                        RunMode::Normal => {
                            save_history(&mut editor);

                            self.push_message(Message::user().with_text(&content));

                            // Track the current directory and last instruction in projects.json
                            if let Err(e) = crate::project_tracker::update_project_tracker(
                                Some(&content),
                                Some(&self.session_id),
                            ) {
                                eprintln!("Warning: Failed to update project tracker with instruction: {}", e);
                            }

                            let _provider = self.agent.provider().await?;

                            output::show_thinking();
                            let start_time = Instant::now();
                            self.process_agent_response(true, CancellationToken::default())
                                .await?;
                            output::hide_thinking();

                            // Display elapsed time
                            let elapsed = start_time.elapsed();
                            let elapsed_str = format_elapsed_time(elapsed);
                            println!(
                                "\n{}",
                                console::style(format!("⏱️  Elapsed time: {}", elapsed_str)).dim()
                            );
                        }
                        RunMode::Plan => {
                            let mut plan_messages = self.messages.clone();
                            plan_messages.push(Message::user().with_text(&content));
                            let reasoner = get_reasoner().await?;
                            self.plan_with_reasoner_model(plan_messages, reasoner)
                                .await?;
                        }
                    }
                }
                input::InputResult::Exit => break,
                input::InputResult::AddExtension(cmd) => {
                    save_history(&mut editor);

                    match self.add_extension(cmd.clone()).await {
                        Ok(_) => output::render_extension_success(&cmd),
                        Err(e) => output::render_extension_error(&cmd, &e.to_string()),
                    }
                }
                input::InputResult::AddBuiltin(names) => {
                    save_history(&mut editor);

                    match self.add_builtin(names.clone()).await {
                        Ok(_) => output::render_builtin_success(&names),
                        Err(e) => output::render_builtin_error(&names, &e.to_string()),
                    }
                }
                input::InputResult::ToggleTheme => {
                    save_history(&mut editor);

                    let current = output::get_theme();
                    let new_theme = match current {
                        output::Theme::Ansi => {
                            println!("Switching to Light theme");
                            output::Theme::Light
                        }
                        output::Theme::Light => {
                            println!("Switching to Dark theme");
                            output::Theme::Dark
                        }
                        output::Theme::Dark => {
                            println!("Switching to Ansi theme");
                            output::Theme::Ansi
                        }
                    };
                    output::set_theme(new_theme);
                    continue;
                }

                input::InputResult::SelectTheme(theme_name) => {
                    save_history(&mut editor);

                    let new_theme = match theme_name.as_str() {
                        "light" => {
                            println!("Switching to Light theme");
                            output::Theme::Light
                        }
                        "dark" => {
                            println!("Switching to Dark theme");
                            output::Theme::Dark
                        }
                        "ansi" => {
                            println!("Switching to Ansi theme");
                            output::Theme::Ansi
                        }
                        _ => output::Theme::Dark,
                    };
                    output::set_theme(new_theme);
                    continue;
                }
                input::InputResult::Retry => continue,
                input::InputResult::ListPrompts(extension) => {
                    save_history(&mut editor);

                    match self.list_prompts(extension).await {
                        Ok(prompts) => output::render_prompts(&prompts),
                        Err(e) => output::render_error(&e.to_string()),
                    }
                }
                input::InputResult::GooseMode(mode) => {
                    save_history(&mut editor);

                    let config = Config::global();
                    let mode = match GooseMode::from_str(&mode.to_lowercase()) {
                        Ok(mode) => mode,
                        Err(_) => {
                            output::render_error(&format!(
                                "Invalid mode '{}'. Mode must be one of: auto, approve, chat, smart_approve",
                                mode
                            ));
                            continue;
                        }
                    };
                    config.set_goose_mode(mode)?;
                    output::goose_mode_message(&format!("Goose mode set to '{:?}'", mode));
                    continue;
                }
                input::InputResult::Plan(options) => {
                    self.run_mode = RunMode::Plan;
                    output::render_enter_plan_mode();

                    let message_text = options.message_text;
                    if message_text.is_empty() {
                        continue;
                    }
                    let mut plan_messages = self.messages.clone();
                    plan_messages.push(Message::user().with_text(&message_text));

                    let reasoner = get_reasoner().await?;
                    self.plan_with_reasoner_model(plan_messages, reasoner)
                        .await?;
                }
                input::InputResult::EndPlan => {
                    self.run_mode = RunMode::Normal;
                    output::render_exit_plan_mode();
                    continue;
                }
                input::InputResult::Clear => {
                    save_history(&mut editor);

                    if let Err(e) = SessionManager::replace_conversation(
                        &self.session_id,
                        &Conversation::default(),
                    )
                    .await
                    {
                        output::render_error(&format!("Failed to clear session: {}", e));
                        continue;
                    }

                    self.messages.clear();
                    tracing::info!("Chat context cleared by user.");
                    output::render_message(
                        &Message::assistant().with_text("Chat context cleared."),
                        self.debug,
                    );

                    continue;
                }
                input::InputResult::PromptCommand(opts) => {
                    save_history(&mut editor);
                    self.handle_prompt_command(opts).await?;
                }
                InputResult::Recipe(filepath_opt) => {
                    println!("{}", console::style("Generating Recipe").green());

                    output::show_thinking();
                    let recipe = self.agent.create_recipe(self.messages.clone()).await;
                    output::hide_thinking();

                    match recipe {
                        Ok(recipe) => {
                            // Use provided filepath or default
                            let filepath_str = filepath_opt.as_deref().unwrap_or("recipe.yaml");
                            match self.save_recipe(&recipe, filepath_str) {
                                Ok(path) => println!(
                                    "{}",
                                    console::style(format!("Saved recipe to {}", path.display()))
                                        .green()
                                ),
                                Err(e) => {
                                    println!("{}", console::style(e).red());
                                }
                            }
                        }
                        Err(e) => {
                            println!(
                                "{}: {:?}",
                                console::style("Failed to generate recipe").red(),
                                e
                            );
                        }
                    }

                    continue;
                }
                InputResult::Compact => {
                    save_history(&mut editor);

                    let prompt = "Are you sure you want to compact this conversation? This will condense the message history.";
                    let should_summarize =
                        match cliclack::confirm(prompt).initial_value(true).interact() {
                            Ok(choice) => choice,
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::Interrupted {
                                    false
                                } else {
                                    return Err(e.into());
                                }
                            }
                        };

                    if should_summarize {
                        self.push_message(Message::user().with_text(MANUAL_COMPACT_TRIGGER));
                        output::show_thinking();
                        self.process_agent_response(true, CancellationToken::default())
                            .await?;
                        output::hide_thinking();
                    } else {
                        println!("{}", console::style("Compaction cancelled.").yellow());
                    }
                    continue;
                }
            }
        }

        println!(
            "Closing session. Session ID: {}",
            console::style(&self.session_id).cyan()
        );

        Ok(())
    }

    async fn plan_with_reasoner_model(
        &mut self,
        plan_messages: Conversation,
        reasoner: Arc<dyn Provider>,
    ) -> Result<(), anyhow::Error> {
        let plan_prompt = self.agent.get_plan_prompt().await?;
        output::show_thinking();
        let (plan_response, _usage) = reasoner
            .complete(&plan_prompt, plan_messages.messages(), &[])
            .await?;
        output::render_message(&plan_response, self.debug);
        output::hide_thinking();
        let planner_response_type =
            classify_planner_response(plan_response.as_concat_text(), self.agent.provider().await?)
                .await?;

        match planner_response_type {
            PlannerResponseType::Plan => {
                println!();
                let should_act = match cliclack::confirm(
                    "Do you want to clear message history & act on this plan?",
                )
                .initial_value(true)
                .interact()
                {
                    Ok(choice) => choice,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::Interrupted {
                            false // If interrupted, set should_act to false
                        } else {
                            return Err(e.into());
                        }
                    }
                };
                if should_act {
                    output::render_act_on_plan();
                    self.run_mode = RunMode::Normal;
                    // set goose mode: auto if that isn't already the case
                    let config = Config::global();
                    let curr_goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
                    if curr_goose_mode != GooseMode::Auto {
                        config.set_goose_mode(GooseMode::Auto).unwrap();
                    }

                    // clear the messages before acting on the plan
                    self.messages.clear();
                    // add the plan response as a user message
                    let plan_message = Message::user().with_text(plan_response.as_concat_text());
                    self.push_message(plan_message);
                    // act on the plan
                    output::show_thinking();
                    self.process_agent_response(true, CancellationToken::default())
                        .await?;
                    output::hide_thinking();

                    // Reset run & goose mode
                    if curr_goose_mode != GooseMode::Auto {
                        config.set_goose_mode(curr_goose_mode)?;
                    }
                } else {
                    // add the plan response (assistant message) & carry the conversation forward
                    // in the next round, the user might wanna slightly modify the plan
                    self.push_message(plan_response);
                }
            }
            PlannerResponseType::ClarifyingQuestions => {
                // add the plan response (assistant message) & carry the conversation forward
                // in the next round, the user will answer the clarifying questions
                self.push_message(plan_response);
            }
        }

        Ok(())
    }

    /// Process a single message and exit
    pub async fn headless(&mut self, prompt: String) -> Result<()> {
        let message = Message::user().with_text(&prompt);
        self.process_message(message, CancellationToken::default())
            .await?;
        Ok(())
    }

    async fn process_agent_response(
        &mut self,
        interactive: bool,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let cancel_token_clone = cancel_token.clone();

        // Cache the output format check to avoid repeated string comparisons in the hot loop
        let is_json_mode = self.output_format == "json";

        let session_config = SessionConfig {
            id: self.session_id.clone(),
            schedule_id: self.scheduled_job_id.clone(),
            max_turns: self.max_turns,
            retry_config: self.retry_config.clone(),
        };
        let user_message = self
            .messages
            .last()
            .ok_or_else(|| anyhow::anyhow!("No user message"))?;
        let mut stream = self
            .agent
            .reply(
                user_message.clone(),
                session_config.clone(),
                Some(cancel_token.clone()),
            )
            .await?;

        let mut progress_bars = output::McpSpinners::new();

        use futures::StreamExt;
        loop {
            tokio::select! {
                result = stream.next() => {
                    match result {
                        Some(Ok(AgentEvent::Message(message))) => {
                            // If it's a confirmation request, get approval but otherwise do not render/persist
                            if let Some(MessageContent::ToolConfirmationRequest(confirmation)) = message.content.first() {
                                output::hide_thinking();

                                // Format the confirmation prompt - use security message if present, otherwise use generic message
                                let prompt = if let Some(security_message) = &confirmation.prompt {
                                    println!("\n{}", security_message);
                                    "Do you allow this tool call?".to_string()
                                } else {
                                    "Goose would like to call the above tool, do you allow?".to_string()
                                };

                                // Get confirmation from user
                                let permission_result = if confirmation.prompt.is_none() {
                                    // No security message - show all options including "Always Allow"
                                    cliclack::select(prompt)
                                        .item(Permission::AllowOnce, "Allow", "Allow the tool call once")
                                        .item(Permission::AlwaysAllow, "Always Allow", "Always allow the tool call")
                                        .item(Permission::DenyOnce, "Deny", "Deny the tool call")
                                        .item(Permission::Cancel, "Cancel", "Cancel the AI response and tool call")
                                        .interact()
                                } else {
                                    // Security message present - don't show "Always Allow"
                                    cliclack::select(prompt)
                                        .item(Permission::AllowOnce, "Allow", "Allow the tool call once")
                                        .item(Permission::DenyOnce, "Deny", "Deny the tool call")
                                        .item(Permission::Cancel, "Cancel", "Cancel the AI response and tool call")
                                        .interact()
                                };

                                let permission = match permission_result {
                                    Ok(p) => p, // If Ok, use the selected permission
                                    Err(e) => {
                                        // Check if the error is an interruption (Ctrl+C/Cmd+C, Escape)
                                        if e.kind() == std::io::ErrorKind::Interrupted {
                                            Permission::Cancel // If interrupted, set permission to Cancel
                                        } else {
                                            return Err(e.into()); // Otherwise, convert and propagate the original error
                                        }
                                    }
                                };

                                if permission == Permission::Cancel {
                                    output::render_text("Tool call cancelled. Returning to chat...", Some(Color::Yellow), true);

                                    let mut response_message = Message::user();
                                    response_message.content.push(MessageContent::tool_response(
                                        confirmation.id.clone(),
                                        Err(ErrorData { code: ErrorCode::INVALID_REQUEST, message: std::borrow::Cow::from("Tool call cancelled by user".to_string()), data: None })
                                    ));
                                    self.messages.push(response_message);
                                    cancel_token_clone.cancel();
                                    drop(stream);
                                    break;
                                } else {
                                    self.agent.handle_confirmation(confirmation.id.clone(), PermissionConfirmation {
                                        principal_type: PrincipalType::Tool,
                                        permission,
                                    },).await;
                                }
                            }
                            else {
                                for content in &message.content {
                                    if let MessageContent::ToolRequest(tool_request) = content {
                                        if let Ok(tool_call) = &tool_request.tool_call {
                                            tracing::info!(counter.goose.tool_calls = 1,
                                                tool_name = %tool_call.name,
                                                "Tool call started"
                                            );
                                        }
                                    }
                                    if let MessageContent::ToolResponse(tool_response) = content {
                                        let tool_name = self.messages
                                            .iter()
                                            .rev()
                                            .find_map(|msg| {
                                                msg.content.iter().find_map(|c| {
                                                    if let MessageContent::ToolRequest(req) = c {
                                                        if req.id == tool_response.id {
                                                            if let Ok(tool_call) = &req.tool_call {
                                                                Some(tool_call.name.clone())
                                                            } else {
                                                                None
                                                            }
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                })
                                            })
                                            .unwrap_or_else(|| "unknown".to_string().into());

                                        let success = tool_response.tool_result.is_ok();
                                        let result_status = if success { "success" } else { "error" };
                                        tracing::info!(
                                            counter.goose.tool_completions = 1,
                                            tool_name = %tool_name,
                                            result = %result_status,
                                            "Tool call completed"
                                        );
                                    }
                                }

                                self.messages.push(message.clone());

                                if interactive {output::hide_thinking()};
                                let _ = progress_bars.hide();

                                // Don't render in JSON mode
                                if !is_json_mode {
                                    output::render_message(&message, self.debug);
                                }
                            }
                        }
                        Some(Ok(AgentEvent::McpNotification((_id, message)))) => {
                            match &message {
                                ServerNotification::LoggingMessageNotification(notification) => {
                                    let data = &notification.params.data;
                                    let (formatted_message, subagent_id, message_notification_type) = match data {
                                        Value::String(s) => (s.clone(), None, None),
                                        Value::Object(o) => {
                                            // Check for subagent notification structure first
                                            if let Some(Value::String(msg)) = o.get("message") {
                                                // Extract subagent info for better display
                                                let subagent_id = o.get("subagent_id")
                                                    .and_then(|v| v.as_str());
                                                let notification_type = o.get("type")
                                                    .and_then(|v| v.as_str());

                                                let formatted = match notification_type {
                                                    Some("subagent_created") | Some("completed") | Some("terminated") => {
                                                        format!("🤖 {}", msg)
                                                    }
                                                    Some("tool_usage") | Some("tool_completed") | Some("tool_error") => {
                                                        format!("🔧 {}", msg)
                                                    }
                                                    Some("message_processing") | Some("turn_progress") => {
                                                        format!("💭 {}", msg)
                                                    }
                                                    Some("response_generated") => {
                                                        // Check verbosity setting for subagent response content
                                                        let config = Config::global();
                                                        let min_priority = config
                                                            .get_param::<f32>("GOOSE_CLI_MIN_PRIORITY")
                                                            .ok()
                                                            .unwrap_or(0.5);

                                                        if min_priority > 0.1 && !self.debug {
                                                            // High/Medium verbosity: show truncated response
                                                            if let Some(response_content) = msg.strip_prefix("Responded: ") {
                                                                format!("🤖 Responded: {}", safe_truncate(response_content, 100))
                                                            } else {
                                                                format!("🤖 {}", msg)
                                                            }
                                                        } else {
                                                            // All verbosity or debug: show full response
                                                            format!("🤖 {}", msg)
                                                        }
                                                    }
                                                    _ => {
                                                        msg.to_string()
                                                    }
                                                };
                                                (formatted, subagent_id.map(str::to_string), notification_type.map(str::to_string))
                                            } else if let Some(Value::String(output)) = o.get("output") {
                                                // Fallback for other MCP notification types
                                                (output.to_owned(), None, None)
                                            } else if let Some(result) = format_task_execution_notification(data) {
                                                result
                                            } else {
                                                (data.to_string(), None, None)
                                            }
                                        },
                                        v => {
                                            (v.to_string(), None, None)
                                        },
                                    };

                                    // Handle subagent notifications - show immediately
                                    if let Some(_id) = subagent_id {
                                        // TODO: proper display for subagent notifications
                                        if interactive {
                                            let _ = progress_bars.hide();
                                            if !is_json_mode {
                                                println!("{}", console::style(&formatted_message).green().dim());
                                            }
                                        } else if !is_json_mode {
                                            progress_bars.log(&formatted_message);
                                        }
                                    } else if let Some(ref notification_type) = message_notification_type {
                                        if notification_type == TASK_EXECUTION_NOTIFICATION_TYPE {
                                            if interactive {
                                                let _ = progress_bars.hide();
                                                if !is_json_mode {
                                                    print!("{}", formatted_message);
                                                    std::io::stdout().flush().unwrap();
                                                }
                                            } else if !is_json_mode {
                                                print!("{}", formatted_message);
                                                std::io::stdout().flush().unwrap();
                                            }
                                        }
                                    }
                                    else if output::is_showing_thinking() {
                                        output::set_thinking_message(&formatted_message);
                                    } else {
                                        progress_bars.log(&formatted_message);
                                    }
                                },
                                ServerNotification::ProgressNotification(notification) => {
                                    let progress = notification.params.progress;
                                    let text = notification.params.message.as_deref();
                                    let total = notification.params.total;
                                    let token = &notification.params.progress_token;
                                    progress_bars.update(
                                        &token.0.to_string(),
                                        progress,
                                        total,
                                        text,
                                    );
                                },
                                _ => (),
                            }
                        }
                        Some(Ok(AgentEvent::HistoryReplaced(updated_conversation))) => {
                            self.messages = updated_conversation;
                        }
                        Some(Ok(AgentEvent::ModelChange { model, mode })) => {
                            // Log model change if in debug mode
                            if self.debug {
                                eprintln!("Model changed to {} in {} mode", model, mode);
                            }
                        }

                        Some(Err(e)) => {
                            // TODO(Douwe): Delete this
                            // Check if it's a ProviderError::ContextLengthExceeded
                            if e.downcast_ref::<goose::providers::errors::ProviderError>()
                                .map(|provider_error| matches!(provider_error, goose::providers::errors::ProviderError::ContextLengthExceeded(_)))
                                .unwrap_or(false) {

                                output::render_text(
                                    "Compaction requested. Should have happened in the agent!",
                                    Some(Color::Yellow),
                                    true
                                );
                                warn!("Compaction requested. Should have happened in the agent!");
                            }
                            eprintln!("Error: {}", e);
                            cancel_token_clone.cancel();
                            drop(stream);
                            if let Err(e) = self.handle_interrupted_messages(false).await {
                                eprintln!("Error handling interruption: {}", e);
                            } else {
                                output::render_error(
                                    "The error above was an exception we were not able to handle.\n\
                                    These errors are often related to connection or authentication\n\
                                    We've removed the conversation up to the most recent user message\n\
                                    - depending on the error you may be able to continue",
                                );
                            }
                            break;
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    cancel_token_clone.cancel();
                    drop(stream);
                    if let Err(e) = self.handle_interrupted_messages(true).await {
                        eprintln!("Error handling interruption: {}", e);
                    }
                    break;
                }
            }
        }

        // Output JSON if requested
        if is_json_mode {
            let metadata = match SessionManager::get_session(&self.session_id, false).await {
                Ok(session) => JsonMetadata {
                    total_tokens: session.total_tokens,
                    status: "completed".to_string(),
                },
                Err(_) => JsonMetadata {
                    total_tokens: None,
                    status: "completed".to_string(),
                },
            };

            let json_output = JsonOutput {
                messages: self.messages.messages().to_vec(),
                metadata,
            };

            println!("{}", serde_json::to_string_pretty(&json_output)?);
        } else {
            println!();
        }

        Ok(())
    }

    async fn handle_interrupted_messages(&mut self, interrupt: bool) -> Result<()> {
        // First, get any tool requests from the last message if it exists
        let tool_requests = self
            .messages
            .last()
            .filter(|msg| msg.role == rmcp::model::Role::Assistant)
            .map_or(Vec::new(), |msg| {
                msg.content
                    .iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(req) = content {
                            Some((req.id.clone(), req.tool_call.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            });

        if !tool_requests.is_empty() {
            // Interrupted during a tool request
            // Create tool responses for all interrupted tool requests
            let mut response_message = Message::user();
            let last_tool_name = tool_requests
                .last()
                .and_then(|(_, tool_call)| {
                    tool_call
                        .as_ref()
                        .ok()
                        .map(|tool| tool.name.to_string().clone())
                })
                .unwrap_or_else(|| "tool".to_string());

            let notification = if interrupt {
                "Interrupted by the user to make a correction".to_string()
            } else {
                "An uncaught error happened during tool use".to_string()
            };
            for (req_id, _) in &tool_requests {
                response_message.content.push(MessageContent::tool_response(
                    req_id.clone(),
                    Err(ErrorData {
                        code: ErrorCode::INTERNAL_ERROR,
                        message: std::borrow::Cow::from(notification.clone()),
                        data: None,
                    }),
                ));
            }
            // TODO(Douwe): update also db
            self.push_message(response_message);
            let prompt = format!(
                "The existing call to {} was interrupted. How would you like to proceed?",
                last_tool_name
            );
            self.push_message(Message::assistant().with_text(&prompt));
            output::render_message(&Message::assistant().with_text(&prompt), self.debug);
        } else {
            // An interruption occurred outside of a tool request-response.
            if let Some(last_msg) = self.messages.last() {
                if last_msg.role == rmcp::model::Role::User {
                    match last_msg.content.first() {
                        Some(MessageContent::ToolResponse(_)) => {
                            // Interruption occurred after a tool had completed but not assistant reply
                            let prompt = "The tool calling loop was interrupted. How would you like to proceed?";
                            self.push_message(Message::assistant().with_text(prompt));
                            output::render_message(
                                &Message::assistant().with_text(prompt),
                                self.debug,
                            );
                        }
                        Some(_) => {
                            // A real users message
                            self.messages.pop();
                            let prompt = "Interrupted before the model replied and removed the last message.";
                            output::render_message(
                                &Message::assistant().with_text(prompt),
                                self.debug,
                            );
                        }
                        None => panic!("No content in last message"),
                    }
                }
            }
        }
        Ok(())
    }

    /// Update the completion cache with fresh data
    /// This should be called before the interactive session starts
    pub async fn update_completion_cache(&mut self) -> Result<()> {
        // Get fresh data
        let prompts = self.agent.list_extension_prompts().await;

        // Update the cache with write lock
        let mut cache = self.completion_cache.write().unwrap();
        cache.prompts.clear();
        cache.prompt_info.clear();

        for (extension, prompt_list) in prompts {
            let names: Vec<String> = prompt_list.iter().map(|p| p.name.clone()).collect();
            cache.prompts.insert(extension.clone(), names);

            for prompt in prompt_list {
                cache.prompt_info.insert(
                    prompt.name.clone(),
                    output::PromptInfo {
                        name: prompt.name.clone(),
                        description: prompt.description.clone(),
                        arguments: prompt.arguments.clone(),
                        extension: Some(extension.clone()),
                    },
                );
            }
        }

        cache.last_updated = Instant::now();
        Ok(())
    }

    /// Invalidate the completion cache
    /// This should be called when extensions are added or removed
    async fn invalidate_completion_cache(&self) {
        let mut cache = self.completion_cache.write().unwrap();
        cache.prompts.clear();
        cache.prompt_info.clear();
        cache.last_updated = Instant::now();
    }

    pub fn message_history(&self) -> Conversation {
        self.messages.clone()
    }

    /// Render all past messages from the session history
    pub fn render_message_history(&self) {
        if self.messages.is_empty() {
            return;
        }

        // Print session restored message
        println!(
            "\n{} {} messages loaded into context.",
            console::style("Session restored:").green().bold(),
            console::style(self.messages.len()).green()
        );

        // Render each message
        for message in self.messages.iter() {
            output::render_message(message, self.debug);
        }

        // Add a visual separator after restored messages
        println!(
            "\n{}\n",
            console::style("──────── New Messages ────────").dim()
        );
    }

    pub async fn get_session(&self) -> Result<goose::session::Session> {
        SessionManager::get_session(&self.session_id, false).await
    }

    // Get the session's total token usage
    pub async fn get_total_token_usage(&self) -> Result<Option<i32>> {
        let metadata = self.get_session().await?;
        Ok(metadata.total_tokens)
    }

    /// Display enhanced context usage with session totals
    pub async fn display_context_usage(&self) -> Result<()> {
        let provider = self.agent.provider().await?;
        let model_config = provider.get_model_config();
        let context_limit = model_config.context_limit();

        let config = Config::global();
        let show_cost = config
            .get_param::<bool>("GOOSE_CLI_SHOW_COST")
            .unwrap_or(false);

        let provider_name = config
            .get_goose_provider()
            .unwrap_or_else(|_| "unknown".to_string());

        // Do not get costing information if show cost is disabled
        // This will prevent the API call to openrouter.ai
        // This is useful if for cases where openrouter.ai may be blocked by corporate firewalls
        if show_cost {
            // Initialize pricing cache on startup
            tracing::info!("Initializing pricing cache...");
            if let Err(e) = initialize_pricing_cache().await {
                tracing::warn!(
                    "Failed to initialize pricing cache: {e}. Pricing data may not be available."
                );
            }
        }

        match self.get_session().await {
            Ok(metadata) => {
                let total_tokens = metadata.total_tokens.unwrap_or(0) as usize;

                output::display_context_usage(total_tokens, context_limit);

                if show_cost {
                    let input_tokens = metadata.input_tokens.unwrap_or(0) as usize;
                    let output_tokens = metadata.output_tokens.unwrap_or(0) as usize;
                    output::display_cost_usage(
                        &provider_name,
                        &model_config.model_name,
                        input_tokens,
                        output_tokens,
                    )
                    .await;
                }
            }
            Err(_) => {
                output::display_context_usage(0, context_limit);
            }
        }

        Ok(())
    }

    /// Handle prompt command execution
    async fn handle_prompt_command(&mut self, opts: input::PromptCommandOptions) -> Result<()> {
        // name is required
        if opts.name.is_empty() {
            output::render_error("Prompt name argument is required");
            return Ok(());
        }

        if opts.info {
            match self.get_prompt_info(&opts.name).await? {
                Some(info) => output::render_prompt_info(&info),
                None => output::render_error(&format!("Prompt '{}' not found", opts.name)),
            }
        } else {
            // Convert the arguments HashMap to a Value
            let arguments = serde_json::to_value(opts.arguments)
                .map_err(|e| anyhow::anyhow!("Failed to serialize arguments: {}", e))?;

            match self.get_prompt(&opts.name, arguments).await {
                Ok(messages) => {
                    let start_len = self.messages.len();
                    let mut valid = true;
                    for (i, prompt_message) in messages.into_iter().enumerate() {
                        let msg = Message::from(prompt_message);
                        // ensure we get a User - Assistant - User type pattern
                        let expected_role = if i % 2 == 0 {
                            rmcp::model::Role::User
                        } else {
                            rmcp::model::Role::Assistant
                        };

                        if msg.role != expected_role {
                            output::render_error(&format!(
                                "Expected {:?} message at position {}, but found {:?}",
                                expected_role, i, msg.role
                            ));
                            valid = false;
                            // get rid of everything we added to messages
                            self.messages.truncate(start_len);
                            break;
                        }

                        if msg.role == rmcp::model::Role::User {
                            output::render_message(&msg, self.debug);
                        }
                        self.push_message(msg);
                    }

                    if valid {
                        output::show_thinking();
                        self.process_agent_response(true, CancellationToken::default())
                            .await?;
                        output::hide_thinking();
                    }
                }
                Err(e) => output::render_error(&e.to_string()),
            }
        }

        Ok(())
    }

    /// Save a recipe to a file
    ///
    /// # Arguments
    /// * `recipe` - The recipe to save
    /// * `filepath_str` - The path to save the recipe to
    ///
    /// # Returns
    /// * `Result<PathBuf, String>` - The path the recipe was saved to or an error message
    fn save_recipe(
        &self,
        recipe: &goose::recipe::Recipe,
        filepath_str: &str,
    ) -> anyhow::Result<PathBuf> {
        let path_buf = PathBuf::from(filepath_str);
        let mut path = path_buf.clone();

        // Update the final path if it's relative
        if path_buf.is_relative() {
            // If the path is relative, resolve it relative to the current working directory
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            path = cwd.join(&path_buf);
        }

        // Check if parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Directory '{}' does not exist",
                    parent.display()
                ));
            }
        }

        // Try creating the file
        let file = std::fs::File::create(path.as_path())
            .context(format!("Failed to create file '{}'", path.display()))?;

        // Write YAML
        serde_yaml::to_writer(file, recipe).context("Failed to save recipe")?;

        Ok(path)
    }

    fn push_message(&mut self, message: Message) {
        self.messages.push(message);
    }
}

async fn get_reasoner() -> Result<Arc<dyn Provider>, anyhow::Error> {
    use goose::model::ModelConfig;
    use goose::providers::create;

    let config = Config::global();

    // Try planner-specific provider first, fallback to default provider
    let provider = if let Ok(provider) = config.get_param::<String>("GOOSE_PLANNER_PROVIDER") {
        provider
    } else {
        println!("WARNING: GOOSE_PLANNER_PROVIDER not found. Using default provider...");
        config
            .get_goose_provider()
            .expect("No provider configured. Run 'goose configure' first")
    };

    // Try planner-specific model first, fallback to default model
    let model = if let Ok(model) = config.get_param::<String>("GOOSE_PLANNER_MODEL") {
        model
    } else {
        println!("WARNING: GOOSE_PLANNER_MODEL not found. Using default model...");
        config
            .get_goose_model()
            .expect("No model configured. Run 'goose configure' first")
    };

    let model_config =
        ModelConfig::new_with_context_env(model, Some("GOOSE_PLANNER_CONTEXT_LIMIT"))?;
    let reasoner = create(&provider, model_config).await?;

    Ok(reasoner)
}

/// Format elapsed time duration
/// Shows seconds if less than 60, otherwise shows minutes:seconds
fn format_elapsed_time(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    if total_secs < 60 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{}m {:02}s", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_elapsed_time_under_60_seconds() {
        // Test sub-second duration
        let duration = Duration::from_millis(500);
        assert_eq!(format_elapsed_time(duration), "0.50s");

        // Test exactly 1 second
        let duration = Duration::from_secs(1);
        assert_eq!(format_elapsed_time(duration), "1.00s");

        // Test 45.75 seconds
        let duration = Duration::from_millis(45750);
        assert_eq!(format_elapsed_time(duration), "45.75s");

        // Test 59.99 seconds
        let duration = Duration::from_millis(59990);
        assert_eq!(format_elapsed_time(duration), "59.99s");
    }

    #[test]
    fn test_format_elapsed_time_minutes() {
        // Test exactly 60 seconds (1 minute)
        let duration = Duration::from_secs(60);
        assert_eq!(format_elapsed_time(duration), "1m 00s");

        // Test 61 seconds (1 minute 1 second)
        let duration = Duration::from_secs(61);
        assert_eq!(format_elapsed_time(duration), "1m 01s");

        // Test 90 seconds (1 minute 30 seconds)
        let duration = Duration::from_secs(90);
        assert_eq!(format_elapsed_time(duration), "1m 30s");

        // Test 119 seconds (1 minute 59 seconds)
        let duration = Duration::from_secs(119);
        assert_eq!(format_elapsed_time(duration), "1m 59s");

        // Test 120 seconds (2 minutes)
        let duration = Duration::from_secs(120);
        assert_eq!(format_elapsed_time(duration), "2m 00s");

        // Test 605 seconds (10 minutes 5 seconds)
        let duration = Duration::from_secs(605);
        assert_eq!(format_elapsed_time(duration), "10m 05s");

        // Test 3661 seconds (61 minutes 1 second)
        let duration = Duration::from_secs(3661);
        assert_eq!(format_elapsed_time(duration), "61m 01s");
    }

    #[test]
    fn test_format_elapsed_time_edge_cases() {
        // Test zero duration
        let duration = Duration::from_secs(0);
        assert_eq!(format_elapsed_time(duration), "0.00s");

        // Test very small duration (1 millisecond)
        let duration = Duration::from_millis(1);
        assert_eq!(format_elapsed_time(duration), "0.00s");

        // Test fractional seconds are truncated for minute display
        // 60.5 seconds should still show as 1m 00s (not 1m 00.5s)
        let duration = Duration::from_millis(60500);
        assert_eq!(format_elapsed_time(duration), "1m 00s");
    }
}
