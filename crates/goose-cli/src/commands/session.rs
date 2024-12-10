use console::style;
use goose::agent::Agent;
use goose::providers::factory;
use rand::{distributions::Alphanumeric, Rng};
use std::path::{Path, PathBuf};
use std::process;

use crate::profile::{get_provider_config, load_profiles, Profile};
use crate::prompt::cliclack::CliclackPrompt;
use crate::prompt::rustyline::RustylinePrompt;
use crate::prompt::Prompt;
use crate::session::{ensure_session_dir, Session};

pub fn build_session<'a>(
    session: Option<String>,
    profile: Option<String>,
    resume: bool,
) -> Box<Session<'a>> {
    let session_dir = ensure_session_dir().expect("Failed to create session directory");
    let session_file = session_path(session.clone(), &session_dir, session.is_none() && !resume);

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

    let provider_config =
        get_provider_config(&loaded_profile.provider, loaded_profile.model.clone());

    // TODO: Odd to be prepping the provider rather than having that done in the agent?
    let provider = factory::get_provider(provider_config).unwrap();
    let agent = Box::new(Agent::new(provider));
    let prompt = match std::env::var("GOOSE_INPUT") {
        Ok(val) => match val.as_str() {
            "cliclack" => Box::new(CliclackPrompt::new()) as Box<dyn Prompt>,
            "rustyline" => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
            _ => Box::new(RustylinePrompt::new()) as Box<dyn Prompt>,
        },
        Err(_) => Box::new(RustylinePrompt::new()),
    };

    println!(
        "{} {} {} {} {}",
        style("starting session |").dim(),
        style("provider:").dim(),
        style(loaded_profile.provider).cyan().dim(),
        style("model:").dim(),
        style(loaded_profile.model).cyan().dim(),
    );
    println!(
        "    {} {}",
        style("logging to").dim(),
        style(session_file.display()).dim().cyan(),
    );
    Box::new(Session::new(agent, prompt, session_file))
}

fn session_path(
    provided_session_name: Option<String>,
    session_dir: &Path,
    retry_on_conflict: bool,
) -> PathBuf {
    let session_name = provided_session_name.unwrap_or(random_session_name());
    let session_file = session_dir.join(format!("{}.json", session_name));

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[should_panic(expected = "Cannot resume session: file")]
    fn test_resume_nonexistent_session_panics() {
        let temp_dir = tempdir().unwrap();
        // Set session directory to our temp directory so we don't actually create it.
        std::env::set_var("GOOSE_SESSION_DIR", temp_dir.path());

        build_session(Some("nonexistent-session".to_string()), None, true);
    }
}
