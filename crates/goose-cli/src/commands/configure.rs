use crate::profile::{
    find_existing_profile, get_provider_config, profile_path, save_profile, Profile,
};
use cliclack::spinner;
use console::style;
use goose::key_manager::{get_keyring_secret, save_to_keyring, KeyRetrievalStrategy};
use goose::models::message::Message;
use goose::providers::factory;
use goose::providers::ollama::OLLAMA_MODEL;
use std::error::Error;

pub async fn handle_configure(
    provided_profile_name: Option<String>,
    provided_provider: Option<String>,
    provided_model: Option<String>,
) -> Result<(), Box<dyn Error>> {
    cliclack::intro(style(" configure-goose ").on_cyan().black())?;

    let profile_name = if let Some(name) = provided_profile_name {
        name
    } else {
        cliclack::input("Which profile should we configure?")
            .default_input("default")
            .interact()?
    };

    // Use default values from existing profile
    let existing_profile_result = find_existing_profile(&profile_name);
    let existing_profile = existing_profile_result.as_ref();

    if existing_profile.is_some() {
        let _ = cliclack::log::info(format!(
            "We are updating the existing profile for {}",
            profile_name
        ));
    }

    let provider_name = if let Some(provider) = provided_provider {
        provider
    } else {
        let default_provider =
            existing_profile.map_or("openai", |profile| profile.provider.as_str());
        cliclack::select("Which model provider should we use?")
            .initial_value(default_provider)
            .items(&[
                ("openai", "OpenAI", "GPT-4o etc"),
                ("databricks", "Databricks", "Models on AI Gateway"),
                ("ollama", "Ollama", "Local open source models"),
                ("anthropic", "Anthropic", "Claude models"),            
            ])
            .interact()?
            .to_string()
    };

    // Depending on the provider, we now want to look for any required keys and check or set them in the keychain
    for key in get_required_keys(&provider_name).iter() {
        // If the key is in the keyring, ask if we want to overwrite
        if get_keyring_secret(key, KeyRetrievalStrategy::KeyringOnly).is_ok() {
            let _ = cliclack::log::info(format!("{} is already available in the keyring", key));
            if cliclack::confirm("Would you like to overwrite this value?").interact()? {
                let value = cliclack::password(format!("Enter the value for {}", key))
                    .mask('▪')
                    .interact()?;

                save_to_keyring(key, &value)?;
            }
        }
        // If the key is in the env, ask if we want to save to keyring
        else if let Ok(value) = get_keyring_secret(key, KeyRetrievalStrategy::EnvironmentOnly) {
            let _ = cliclack::log::info(format!("Detected {} in env, we can use this from your environment.\nIt will need to continue to be set in future goose usage.", key));
            if cliclack::confirm("Would you like to save it to your kerying?").interact()? {
                save_to_keyring(key, &value)?;
            }
        }
        // We don't have a value, so we prompt for one
        else {
            let value = cliclack::password(format!(
                "Provider {} requires {}, please enter a value. (Will be saved to your keyring)",
                provider_name, key
            ))
            .mask('▪')
            .interact()?;

            save_to_keyring(key, &value)?;
        }
    }

    let model = if let Some(model) = provided_model {
        model
    } else {
        let recommended_model = get_recommended_model(&provider_name);
        let default_model_value =
            existing_profile.map_or(recommended_model, |profile| profile.model.as_str());
        cliclack::input("Enter a model from that provider:")
            .default_input(default_model_value)
            .interact()?
    };

    // Forward any existing systems from the profile if present
    let additional_systems =
        existing_profile.map_or(Vec::new(), |profile| profile.additional_systems.clone());

    if !additional_systems.is_empty() {
        let _ = cliclack::log::info(
            format!("We kept the existing systems from your {} profile. You can edit this with `goose system`", profile_name)
        );
    }

    let profile = Profile {
        provider: provider_name.to_string(),
        model: model.clone(),
        additional_systems,
    };

    // Confirm everything is configured correctly by calling a model!
    let provider_config = get_provider_config(&provider_name, model.clone());
    let spin = spinner();
    spin.start("Checking your configuration...");
    let provider = factory::get_provider(provider_config).unwrap();
    let message = Message::user().with_text("Please give a nice welcome messsage (one sentence) and let them know they are all set to use this agent");
    let result = provider.complete("You are an AI agent called Goose. You use tools of connected systems to solve problems.", &[message], &[]).await;

    match result {
        Ok((message, _usage)) => {
            if let Some(content) = message.content.first() {
                if let Some(text) = content.as_text() {
                    spin.stop(text);
                } else {
                    spin.stop("No response text available");
                }
            } else {
                spin.stop("No response content available");
            }

            let _ = match save_profile(&profile_name, profile) {
                Ok(()) => cliclack::outro(format!("Profile saved to: {:?}", profile_path()?)),
                Err(e) => cliclack::outro(format!("Failed to save profile: {}", e)),
            };
        }
        Err(e) => {
            println!("{:?}", e);
            spin.stop("We could not connect!");
            let _ = cliclack::outro("Try rerunning configure and check your credentials.");
        }
    }

    Ok(())
}

pub fn get_recommended_model(provider_name: &str) -> &str {
    match provider_name {
        "openai" => "gpt-4o",
        "databricks" => "claude-3-5-sonnet-2",
        "ollama" => OLLAMA_MODEL,
        "anthropic" => "claude-3-5-sonnet-2",
        _ => panic!("Invalid provider name"),
    }
}

pub fn get_required_keys(provider_name: &str) -> Vec<&'static str> {
    match provider_name {
        "openai" => vec!["OPENAI_API_KEY"],
        "databricks" => vec!["DATABRICKS_HOST"],
        "ollama" => vec!["OLLAMA_HOST"],
        "anthropic" => vec!["ANTHROPIC_API_KEY"],  // Removed ANTHROPIC_HOST since we use a fixed endpoint
        _ => panic!("Invalid provider name"),
    }
}