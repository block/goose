use crate::profile::profile::Profile;
use crate::profile::provider_helper::set_provider_config;
use crate::session::Session;

use goose::agent::Agent;
use goose::providers::factory;

use crate::prompt::cliclack::CliclackPrompt;
use rand::{distributions::Alphanumeric, Rng};

use crate::commands::expected_config::get_recommended_models;
use crate::profile::profile_handler::{load_profiles, PROFILE_DEFAULT_NAME};
use crate::profile::provider_helper::PROVIDER_OPEN_AI;

pub fn build_session<'a>(session: Option<String>, profile: Option<String>) -> Box<Session<'a>> {
    // TODO: Use session_name.
    let _session_name = session_name(session);

    let loaded_profile = load_profile(profile);

    // TODO: Reconsider fn name as we are just using the fn to ask the user if env vars are not set
    let provider_config =
        set_provider_config(&loaded_profile.provider, loaded_profile.processor.clone());

    // TODO: Odd to be prepping the provider rather than having that done in the agent?
    let provider = factory::get_provider(provider_config).unwrap();
    let agent = Box::new(Agent::new(provider));
    let prompt = Box::new(CliclackPrompt::new());

    Box::new(Session::new(agent, prompt))
}

fn session_name(session: Option<String>) -> String {
    match session {
        Some(name) => name,
        None => rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(4)
            .map(char::from)
            .collect(),
    }
}

fn load_profile(profile_name: Option<String>) -> Box<Profile> {
    let profiles = load_profiles().unwrap();
    let loaded_profile = if profiles.is_empty() {
        let recommended_models = get_recommended_models(PROVIDER_OPEN_AI);
        Box::new(Profile {
            provider: PROVIDER_OPEN_AI.to_string(),
            processor: recommended_models.processor.to_string(),
            accelerator: recommended_models.accelerator.to_string(),
            additional_systems: Vec::new(),
        })
    } else {
        match profile_name {
            Some(name) => match profiles.get(name.as_str()) {
                Some(profile) => Box::new(profile.clone()),
                None => panic!("Profile '{}' not found", name),
            },
            None => match profiles.get(PROFILE_DEFAULT_NAME) {
                Some(profile) => Box::new(profile.clone()),
                None => panic!(
                    "No '{}' profile found. Run configure to create a profile.",
                    PROFILE_DEFAULT_NAME.to_string()
                ),
            }, // Default to the first profile. TODO: Define a constant name for the default profile.
        }
    };
    loaded_profile
}
