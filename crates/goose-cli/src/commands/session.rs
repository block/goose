use console::style;
use goose::agents::AgentFactory;
use goose::agents::SystemConfig;
use goose::providers::factory;
use rand::{distributions::Alphanumeric, Rng};
use std::path::{Path, PathBuf};
use std::process;

use crate::profile::{get_provider_config, load_profiles, Profile};
use crate::prompt::rustyline::RustylinePrompt;
use crate::prompt::Prompt;
use crate::session::{ensure_session_dir, get_most_recent_session, Session};

pub async fn build_session<'a>(
    session: Option<String>,
    profile: Option<String>,
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

    let loaded_profile = load_profile(profile);

    let provider_config = get_provider_config(&loaded_profile.provider, (*loaded_profile).clone());

    let provider = factory::get_provider(provider_config).unwrap();

    let mut agent =
        AgentFactory::create(agent_version.as_deref().unwrap_or("default"), provider).unwrap();

    // We now add systems to the session based on configuration
    // TODO update the profile system tracking
    // TODO use systems from the profile
    // TODO once the client/server for MCP has stabilized, we should probably add InProcess transport to each
    //      and avoid spawning here. But it is at least included in the CLI for portability
    let config = SystemConfig::stdio("goose").with_args(vec!["mcp", "developer"]);
    agent
        .add_system(config)
        .await
        .expect("should start developer server");

    let prompt = match std::env::var("GOOSE_INPUT") {
        Ok(val) => match val.as_str() {
            "rustyline" => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
            _ => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
        },
        Err(_) => Box::new(RustylinePrompt::new()),
    };

    display_session_info(
        resume,
        loaded_profile.provider,
        loaded_profile.model,
        session_file.as_path(),
    );
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

fn load_profile(profile_name: Option<String>) -> Box<Profile> {
    let configure_profile_message = "Please create a profile first via goose configure.";
    let profiles = load_profiles().unwrap();
    let loaded_profile = if profiles.is_empty() {
        println!("No profiles found. {}", configure_profile_message);
        process::exit(1);
    } else {
        match profile_name {
            Some(name) => match profiles.get(name.as_str()) {
                Some(profile) => Box::new(profile.clone()),
                None => {
                    println!(
                        "Profile '{}' not found. {}",
                        name, configure_profile_message
                    );
                    process::exit(1);
                }
            },
            None => match profiles.get("default") {
                Some(profile) => Box::new(profile.clone()),
                None => {
                    println!("No 'default' profile found. {}", configure_profile_message);
                    process::exit(1);
                }
            },
        }
    };
    loaded_profile
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
