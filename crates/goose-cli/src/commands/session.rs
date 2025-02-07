use rand::{distributions::Alphanumeric, Rng};
use std::process;

use crate::prompt::rustyline::RustylinePrompt;
use crate::session::{ensure_session_dir, get_most_recent_session, legacy_session_dir, Session};
use console::style;
use goose::agents::extension::{Envs, ExtensionError};
use goose::agents::AgentFactory;
use goose::config::{Config, ExtensionConfig, ExtensionManager};
use goose::providers::create;
use std::path::Path;

use mcp_client::transport::Error as McpClientError;

pub async fn build_session(
    name: Option<String>,
    resume: bool,
    extensions: Vec<String>,
    builtins: Vec<String>,
) -> Session<'static> {
    // Load config and get provider/model
    let config = Config::global();

    let provider_name: String = config
        .get("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");
    let session_dir = ensure_session_dir().expect("Failed to create session directory");

    let model: String = config
        .get("GOOSE_MODEL")
        .expect("No model configured. Run 'goose configure' first");
    let model_config = goose::model::ModelConfig::new(model.clone());
    let provider = create(&provider_name, model_config).expect("Failed to create provider");

    // Create the agent
    let agent_version: Option<String> = config.get("GOOSE_AGENT").ok();
    let mut agent = match agent_version {
        Some(version) => AgentFactory::create(&version, provider),
        None => AgentFactory::create(AgentFactory::default_version(), provider),
    }
    .expect("Failed to create agent");

    // Setup extensions for the agent
    for extension in ExtensionManager::get_all().expect("should load extensions") {
        if extension.enabled {
            let config = extension.config.clone();
            agent
                .add_extension(config.clone())
                .await
                .unwrap_or_else(|e| {
                    let err = match e {
                        ExtensionError::Transport(McpClientError::StdioProcessError(inner)) => {
                            inner
                        }
                        _ => e.to_string(),
                    };
                    println!("Failed to start extension: {}, {:?}", config.name(), err);
                    println!(
                        "Please check extension configuration for {}.",
                        config.name()
                    );
                    process::exit(1);
                });
        }
    }

    // Add extensions if provided
    for extension_str in extensions {
        let mut parts: Vec<&str> = extension_str.split_whitespace().collect();
        let mut envs = std::collections::HashMap::new();

        // Parse environment variables (format: KEY=value)
        while let Some(part) = parts.first() {
            if !part.contains('=') {
                break;
            }
            let env_part = parts.remove(0);
            let (key, value) = env_part.split_once('=').unwrap();
            envs.insert(key.to_string(), value.to_string());
        }

        if parts.is_empty() {
            eprintln!("No command provided in extension string");
            process::exit(1);
        }

        let cmd = parts.remove(0).to_string();
        //this is an ephemeral extension so name does not matter
        let name = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let config = ExtensionConfig::Stdio {
            name,
            cmd,
            args: parts.iter().map(|s| s.to_string()).collect(),
            envs: Envs::new(envs),
        };

        agent.add_extension(config).await.unwrap_or_else(|e| {
            eprintln!("Failed to start extension: {}", e);
            process::exit(1);
        });
    }

    // Add builtin extensions
    for name in builtins {
        let config = ExtensionConfig::Builtin { name };
        agent.add_extension(config).await.unwrap_or_else(|e| {
            eprintln!("Failed to start builtin extension: {}", e);
            process::exit(1);
        });
    }

    // If resuming, try to find the session
    if resume {
        if let Some(ref session_name) = name {
            // Try to resume specific session
            let session_file = session_dir.join(format!("{}.jsonl", session_name));
            if session_file.exists() {
                let prompt = Box::new(RustylinePrompt::new());
                return Session::new(agent, prompt, session_file);
            }

            // LEGACY NOTE: remove this once old paths are no longer needed.
            if let Some(legacy_dir) = legacy_session_dir() {
                let legacy_file = legacy_dir.join(format!("{}.jsonl", session_name));
                if legacy_file.exists() {
                    let prompt = Box::new(RustylinePrompt::new());
                    return Session::new(agent, prompt, legacy_file);
                }
            }

            eprintln!("Session '{}' not found, starting new session", session_name);
        } else {
            // Try to resume most recent session
            if let Ok(session_file) = get_most_recent_session() {
                let prompt = Box::new(RustylinePrompt::new());
                return Session::new(agent, prompt, session_file);
            } else {
                eprintln!("No previous sessions found, starting new session");
            }
        }
    }

    // Generate session name if not provided
    let name = name.unwrap_or_else(|| {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect()
    });

    let session_file = session_dir.join(format!("{}.jsonl", name));
    if session_file.exists() {
        eprintln!("Session '{}' already exists", name);
        process::exit(1);
    }

    let prompt = Box::new(RustylinePrompt::new());

    display_session_info(resume, &provider_name, &model, &session_file);
    Session::new(agent, prompt, session_file)
}

fn display_session_info(resume: bool, provider: &str, model: &str, session_file: &Path) {
    let start_session_msg = if resume {
        "resuming session |"
    } else {
        "starting session |"
    };
    println!(
        "{} {} {} {} {}",
        style(start_session_msg).dim(),
        style("provider:").dim(),
        style(provider).cyan().dim(),
        style("model:").dim(),
        style(model).cyan().dim(),
    );
    println!(
        "    {} {}",
        style("logging to").dim(),
        style(session_file.display()).dim().cyan(),
    );
}
