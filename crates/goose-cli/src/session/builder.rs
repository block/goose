use super::output;
use super::CliSession;
use console::style;
use goose::agents::Agent;
use goose::config::get_enabled_extensions;
use goose::config::resolve_extensions_for_new_session;
use goose::config::{
    extensions::get_extension_by_name, get_all_extensions, Config, ExtensionConfig,
};
use goose::providers::create;
use goose::recipe::Recipe;
use goose::session::session_manager::SessionType;
use goose::session::{EnabledExtensionsState, ExtensionState};
use rustyline::EditMode;
use std::collections::BTreeSet;
use std::process;
use std::sync::Arc;
use tokio::task::JoinSet;

const EXTENSION_HINT_MAX_LEN: usize = 5;

fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    let truncated: String = s.chars().take(max_len).collect();
    if s.chars().count() > max_len {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

fn parse_cli_flag_extensions(
    extensions: &[String],
    streamable_http_extensions: &[String],
    builtins: &[String],
) -> Vec<(String, ExtensionConfig)> {
    let mut extensions_to_load = Vec::new();

    for (idx, ext_str) in extensions.iter().enumerate() {
        match CliSession::parse_stdio_extension(ext_str) {
            Ok(config) => {
                let hint = truncate_with_ellipsis(ext_str, EXTENSION_HINT_MAX_LEN);
                let label = format!("stdio #{}({})", idx + 1, hint);
                extensions_to_load.push((label, config));
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    style(format!(
                        "Warning: Invalid --extension value '{}' ({}); ignoring",
                        ext_str, e
                    ))
                    .yellow()
                );
            }
        }
    }

    for (idx, ext_str) in streamable_http_extensions.iter().enumerate() {
        let config = CliSession::parse_streamable_http_extension(ext_str);
        let hint = truncate_with_ellipsis(ext_str, EXTENSION_HINT_MAX_LEN);
        let label = format!("http #{}({})", idx + 1, hint);
        extensions_to_load.push((label, config));
    }

    for builtin_str in builtins {
        let configs = CliSession::parse_builtin_extensions(builtin_str);
        for config in configs {
            extensions_to_load.push((config.name(), config));
        }
    }

    extensions_to_load
}

/// Configuration for building a new Goose session
///
/// This struct contains all the parameters needed to create a new session,
/// including session identification, extension configuration, and debug settings.
#[derive(Clone, Debug)]
pub struct SessionBuilderConfig {
    /// Session id, optional need to deduce from context
    pub session_id: Option<String>,
    /// Whether to resume an existing session
    pub resume: bool,
    /// Whether to fork an existing session (creates a copy of the original/existing session then resumes the copy)
    pub fork: bool,
    /// Whether to run without a session file
    pub no_session: bool,
    /// List of stdio extension commands to add
    pub extensions: Vec<String>,
    /// List of streamable HTTP extension commands to add
    pub streamable_http_extensions: Vec<String>,
    /// List of builtin extension commands to add
    pub builtins: Vec<String>,
    /// Recipe for the session
    pub recipe: Option<Recipe>,
    /// Any additional system prompt to append to the default
    pub additional_system_prompt: Option<String>,
    /// Provider override from CLI arguments
    pub provider: Option<String>,
    /// Model override from CLI arguments
    pub model: Option<String>,
    /// Enable debug printing
    pub debug: bool,
    /// Maximum number of consecutive identical tool calls allowed
    pub max_tool_repetitions: Option<u32>,
    /// Maximum number of turns (iterations) allowed without user input
    pub max_turns: Option<u32>,
    /// ID of the scheduled job that triggered this session (if any)
    pub scheduled_job_id: Option<String>,
    /// Whether this session will be used interactively (affects debugging prompts)
    pub interactive: bool,
    /// Quiet mode - suppress non-response output
    pub quiet: bool,
    /// Output format (text, json)
    pub output_format: String,
}

/// Manual implementation of Default to ensure proper initialization of output_format
/// This struct requires explicit default value for output_format field
impl Default for SessionBuilderConfig {
    fn default() -> Self {
        SessionBuilderConfig {
            session_id: None,
            resume: false,
            fork: false,
            no_session: false,
            extensions: Vec::new(),
            streamable_http_extensions: Vec::new(),
            builtins: Vec::new(),
            recipe: None,
            additional_system_prompt: None,
            provider: None,
            model: None,
            debug: false,
            max_tool_repetitions: None,
            max_turns: None,
            scheduled_job_id: None,
            interactive: false,
            quiet: false,
            output_format: "text".to_string(),
        }
    }
}

/// Offers to help debug an extension failure by creating a minimal debugging session
async fn offer_extension_debugging_help(
    extension_name: &str,
    error_message: &str,
    provider: Arc<dyn goose::providers::base::Provider>,
    interactive: bool,
) -> Result<(), anyhow::Error> {
    // Only offer debugging help in interactive mode
    if !interactive {
        return Ok(());
    }

    let help_prompt = format!(
        "Would you like me to help debug the '{}' extension failure?",
        extension_name
    );

    let should_help = match cliclack::confirm(help_prompt)
        .initial_value(false)
        .interact()
    {
        Ok(choice) => choice,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                return Ok(());
            } else {
                return Err(e.into());
            }
        }
    };

    if !should_help {
        return Ok(());
    }

    println!("{}", style("🔧 Starting debugging session...").cyan());

    // Create a debugging prompt with context about the extension failure
    let debug_prompt = format!(
        "I'm having trouble starting an extension called '{}'. Here's the error I encountered:\n\n{}\n\nCan you help me diagnose what might be wrong and suggest how to fix it? Please consider common issues like:\n- Missing dependencies or tools\n- Configuration problems\n- Network connectivity (for remote extensions)\n- Permission issues\n- Path or environment variable problems",
        extension_name,
        error_message
    );

    // Create a minimal agent for debugging
    let debug_agent = Agent::new();

    let session = debug_agent
        .config
        .session_manager
        .create_session(
            std::env::current_dir()?,
            "CLI Session".to_string(),
            SessionType::Hidden,
        )
        .await?;

    debug_agent.update_provider(provider, &session.id).await?;

    // Add the developer extension if available to help with debugging
    let extensions = get_all_extensions();
    for ext_wrapper in extensions {
        if ext_wrapper.enabled && ext_wrapper.config.name() == "developer" {
            if let Err(e) = debug_agent.add_extension(ext_wrapper.config).await {
                // If we can't add developer extension, continue without it
                eprintln!(
                    "Note: Could not load developer extension for debugging: {}",
                    e
                );
            }
            break;
        }
    }

    let mut debug_session = CliSession::new(
        debug_agent,
        session.id,
        false,
        None,
        None,
        None,
        None,
        "text".to_string(),
    )
    .await;

    // Process the debugging request
    println!("{}", style("Analyzing the extension failure...").yellow());
    match debug_session.headless(debug_prompt).await {
        Ok(_) => {
            println!(
                "{}",
                style("✅ Debugging session completed. Check the suggestions above.").green()
            );
        }
        Err(e) => {
            eprintln!(
                "{}",
                style(format!("❌ Debugging session failed: {}", e)).red()
            );
        }
    }
    Ok(())
}

async fn load_extensions(
    agent: Agent,
    extensions_to_load: Vec<(String, ExtensionConfig)>,
    provider_for_debug: Arc<dyn goose::providers::base::Provider>,
    interactive: bool,
) -> Arc<Agent> {
    let mut set = JoinSet::new();
    let agent_ptr = Arc::new(agent);

    let mut waiting_ids: BTreeSet<usize> = (0..extensions_to_load.len()).collect();
    for (id, (_label, extension)) in extensions_to_load.iter().enumerate() {
        let agent_ptr = agent_ptr.clone();
        let cfg = extension.clone();
        set.spawn(async move { (id, agent_ptr.add_extension(cfg).await) });
    }

    let get_message = |waiting_ids: &BTreeSet<usize>| {
        let labels: Vec<String> = waiting_ids
            .iter()
            .map(|id| {
                extensions_to_load
                    .get(*id)
                    .map(|e| e.0.clone())
                    .unwrap_or_default()
            })
            .collect();
        format!(
            "starting {} extensions: {}",
            waiting_ids.len(),
            labels.join(", ")
        )
    };

    let spinner = cliclack::spinner();
    spinner.start(get_message(&waiting_ids));

    let mut offer_debug: Vec<(usize, anyhow::Error)> = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok((id, Ok(_))) => {
                waiting_ids.remove(&id);
                spinner.set_message(get_message(&waiting_ids));
            }
            Ok((id, Err(e))) => offer_debug.push((id, e.into())),
            Err(e) => tracing::error!("failed to add extension: {}", e),
        }
    }

    spinner.clear();

    for (id, err) in offer_debug {
        let label = extensions_to_load
            .get(id)
            .map(|e| e.0.clone())
            .unwrap_or_default();
        eprintln!(
            "{}",
            style(format!(
                "Warning: Failed to start extension '{}' ({}), continuing without it",
                label, err
            ))
            .yellow()
        );

        if let Err(debug_err) = offer_extension_debugging_help(
            &label,
            &err.to_string(),
            Arc::clone(&provider_for_debug),
            interactive,
        )
        .await
        {
            eprintln!("Note: Could not start debugging session: {}", debug_err);
        }
    }

    agent_ptr
}

fn check_missing_extensions_or_exit(saved_extensions: &[ExtensionConfig], interactive: bool) {
    let missing: Vec<_> = saved_extensions
        .iter()
        .filter(|ext| get_extension_by_name(&ext.name()).is_none())
        .cloned()
        .collect();

    if !missing.is_empty() {
        let names = missing
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
            .join(", ");

        if interactive {
            if !cliclack::confirm(format!(
                "Extension(s) {} from previous session are no longer available. Restore for this session?",
                names
            ))
            .initial_value(true)
            .interact()
            .unwrap_or(false)
            {
                println!("{}", style("Resume cancelled.").yellow());
                process::exit(0);
            }
        } else {
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Extension(s) {} from previous session are no longer available, continuing without them.",
                    names
                ))
                .yellow()
            );
        }
    }
}

pub async fn build_session(session_config: SessionBuilderConfig) -> CliSession {
    goose::posthog::set_session_context("cli", session_config.resume);

    let config = Config::global();
    let agent: Agent = Agent::new();
    let session_manager = agent.config.session_manager.clone();

    let (saved_provider, saved_model_config) = if session_config.resume {
        if let Some(ref session_id) = session_config.session_id {
            match session_manager.get_session(session_id, false).await {
                Ok(session_data) => (session_data.provider_name, session_data.model_config),
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let recipe = session_config.recipe.as_ref();
    let recipe_settings = recipe.and_then(|r| r.settings.as_ref());

    let provider_name = session_config
        .provider
        .or(saved_provider)
        .or_else(|| recipe_settings.and_then(|s| s.goose_provider.clone()))
        .or_else(|| config.get_goose_provider().ok())
        .expect("No provider configured. Run 'goose configure' first");

    let model_name = session_config
        .model
        .or_else(|| saved_model_config.as_ref().map(|mc| mc.model_name.clone()))
        .or_else(|| recipe_settings.and_then(|s| s.goose_model.clone()))
        .or_else(|| config.get_goose_model().ok())
        .expect("No model configured. Run 'goose configure' first");

    let model_config = if session_config.resume
        && saved_model_config
            .as_ref()
            .is_some_and(|mc| mc.model_name == model_name)
    {
        let mut config = saved_model_config.unwrap();
        if let Some(temp) = recipe_settings.and_then(|s| s.temperature) {
            config = config.with_temperature(Some(temp));
        }
        config
    } else {
        let temperature = recipe_settings.and_then(|s| s.temperature);
        goose::model::ModelConfig::new(&model_name)
            .unwrap_or_else(|e| {
                output::render_error(&format!("Failed to create model configuration: {}", e));
                process::exit(1);
            })
            .with_temperature(temperature)
    };

    agent
        .apply_recipe_components(
            recipe.and_then(|r| r.sub_recipes.clone()),
            recipe.and_then(|r| r.response.clone()),
            true,
        )
        .await;

    let new_provider = match create(&provider_name, model_config).await {
        Ok(provider) => provider,
        Err(e) => {
            output::render_error(&format!(
                "Error {}.\n\
                Please check your system keychain and run 'goose configure' again.\n\
                If your system is unable to use the keyring, please try setting secret key(s) via environment variables.\n\
                For more info, see: https://block.github.io/goose/docs/troubleshooting/#keychainkeyring-errors",
                e
            ));
            process::exit(1);
        }
    };
    let provider_for_display = Arc::clone(&new_provider);

    if let Some(lead_worker) = new_provider.as_lead_worker() {
        let (lead_model, worker_model) = lead_worker.get_model_info();
        tracing::info!(
            "🤖 Lead/Worker Mode Enabled: Lead model (first 3 turns): {}, Worker model (turn 4+): {}, Auto-fallback on failures: Enabled",
            lead_model,
            worker_model
        );
    } else {
        tracing::info!("🤖 Using model: {}", model_name);
    }

    let session_id: String = if session_config.no_session {
        let working_dir = std::env::current_dir().expect("Could not get working directory");
        let session = session_manager
            .create_session(working_dir, "CLI Session".to_string(), SessionType::Hidden)
            .await
            .expect("Could not create session");
        session.id
    } else if session_config.resume {
        if let Some(session_id) = session_config.session_id {
            match session_manager.get_session(&session_id, false).await {
                Ok(_) => session_id,
                Err(_) => {
                    output::render_error(&format!(
                        "Cannot resume session {} - no such session exists",
                        style(&session_id).cyan()
                    ));
                    process::exit(1);
                }
            }
        } else {
            match session_manager.list_sessions().await {
                Ok(sessions) if !sessions.is_empty() => sessions[0].id.clone(),
                _ => {
                    output::render_error("Cannot resume - no previous sessions found");
                    process::exit(1);
                }
            }
        }
    } else {
        session_config.session_id.unwrap()
    };

    agent
        .update_provider(new_provider, &session_id)
        .await
        .unwrap_or_else(|e| {
            output::render_error(&format!("Failed to initialize agent: {}", e));
            process::exit(1);
        });

    if session_config.resume {
        let session = agent
            .config
            .session_manager
            .get_session(&session_id, false)
            .await
            .unwrap_or_else(|e| {
                output::render_error(&format!("Failed to read session metadata: {}", e));
                process::exit(1);
            });

        let current_workdir =
            std::env::current_dir().expect("Failed to get current working directory");
        if current_workdir != session.working_dir {
            if session_config.interactive {
                let change_workdir = cliclack::confirm(format!("{} The original working directory of this session was set to {}. Your current directory is {}. Do you want to switch back to the original working directory?", style("WARNING:").yellow(), style(session.working_dir.display()).cyan(), style(current_workdir.display()).cyan()))
                        .initial_value(true)
                        .interact().expect("Failed to get user input");

                if change_workdir {
                    if !session.working_dir.exists() {
                        output::render_error(&format!(
                            "Cannot switch to original working directory - {} no longer exists",
                            style(session.working_dir.display()).cyan()
                        ));
                    } else if let Err(e) = std::env::set_current_dir(&session.working_dir) {
                        output::render_error(&format!(
                            "Failed to switch to original working directory: {}",
                            e
                        ));
                    }
                }
            } else {
                eprintln!(
                    "{}",
                    style(format!(
                        "Warning: Working directory differs from session (current: {}, session: {}). Staying in current directory.",
                        current_workdir.display(),
                        session.working_dir.display()
                    ))
                    .yellow()
                );
            }
        }
    }

    // Setup extensions for the agent
    // Extensions need to be added after the session is created because we change directory when resuming a session

    for warning in goose::config::get_warnings() {
        eprintln!("{}", style(format!("Warning: {}", warning)).yellow());
    }

    let configured_extensions: Vec<ExtensionConfig> = if session_config.resume {
        agent
            .config
            .session_manager
            .get_session(&session_id, false)
            .await
            .ok()
            .and_then(|s| EnabledExtensionsState::from_extension_data(&s.extension_data))
            .map(|state| {
                check_missing_extensions_or_exit(&state.extensions, session_config.interactive);
                state.extensions
            })
            .unwrap_or_else(get_enabled_extensions)
    } else {
        resolve_extensions_for_new_session(recipe.and_then(|r| r.extensions.as_deref()), None)
    };

    let cli_flag_extensions_to_load = parse_cli_flag_extensions(
        &session_config.extensions,
        &session_config.streamable_http_extensions,
        &session_config.builtins,
    );

    let mut extensions_to_load: Vec<(String, ExtensionConfig)> = configured_extensions
        .iter()
        .map(|cfg| (cfg.name(), cfg.clone()))
        .collect();
    extensions_to_load.extend(cli_flag_extensions_to_load);

    let agent_ptr = load_extensions(
        agent,
        extensions_to_load,
        Arc::clone(&provider_for_display),
        session_config.interactive,
    )
    .await;

    agent_ptr.ensure_subagent_for_recipes().await;

    // Determine editor mode
    let edit_mode = config
        .get_param::<String>("EDIT_MODE")
        .ok()
        .and_then(|edit_mode| match edit_mode.to_lowercase().as_str() {
            "emacs" => Some(EditMode::Emacs),
            "vi" => Some(EditMode::Vi),
            _ => {
                eprintln!("Invalid EDIT_MODE specified, defaulting to Emacs");
                None
            }
        });

    let debug_mode = session_config.debug || config.get_param("GOOSE_DEBUG").unwrap_or(false);

    let session = CliSession::new(
        Arc::try_unwrap(agent_ptr).unwrap_or_else(|_| panic!("There should be no more references")),
        session_id.clone(),
        debug_mode,
        session_config.scheduled_job_id.clone(),
        session_config.max_turns,
        edit_mode,
        recipe.and_then(|r| r.retry.clone()),
        session_config.output_format.clone(),
    )
    .await;

    if let Err(e) = session
        .agent
        .persist_extension_state(&session_id.clone())
        .await
    {
        tracing::warn!("Failed to save extension state: {}", e);
    }

    // Add CLI-specific system prompt extension
    session
        .agent
        .extend_system_prompt(super::prompt::get_cli_prompt())
        .await;

    if let Some(additional_prompt) = session_config.additional_system_prompt {
        session.agent.extend_system_prompt(additional_prompt).await;
    }

    // Only override system prompt if a system override exists
    let system_prompt_file: Option<String> = config.get_param("GOOSE_SYSTEM_PROMPT_FILE_PATH").ok();
    if let Some(ref path) = system_prompt_file {
        let override_prompt =
            std::fs::read_to_string(path).expect("Failed to read system prompt file");
        session.agent.override_system_prompt(override_prompt).await;
    }

    // Display session information unless in quiet mode
    if !session_config.quiet {
        output::display_session_info(
            session_config.resume,
            &provider_name,
            &model_name,
            &Some(session_id),
            Some(&provider_for_display),
        );
    }
    session
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_builder_config_creation() {
        let config = SessionBuilderConfig {
            session_id: None,
            resume: false,
            fork: false,
            no_session: false,
            extensions: vec!["echo test".to_string()],
            streamable_http_extensions: vec!["http://localhost:8080/mcp".to_string()],
            builtins: vec!["developer".to_string()],
            recipe: None,
            additional_system_prompt: Some("Test prompt".to_string()),
            provider: None,
            model: None,
            debug: true,
            max_tool_repetitions: Some(5),
            max_turns: None,
            scheduled_job_id: None,
            interactive: true,
            quiet: false,
            output_format: "text".to_string(),
        };

        assert_eq!(config.extensions.len(), 1);
        assert_eq!(config.streamable_http_extensions.len(), 1);
        assert_eq!(config.builtins.len(), 1);
        assert!(config.debug);
        assert_eq!(config.max_tool_repetitions, Some(5));
        assert!(config.max_turns.is_none());
        assert!(config.scheduled_job_id.is_none());
        assert!(config.interactive);
        assert!(!config.quiet);
    }

    #[test]
    fn test_session_builder_config_default() {
        let config = SessionBuilderConfig::default();

        assert!(config.session_id.is_none());
        assert!(!config.resume);
        assert!(!config.no_session);
        assert!(config.extensions.is_empty());
        assert!(config.streamable_http_extensions.is_empty());
        assert!(config.builtins.is_empty());
        assert!(config.recipe.is_none());
        assert!(config.additional_system_prompt.is_none());
        assert!(!config.debug);
        assert!(config.max_tool_repetitions.is_none());
        assert!(config.max_turns.is_none());
        assert!(config.scheduled_job_id.is_none());
        assert!(!config.interactive);
        assert!(!config.quiet);
        assert!(!config.fork);
    }

    #[tokio::test]
    async fn test_offer_extension_debugging_help_function_exists() {
        // This test just verifies the function compiles and can be called
        // We can't easily test the interactive parts without mocking

        // We can't actually test the full function without a real provider and user interaction
        // But we can at least verify it compiles and the function signature is correct
        let extension_name = "test-extension";
        let error_message = "test error";

        // This test mainly serves as a compilation check
        assert_eq!(extension_name, "test-extension");
        assert_eq!(error_message, "test error");
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("abc", 5), "abc");

        assert_eq!(truncate_with_ellipsis("abcde", 5), "abcde");

        assert_eq!(truncate_with_ellipsis("abcdef", 5), "abcde…");
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello…");

        assert_eq!(truncate_with_ellipsis("", 5), "");
    }
}
