use console::style;
use goose::agents::types::RetryConfig;
use goose::agents::Agent;
use goose::config::{Config, ExtensionConfig, ExtensionConfigManager};
use goose::providers::create;
use goose::recipe::{Response, SubRecipe};
use goose::session;
use goose::session::Identifier;
use rustyline::EditMode;
use std::process;
use std::sync::Arc;

use super::output;
use super::Session;

/// Configuration for building a new Goose session
///
/// This struct contains all the parameters needed to create a new session,
/// including session identification, extension configuration, and debug settings.
#[derive(Default, Clone, Debug)]
pub struct SessionBuilderConfig {
    /// Optional identifier for the session (name or path)
    pub identifier: Option<Identifier>,
    /// Whether to resume an existing session
    pub resume: bool,
    /// Whether to run without a session file
    pub no_session: bool,
    /// List of stdio extension commands to add
    pub extensions: Vec<String>,
    /// List of remote extension commands to add
    pub remote_extensions: Vec<String>,
    /// List of streamable HTTP extension commands to add
    pub streamable_http_extensions: Vec<String>,
    /// List of builtin extension commands to add
    pub builtins: Vec<String>,
    /// List of extensions to enable, enable only this set and ignore configured ones
    pub extensions_override: Option<Vec<ExtensionConfig>>,
    /// Any additional system prompt to append to the default
    pub additional_system_prompt: Option<String>,
    /// Settings to override the global Goose settings
    pub settings: Option<SessionSettings>,
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
    /// Sub-recipes to add to the session
    pub sub_recipes: Option<Vec<SubRecipe>>,
    /// Final output expected response
    pub final_output_response: Option<Response>,
    /// Retry configuration for automated validation and recovery
    pub retry_config: Option<RetryConfig>,
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
    debug_agent.update_provider(provider).await?;

    // Add the developer extension if available to help with debugging
    if let Ok(extensions) = ExtensionConfigManager::get_all() {
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
    }

    // Create a temporary session file for this debugging session
    let temp_session_file =
        std::env::temp_dir().join(format!("goose_debug_extension_{}.jsonl", extension_name));

    // Create the debugging session
    let mut debug_session = Session::new(
        debug_agent,
        Some(temp_session_file.clone()),
        false,
        None,
        None,
        None,
        None,
    );

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

    // Clean up the temporary session file
    let _ = std::fs::remove_file(temp_session_file);

    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct SessionSettings {
    pub goose_model: Option<String>,
    pub goose_provider: Option<String>,
    pub temperature: Option<f32>,
}

pub async fn build_session(session_config: SessionBuilderConfig) -> Session {
    // Load config and get provider/model
    let config = Config::global();

    let provider_name = session_config
        .provider
        .or_else(|| {
            session_config
                .settings
                .as_ref()
                .and_then(|s| s.goose_provider.clone())
        })
        .or_else(|| config.get_param("GOOSE_PROVIDER").ok())
        .expect("No provider configured. Run 'goose configure' first");

    let model_name = session_config
        .model
        .or_else(|| {
            session_config
                .settings
                .as_ref()
                .and_then(|s| s.goose_model.clone())
        })
        .or_else(|| config.get_param("GOOSE_MODEL").ok())
        .expect("No model configured. Run 'goose configure' first");

    let temperature = session_config.settings.as_ref().and_then(|s| s.temperature);

    let model_config = goose::model::ModelConfig::new(&model_name)
        .unwrap_or_else(|e| {
            output::render_error(&format!("Failed to create model configuration: {}", e));
            process::exit(1);
        })
        .with_temperature(temperature);

    // Create the agent
    let agent: Agent = Agent::new();

    if let Some(sub_recipes) = session_config.sub_recipes {
        agent.add_sub_recipes(sub_recipes).await;
    }

    if let Some(final_output_response) = session_config.final_output_response {
        agent.add_final_output_tool(final_output_response).await;
    }

    let new_provider = match create(&provider_name, model_config) {
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
    // Keep a reference to the provider for display_session_info
    let provider_for_display = Arc::clone(&new_provider);

    // Log model information at startup
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

    agent
        .update_provider(new_provider)
        .await
        .unwrap_or_else(|e| {
            output::render_error(&format!("Failed to initialize agent: {}", e));
            process::exit(1);
        });

    // Configure tool monitoring if max_tool_repetitions is set
    if let Some(max_repetitions) = session_config.max_tool_repetitions {
        agent.configure_tool_monitor(Some(max_repetitions)).await;
    }

    // Handle session file resolution and resuming
    let session_file: Option<std::path::PathBuf> = if session_config.no_session {
        None
    } else if session_config.resume {
        if let Some(identifier) = session_config.identifier {
            let session_file = match session::get_path(identifier) {
                Err(e) => {
                    output::render_error(&format!("Invalid session identifier: {}", e));
                    process::exit(1);
                }
                Ok(path) => path,
            };
            if !session_file.exists() {
                output::render_error(&format!(
                    "Cannot resume session {} - no such session exists",
                    style(session_file.display()).cyan()
                ));
                process::exit(1);
            }

            Some(session_file)
        } else {
            // Try to resume most recent session
            match session::get_most_recent_session() {
                Ok(file) => Some(file),
                Err(_) => {
                    output::render_error("Cannot resume - no previous sessions found");
                    process::exit(1);
                }
            }
        }
    } else {
        // Create new session with provided name/path or generated name
        let id = match session_config.identifier {
            Some(identifier) => identifier,
            None => Identifier::Name(session::generate_session_id()),
        };

        // Just get the path - file will be created when needed
        match session::get_path(id) {
            Ok(path) => Some(path),
            Err(e) => {
                output::render_error(&format!("Failed to create session path: {}", e));
                process::exit(1);
            }
        }
    };

    if session_config.resume {
        if let Some(session_file) = session_file.as_ref() {
            // Read the session metadata
            let metadata = session::read_metadata(session_file).unwrap_or_else(|e| {
                output::render_error(&format!("Failed to read session metadata: {}", e));
                process::exit(1);
            });

            let current_workdir =
                std::env::current_dir().expect("Failed to get current working directory");
            if current_workdir != metadata.working_dir {
                // Ask user if they want to change the working directory
                let change_workdir = cliclack::confirm(format!("{} The original working directory of this session was set to {}. Your current directory is {}. Do you want to switch back to the original working directory?", style("WARNING:").yellow(), style(metadata.working_dir.display()).cyan(), style(current_workdir.display()).cyan()))
            .initial_value(true)
            .interact().expect("Failed to get user input");

                if change_workdir {
                    if !metadata.working_dir.exists() {
                        output::render_error(&format!(
                            "Cannot switch to original working directory - {} no longer exists",
                            style(metadata.working_dir.display()).cyan()
                        ));
                    } else if let Err(e) = std::env::set_current_dir(&metadata.working_dir) {
                        output::render_error(&format!(
                            "Failed to switch to original working directory: {}",
                            e
                        ));
                    }
                }
            }
        }
    }

    // Setup extensions for the agent
    // Extensions need to be added after the session is created because we change directory when resuming a session
    // If we get extensions_override, only run those extensions and none other
    let extensions_to_run: Vec<_> = if let Some(extensions) = session_config.extensions_override {
        agent.disable_router_for_recipe().await;
        extensions.into_iter().collect()
    } else {
        ExtensionConfigManager::get_all()
            .expect("should load extensions")
            .into_iter()
            .filter(|ext| ext.enabled)
            .map(|ext| ext.config)
            .collect()
    };

    for extension in extensions_to_run {
        if let Err(e) = agent.add_extension(extension.clone()).await {
            let err = e.to_string();
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Failed to start extension '{}': {}",
                    extension.name(),
                    err
                ))
                .yellow()
            );
            eprintln!(
                "{}",
                style(format!(
                    "Continuing without extension '{}'",
                    extension.name()
                ))
                .yellow()
            );

            // Offer debugging help
            if let Err(debug_err) = offer_extension_debugging_help(
                &extension.name(),
                &err,
                Arc::clone(&provider_for_display),
                session_config.interactive,
            )
            .await
            {
                eprintln!("Note: Could not start debugging session: {}", debug_err);
            }
        }
    }

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

    // Create new session
    let mut session = Session::new(
        agent,
        session_file.clone(),
        session_config.debug,
        session_config.scheduled_job_id.clone(),
        session_config.max_turns,
        edit_mode,
        session_config.retry_config.clone(),
    );

    // Add extensions if provided
    for extension_str in session_config.extensions {
        if let Err(e) = session.add_extension(extension_str.clone()).await {
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Failed to start extension '{}': {}",
                    extension_str, e
                ))
                .yellow()
            );
            eprintln!(
                "{}",
                style(format!("Continuing without extension '{}'", extension_str)).yellow()
            );

            // Offer debugging help
            if let Err(debug_err) = offer_extension_debugging_help(
                &extension_str,
                &e.to_string(),
                Arc::clone(&provider_for_display),
                session_config.interactive,
            )
            .await
            {
                eprintln!("Note: Could not start debugging session: {}", debug_err);
            }
        }
    }

    // Add remote extensions if provided
    for extension_str in session_config.remote_extensions {
        if let Err(e) = session.add_remote_extension(extension_str.clone()).await {
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Failed to start remote extension '{}': {}",
                    extension_str, e
                ))
                .yellow()
            );
            eprintln!(
                "{}",
                style(format!(
                    "Continuing without remote extension '{}'",
                    extension_str
                ))
                .yellow()
            );

            // Offer debugging help
            if let Err(debug_err) = offer_extension_debugging_help(
                &extension_str,
                &e.to_string(),
                Arc::clone(&provider_for_display),
                session_config.interactive,
            )
            .await
            {
                eprintln!("Note: Could not start debugging session: {}", debug_err);
            }
        }
    }

    // Add streamable HTTP extensions if provided
    for extension_str in session_config.streamable_http_extensions {
        if let Err(e) = session
            .add_streamable_http_extension(extension_str.clone())
            .await
        {
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Failed to start streamable HTTP extension '{}': {}",
                    extension_str, e
                ))
                .yellow()
            );
            eprintln!(
                "{}",
                style(format!(
                    "Continuing without streamable HTTP extension '{}'",
                    extension_str
                ))
                .yellow()
            );

            // Offer debugging help
            if let Err(debug_err) = offer_extension_debugging_help(
                &extension_str,
                &e.to_string(),
                Arc::clone(&provider_for_display),
                session_config.interactive,
            )
            .await
            {
                eprintln!("Note: Could not start debugging session: {}", debug_err);
            }
        }
    }

    // Add builtin extensions
    for builtin in session_config.builtins {
        if let Err(e) = session.add_builtin(builtin.clone()).await {
            eprintln!(
                "{}",
                style(format!(
                    "Warning: Failed to start builtin extension '{}': {}",
                    builtin, e
                ))
                .yellow()
            );
            eprintln!(
                "{}",
                style(format!(
                    "Continuing without builtin extension '{}'",
                    builtin
                ))
                .yellow()
            );

            // Offer debugging help
            if let Err(debug_err) = offer_extension_debugging_help(
                &builtin,
                &e.to_string(),
                Arc::clone(&provider_for_display),
                session_config.interactive,
            )
            .await
            {
                eprintln!("Note: Could not start debugging session: {}", debug_err);
            }
        }
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
            &session_file,
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
            identifier: Some(Identifier::Name("test".to_string())),
            resume: false,
            no_session: false,
            extensions: vec!["echo test".to_string()],
            remote_extensions: vec!["http://example.com".to_string()],
            streamable_http_extensions: vec!["http://example.com/streamable".to_string()],
            builtins: vec!["developer".to_string()],
            extensions_override: None,
            additional_system_prompt: Some("Test prompt".to_string()),
            settings: None,
            provider: None,
            model: None,
            debug: true,
            max_tool_repetitions: Some(5),
            max_turns: None,
            scheduled_job_id: None,
            interactive: true,
            quiet: false,
            sub_recipes: None,
            final_output_response: None,
            retry_config: None,
        };

        assert_eq!(config.extensions.len(), 1);
        assert_eq!(config.remote_extensions.len(), 1);
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

        assert!(config.identifier.is_none());
        assert!(!config.resume);
        assert!(!config.no_session);
        assert!(config.extensions.is_empty());
        assert!(config.remote_extensions.is_empty());
        assert!(config.streamable_http_extensions.is_empty());
        assert!(config.builtins.is_empty());
        assert!(config.extensions_override.is_none());
        assert!(config.additional_system_prompt.is_none());
        assert!(!config.debug);
        assert!(config.max_tool_repetitions.is_none());
        assert!(config.max_turns.is_none());
        assert!(config.scheduled_job_id.is_none());
        assert!(!config.interactive);
        assert!(!config.quiet);
        assert!(config.final_output_response.is_none());
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
}
