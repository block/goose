use rand::{distributions::Alphanumeric, Rng};
use std::process;

use crate::prompt::rustyline::RustylinePrompt;
use crate::session::{ensure_session_dir, get_most_recent_session, Session};
use goose::agents::extension::ExtensionError;
use goose::agents::AgentFactory;
use goose::config::{Config, ExtensionManager};
use goose::providers::create;

use mcp_client::transport::Error as McpClientError;

pub async fn build_session(name: Option<String>, resume: bool) -> Session<'static> {
    // Load config and get provider/model
    let config = Config::global();

    let provider_name: String = config
        .get("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");
    let session_dir = ensure_session_dir().expect("Failed to create session directory");

    let model = config
        .get("GOOSE_MODEL")
        .expect("No model configured. Run 'goose configure' first");
    let model_config = goose::model::ModelConfig::new(model);
    let provider = create(&provider_name, model_config).expect("Failed to create provider");

    // Create the agent
    let agent_version: Option<String> = config.get("GOOSE_AGENT").ok();
    let mut agent = match agent_version {
        Some(version) => AgentFactory::create(&version, provider),
        None => AgentFactory::create(AgentFactory::default_version(), provider),
    }
    .expect("Failed to create agent");

    // Setup extensions for the agent
    for (name, extension) in ExtensionManager::get_all().expect("should load extensions") {
        if extension.enabled {
            agent
                .add_extension(extension.config.clone())
                .await
                .unwrap_or_else(|e| {
                    let err = match e {
                        ExtensionError::Transport(McpClientError::StdioProcessError(inner)) => {
                            inner
                        }
                        _ => e.to_string(),
                    };
                    println!("Failed to start extension: {}, {:?}", name, err);
                    println!("Please check extension configuration for {}.", name);
                    process::exit(1);
                });
        }
    }

    // If resuming, try to find the session
    if resume {
        if let Some(ref session_name) = name {
            // Try to resume specific session
            let session_file = session_dir.join(format!("{}.jsonl", session_name));
            if session_file.exists() {
                let prompt = Box::new(RustylinePrompt::new());
                return Session::new(agent, prompt, session_file);
            } else {
                eprintln!("Session '{}' not found, starting new session", session_name);
            }
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
    Session::new(agent, prompt, session_file)
}
