use console::style;
use goose::agents::extension::ExtensionError;
use goose::agents::Agent;
use goose::config::{Config, ExtensionConfig, ExtensionConfigManager};
use goose::session;
use goose::session::Identifier;
use mcp_client::transport::Error as McpClientError;
use std::process;

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
    /// List of stdio extension commands to add
    pub extensions: Vec<String>,
    /// List of remote extension commands to add
    pub remote_extensions: Vec<String>,
    /// List of builtin extension commands to add
    pub builtins: Vec<String>,
    /// List of extensions to enable, enable only this set and ignore configured ones
    pub extensions_override: Option<Vec<ExtensionConfig>>,
    /// Any additional system prompt to append to the default
    pub additional_system_prompt: Option<String>,
    /// Enable debug printing
    pub debug: bool,
}

pub async fn build_session(session_config: SessionBuilderConfig) -> Session {
    // Load config and get provider/model
    let config = Config::global();

    let provider_name: String = config
        .get_param("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");

    let model: String = config
        .get_param("GOOSE_MODEL")
        .expect("No model configured. Run 'goose configure' first");
    let model_config = goose::model::ModelConfig::new(model.clone());
    let provider =
        goose::providers::create(&provider_name, model_config).expect("Failed to create provider");

    // Create the agent
    let mut agent = Agent::new(provider);

    // Handle session file resolution and resuming
    let session_file = if session_config.resume {
        if let Some(identifier) = session_config.identifier {
            let session_file = session::get_path(identifier);
            if !session_file.exists() {
                output::render_error(&format!(
                    "Cannot resume session {} - no such session exists",
                    style(session_file.display()).cyan()
                ));
                process::exit(1);
            }

            session_file
        } else {
            // Try to resume most recent session
            match session::get_most_recent_session() {
                Ok(file) => file,
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
        session::get_path(id)
    };

    if session_config.resume {
        // Read the session metadata
        let metadata = session::read_metadata(&session_file).unwrap_or_else(|e| {
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
                std::env::set_current_dir(metadata.working_dir).unwrap();
            }
        }
    }

    // Setup extensions for the agent
    // Extensions need to be added after the session is created because we change directory when resuming a session
    // If we get extensions_override, only run those extensions and none other
    let extensions_to_run: Vec<_> = if let Some(extensions) = session_config.extensions_override {
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
            let err = match e {
                ExtensionError::Transport(McpClientError::StdioProcessError(inner)) => inner,
                _ => e.to_string(),
            };
            eprintln!("Failed to start extension: {}, {:?}", extension.name(), err);
            eprintln!(
                "Please check extension configuration for {}.",
                extension.name()
            );
            process::exit(1);
        }
    }

    // Create new session
    let mut session = Session::new(agent, session_file.clone(), session_config.debug);

    // Add extensions if provided
    for extension_str in session_config.extensions {
        if let Err(e) = session.add_extension(extension_str).await {
            eprintln!("Failed to start extension: {}", e);
            process::exit(1);
        }
    }

    // Add remote extensions if provided
    for extension_str in session_config.remote_extensions {
        if let Err(e) = session.add_remote_extension(extension_str).await {
            eprintln!("Failed to start extension: {}", e);
            process::exit(1);
        }
    }

    // Add builtin extensions
    for builtin in session_config.builtins {
        if let Err(e) = session.add_builtin(builtin).await {
            eprintln!("Failed to start builtin extension: {}", e);
            process::exit(1);
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

    output::display_session_info(session_config.resume, &provider_name, &model, &session_file);
    session
}
