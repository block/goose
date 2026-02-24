use crate::cli::StreamableHttpOptions;
use crate::goosed_client::GoosedClient;

use super::output;
use super::CliSession;
use console::style;
use goose::agents::Container;
use goose::agents::ExtensionError;
use goose::config::resolve_extensions_for_new_session;
use goose::config::{Config, ExtensionConfig};
use goose::recipe::Recipe;
use goose::session::session_manager::SessionType;
use goose::session::EnabledExtensionsState;
use goose::session::SessionManager;
use rustyline::EditMode;
use std::process;
use std::sync::Arc;

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
    streamable_http_extensions: &[StreamableHttpOptions],
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

    for (idx, opts) in streamable_http_extensions.iter().enumerate() {
        let config = CliSession::parse_streamable_http_extension(&opts.url, opts.timeout);
        let hint = truncate_with_ellipsis(&opts.url, EXTENSION_HINT_MAX_LEN);
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
    pub streamable_http_extensions: Vec<StreamableHttpOptions>,
    /// List of builtin extension commands to add
    pub builtins: Vec<String>,
    pub no_profile: bool,
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
    /// Docker container to run stdio extensions inside
    pub container: Option<Container>,

    /// Per-invocation override for orchestrator max concurrency.
    /// This is propagated to goosed via the GOOSE_ORCHESTRATOR_MAX_CONCURRENCY env var.
    pub orchestrator_max_concurrency: Option<usize>,
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
            no_profile: false,
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
            container: None,
            orchestrator_max_concurrency: None,
        }
    }
}

fn env_overrides_from_session_config(
    session_config: &SessionBuilderConfig,
) -> Vec<(String, String)> {
    let mut env = Vec::new();

    if let Some(max_concurrency) = session_config.orchestrator_max_concurrency {
        env.push((
            "GOOSE_ORCHESTRATOR_MAX_CONCURRENCY".to_string(),
            max_concurrency.to_string(),
        ));
    }

    env
}

/// Offers to help debug an extension failure by creating a minimal debugging session
struct ResolvedProviderConfig {
    provider_name: String,
    model_name: String,
}

fn resolve_provider_and_model(
    session_config: &SessionBuilderConfig,
    config: &Config,
    saved_provider: Option<String>,
    saved_model_config: Option<goose::model::ModelConfig>,
) -> ResolvedProviderConfig {
    let recipe_settings = session_config
        .recipe
        .as_ref()
        .and_then(|r| r.settings.as_ref());

    let provider_name = session_config
        .provider
        .clone()
        .or(saved_provider)
        .or_else(|| recipe_settings.and_then(|s| s.goose_provider.clone()))
        .or_else(|| config.get_goose_provider().ok())
        .expect("No provider configured. Run 'goose configure' first");

    let model_name = session_config
        .model
        .clone()
        .or_else(|| saved_model_config.as_ref().map(|mc| mc.model_name.clone()))
        .or_else(|| recipe_settings.and_then(|s| s.goose_model.clone()))
        .or_else(|| config.get_goose_model().ok())
        .expect("No model configured. Run 'goose configure' first");

    ResolvedProviderConfig {
        provider_name,
        model_name,
    }
}

async fn resolve_session_id(
    session_config: &SessionBuilderConfig,
    session_manager: &goose::session::session_manager::SessionManager,
) -> String {
    if session_config.no_session {
        let working_dir = std::env::current_dir().expect("Could not get working directory");
        let session = session_manager
            .create_session(working_dir, "CLI Session".to_string(), SessionType::Hidden)
            .await
            .expect("Could not create session");
        session.id
    } else if session_config.resume {
        if let Some(ref session_id) = session_config.session_id {
            match session_manager.get_session(session_id, false).await {
                Ok(_) => session_id.clone(),
                Err(_) => {
                    output::render_error(&format!(
                        "Cannot resume session {} - no such session exists",
                        style(session_id).cyan()
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
        session_config.session_id.clone().unwrap()
    }
}

async fn handle_resumed_session_workdir(
    session_manager: &SessionManager,
    session_id: &str,
    interactive: bool,
) {
    let session = session_manager
        .get_session(session_id, false)
        .await
        .unwrap_or_else(|e| {
            output::render_error(&format!("Failed to read session metadata: {}", e));
            process::exit(1);
        });

    let current_workdir = std::env::current_dir().expect("Failed to get current working directory");
    if current_workdir == session.working_dir {
        return;
    }

    if interactive {
        let change_workdir = cliclack::confirm(format!(
            "{} The original working directory of this session was set to {}. \
             Your current directory is {}. \
             Do you want to switch back to the original working directory?",
            style("WARNING:").yellow(),
            style(session.working_dir.display()).cyan(),
            style(current_workdir.display()).cyan(),
        ))
        .initial_value(true)
        .interact()
        .expect("Failed to get user input");

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
                "Warning: Working directory differs from session (current: {}, session: {}). \
                 Staying in current directory.",
                current_workdir.display(),
                session.working_dir.display()
            ))
            .yellow()
        );
    }
}

async fn collect_extension_configs(
    session_manager: &SessionManager,
    session_config: &SessionBuilderConfig,
    recipe: Option<&Recipe>,
    session_id: &str,
) -> Result<Vec<ExtensionConfig>, ExtensionError> {
    let configured_extensions: Vec<ExtensionConfig> = if session_config.resume {
        EnabledExtensionsState::for_session(session_manager, session_id, Config::global()).await
    } else if session_config.no_profile {
        Vec::new()
    } else {
        resolve_extensions_for_new_session(recipe.and_then(|r| r.extensions.as_deref()), None)
    };

    let cli_flag_extensions = parse_cli_flag_extensions(
        &session_config.extensions,
        &session_config.streamable_http_extensions,
        &session_config.builtins,
    );

    let mut all: Vec<ExtensionConfig> = configured_extensions;
    all.extend(cli_flag_extensions.into_iter().map(|(_, cfg)| cfg));

    Ok(all)
}

pub async fn build_session(session_config: SessionBuilderConfig) -> CliSession {
    goose::posthog::set_session_context("cli", session_config.resume);

    let config = Config::global();
    let working_dir = std::env::current_dir().unwrap_or_default();
    let working_dir_str = working_dir.to_string_lossy().to_string();

    let session_manager = Arc::new(SessionManager::instance());

    let env_overrides = env_overrides_from_session_config(&session_config);

    // Spawn (or reuse) goosed.
    // If we have per-invocation env overrides, we must spawn a fresh goosed and avoid reusing
    // a previously-discovered instance (it may have different settings).
    let goosed =
        match GoosedClient::spawn_or_discover_with_env(&working_dir_str, &env_overrides).await {
            Ok(client) => client,
            Err(e) => {
                output::render_error(&format!("Failed to start goosed: {}", e));
                std::process::exit(1);
            }
        };

    let resolved = resolve_provider_and_model(&session_config, config, None, None);

    let session_id = resolve_session_id(&session_config, &session_manager).await;

    if session_config.resume {
        handle_resumed_session_workdir(&session_manager, &session_id, session_config.interactive)
            .await;
    }

    let extensions = match collect_extension_configs(
        &session_manager,
        &session_config,
        session_config.recipe.as_ref(),
        &session_id,
    )
    .await
    {
        Ok(ext) => ext,
        Err(e) => {
            output::render_error(&format!("Failed to collect extensions: {}", e));
            std::process::exit(1);
        }
    };

    if session_config.resume {
        if let Err(e) = goosed.resume_agent(&session_id).await {
            output::render_error(&format!("Failed to resume agent: {}", e));
            std::process::exit(1);
        }
    } else {
        let ext_overrides = if extensions.is_empty() {
            None
        } else {
            Some(extensions)
        };

        if let Err(e) = goosed
            .start_agent(
                &working_dir_str,
                session_config.recipe.as_ref(),
                ext_overrides,
            )
            .await
        {
            output::render_error(&format!("Failed to start agent: {}", e));
            std::process::exit(1);
        }
    }

    let edit_mode = config
        .get_param::<String>("EDIT_MODE")
        .ok()
        .and_then(|edit_mode| match edit_mode.to_lowercase().as_str() {
            "emacs" => Some(EditMode::Emacs),
            "vi" => Some(EditMode::Vi),
            _ => None,
        });

    let debug_mode = session_config.debug || config.get_param("GOOSE_DEBUG").unwrap_or(false);

    let session = CliSession::new(
        goosed,
        session_id.clone(),
        debug_mode,
        edit_mode,
        session_config.output_format.clone(),
    )
    .await;

    if !session_config.quiet {
        output::display_session_info(
            session_config.resume,
            &resolved.provider_name,
            &resolved.model_name,
            &Some(session_id),
            None,
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
            streamable_http_extensions: vec![StreamableHttpOptions {
                url: "http://localhost:8080/mcp".to_string(),
                timeout: goose::config::DEFAULT_EXTENSION_TIMEOUT,
            }],
            builtins: vec!["developer".to_string()],
            no_profile: false,
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
            container: None,
            orchestrator_max_concurrency: None,
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
        assert!(!config.no_profile);
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

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("abc", 5), "abc");

        assert_eq!(truncate_with_ellipsis("abcde", 5), "abcde");

        assert_eq!(truncate_with_ellipsis("abcdef", 5), "abcde…");
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello…");

        assert_eq!(truncate_with_ellipsis("", 5), "");
    }
}
