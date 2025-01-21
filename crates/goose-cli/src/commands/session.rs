use console::style;
use goose::agents::AgentFactory;
use goose::providers::factory;
use rand::{distributions::Alphanumeric, Rng};
use std::path::{Path, PathBuf};
use std::process;

use crate::config::Config;
use crate::prompt::rustyline::RustylinePrompt;
use crate::prompt::Prompt;
use crate::session::{ensure_session_dir, get_most_recent_session, Session};

/// Get the provider and model to use, following priority:
/// 1. CLI arguments
/// 2. Environment variables
/// 3. Config file
fn get_provider_and_model(
    cli_provider: Option<String>,
    cli_model: Option<String>,
    config: &Config,
) -> (String, String) {
    let provider = cli_provider
        .or_else(|| std::env::var("GOOSE_PROVIDER").ok())
        .unwrap_or_else(|| config.default_provider.clone());

    let model = cli_model
        .or_else(|| std::env::var("GOOSE_MODEL").ok())
        .unwrap_or_else(|| config.default_model.clone());

    (provider, model)
}

pub async fn build_session<'a>(
    session: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    agent_version: Option<String>,
    resume: bool,
) -> Box<Session<'a>> {
    let session_dir = ensure_session_dir().expect("Failed to create session directory");
    let session_file = if resume && session.is_none() {
        // When resuming without a specific session name, use the most recent session
        get_most_recent_session().expect("Failed to get most recent session")
    } else {
        session_path(session.clone(), &session_dir, session.is_none() && !resume)
    };

    // Guard against resuming a non-existent session
    if resume && !session_file.exists() {
        panic!(
            "Cannot resume session: file {} does not exist",
            session_file.display()
        );
    }

    // Guard against running a new session with a file that already exists
    if !resume && session_file.exists() {
        panic!(
            "Session file {} already exists. Use --resume to continue an existing session",
            session_file.display()
        );
    }

    let config_path = Config::config_path().expect("should identify default config path");

    if !config_path.exists() {
        println!("No configuration found. Please run 'goose configure' first.");
        process::exit(1);
    }

    let config = Config::load().unwrap_or_else(|_| {
        println!("The loaded configuration from {} was invalid", config_path.display());
        println!(" please edit the file to make it valid or consider deleting and recreating it via `goose configure`");
        process::exit(1);
    });

    let (provider_name, model_name) = get_provider_and_model(provider, model, &config);
    let provider = factory::get_provider(&provider_name).unwrap();

    let mut agent = AgentFactory::create(
        agent_version
            .as_deref()
            .unwrap_or(AgentFactory::default_version()),
        provider,
    )
    .unwrap();

    // Add configured systems
    for (name, _) in config.systems.iter() {
        if let Some(system_config) = config.get_system_config(name) {
            agent
                .add_system(system_config.clone())
                .await
                .unwrap_or_else(|_| panic!("Failed to start system: {}", name));
        }
    }

    let prompt = match std::env::var("GOOSE_INPUT") {
        Ok(val) => match val.as_str() {
            "rustyline" => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
            _ => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
        },
        Err(_) => Box::new(RustylinePrompt::new()),
    };

    display_session_info(resume, provider_name, model_name, session_file.as_path());
    Box::new(Session::new(agent, prompt, session_file))
}

fn session_path(
    provided_session_name: Option<String>,
    session_dir: &Path,
    retry_on_conflict: bool,
) -> PathBuf {
    let session_name = provided_session_name.unwrap_or(random_session_name());
    let session_file = session_dir.join(format!("{}.jsonl", session_name));

    if session_file.exists() && retry_on_conflict {
        generate_new_session_path(session_dir)
    } else {
        session_file
    }
}

fn random_session_name() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

// For auto-generated names, try up to 5 times to get a unique name
fn generate_new_session_path(session_dir: &Path) -> PathBuf {
    let mut attempts = 0;
    let max_attempts = 5;

    loop {
        let generated_name = random_session_name();
        let generated_file = session_dir.join(format!("{}.jsonl", generated_name));

        if !generated_file.exists() {
            break generated_file;
        }

        attempts += 1;
        if attempts >= max_attempts {
            panic!(
                "Failed to generate unique session name after {} attempts",
                max_attempts
            );
        }
    }
}

fn display_session_info(resume: bool, provider: String, model: String, session_file: &Path) {
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
