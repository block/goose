use cliclack::spinner;
use console::style;
use goose::agents::{system::Envs, SystemConfig};
use goose::key_manager::{get_keyring_secret, save_to_keyring, KeyRetrievalStrategy};
use goose::message::Message;
use goose::providers::anthropic::ANTHROPIC_DEFAULT_MODEL;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;
use goose::providers::factory;
use goose::providers::google::GOOGLE_DEFAULT_MODEL;
use goose::providers::groq::GROQ_DEFAULT_MODEL;
use goose::providers::ollama::OLLAMA_MODEL;
use goose::providers::openai::OPEN_AI_DEFAULT_MODEL;
use std::collections::HashMap;
use std::error::Error;

use crate::config::{Config, SystemEntry};

pub async fn handle_configure(
    provided_provider: Option<String>,
    provided_model: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Load existing config or create new one
    let config_exists = Config::config_path()?.exists();

    if !config_exists {
        // First time setup flow
        println!();
        println!(
            "{}",
            style("Welcome to goose! Let's get you set up with a provider.").dim()
        );
        println!(
            "{}",
            style("  you can rerun this command later to update your configuration").dim()
        );
        println!();
        cliclack::intro(style(" goose-configure ").on_cyan().black())?;
        configure_provider_dialog(provided_provider, provided_model).await?;
        println!(
            "\n  {}: Run '{}' again to adjust your config or add systems",
            style("Tip").green().italic(),
            style("goose configure").cyan()
        );
        Ok(())
    } else {
        println!();
        println!(
            "{}",
            style("This will update your existing config file").dim()
        );
        println!(
            "{} {}",
            style("  if you prefer, you can edit it directly at").dim(),
            Config::config_path()?.display()
        );
        println!();

        cliclack::intro(style(" goose-configure ").on_cyan().black())?;
        let action = cliclack::select("What would you like to configure?")
            .item(
                "providers",
                "Configure Providers",
                "Change provider or update credentials",
            )
            .item(
                "toggle",
                "Toggle Systems",
                "Enable or disable connected systems",
            )
            .item("add", "Add System", "Connect to a new system")
            .interact()?;

        match action {
            "toggle" => toggle_systems_dialog(),
            "add" => configure_systems_dialog(),
            "providers" => configure_provider_dialog(provided_provider, provided_model).await,
            _ => unreachable!(),
        }
    }
}

/// Dialog for configuring the AI provider and model
pub async fn configure_provider_dialog(
    provided_provider: Option<String>,
    provided_model: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Load existing config or create new one
    let mut config = Config::load().unwrap_or_default();

    // TODO offload to separate provider
    let provider_name = if let Some(provider) = provided_provider {
        provider
    } else {
        let providers = [
            "openai".to_string(),
            "databricks".to_string(),
            "ollama".to_string(),
            "anthropic".to_string(),
            "google".to_string(),
            "groq".to_string(),
        ];
        let provider = cliclack::select("Which model provider should we use?")
            .initial_value(&config.default_provider)
            .items(&[
                (&providers[0], "OpenAI", "GPT-4 etc"),
                (&providers[1], "Databricks", "Models on AI Gateway"),
                (&providers[2], "Ollama", "Local open source models"),
                (&providers[3], "Anthropic", "Claude models"),
                (&providers[4], "Google Gemini", "Gemini models"),
                (&providers[5], "Groq", "AI models"),
            ])
            .interact()?;
        provider.to_string()
    };

    // Configure provider keys
    for key in get_required_keys(&provider_name).iter() {
        // If the key is in the keyring, ask if we want to overwrite
        if get_keyring_secret(key, KeyRetrievalStrategy::KeyringOnly).is_ok() {
            cliclack::log::info(format!("{} is already available in the keyring", key))?;
            if cliclack::confirm("Would you like to overwrite this value?").interact()? {
                let value = cliclack::password(format!("Enter the value for {}", key))
                    .mask('▪')
                    .interact()?;

                save_to_keyring(key, &value)?;
            }
        }
        // If the key is in the env, ask if we want to save to keyring
        else if let Ok(value) = get_keyring_secret(key, KeyRetrievalStrategy::EnvironmentOnly) {
            cliclack::log::info(format!("Detected {} in env, we can use this from your environment.\nIt will need to continue to be set in future goose usage.", key))?;
            if cliclack::confirm("Would you like to save it to your keyring?").interact()? {
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
        cliclack::input("Enter a model from that provider:")
            .default_input(recommended_model)
            .interact()?
    };

    // Update config with new values
    config.default_provider = provider_name.clone();
    config.default_model = model.clone();

    // Test the configuration
    let spin = spinner();
    spin.start("Checking your configuration...");
    let provider = factory::get_provider(&provider_name).unwrap();
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

            match config.save() {
                Ok(()) => {
                    let msg = format!("Configuration saved to: {:?}", Config::config_path()?);
                    cliclack::outro(msg)?;
                }
                Err(e) => cliclack::outro(format!("Failed to save configuration: {}", e))?,
            };
        }
        Err(e) => {
            println!("{:?}", e);
            spin.stop("We could not connect!");
            cliclack::outro("Try rerunning configure and check your credentials.")?;
        }
    }

    Ok(())
}

pub fn get_recommended_model(provider_name: &str) -> &str {
    match provider_name {
        "openai" => OPEN_AI_DEFAULT_MODEL,
        "databricks" => DATABRICKS_DEFAULT_MODEL,
        "ollama" => OLLAMA_MODEL,
        "anthropic" => ANTHROPIC_DEFAULT_MODEL,
        "google" => GOOGLE_DEFAULT_MODEL,
        "groq" => GROQ_DEFAULT_MODEL,
        _ => panic!("Invalid provider name"),
    }
}

pub fn get_required_keys(provider_name: &str) -> Vec<&'static str> {
    match provider_name {
        "openai" => vec!["OPENAI_API_KEY"],
        "databricks" => vec!["DATABRICKS_HOST"],
        "ollama" => vec!["OLLAMA_HOST"],
        "anthropic" => vec!["ANTHROPIC_API_KEY"],
        "google" => vec!["GOOGLE_API_KEY"],
        "groq" => vec!["GROQ_API_KEY"],
        _ => panic!("Invalid provider name"),
    }
}

/// Configure systems that can be used with goose
/// Dialog for toggling which systems are enabled/disabled
pub fn toggle_systems_dialog() -> Result<(), Box<dyn Error>> {
    // Load existing config
    let mut config = Config::load().unwrap_or_default();

    if config.systems.is_empty() {
        cliclack::outro("No systems configured yet. Run configure and add some systems first.")?;
        return Ok(());
    }

    // Create a list of system names and their enabled status
    let mut system_status: Vec<(String, bool)> = Vec::new();
    for (name, entry) in config.systems.iter() {
        system_status.push((name.clone(), entry.enabled));
    }

    // Get currently enabled systems for the selection
    let enabled_systems: Vec<&String> = system_status
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| name)
        .collect();

    // Let user toggle systems
    let selected =
        cliclack::multiselect("enable systems: (use \"space\" to toggle and \"enter\" to submit)")
            .required(false)
            .items(
                &system_status
                    .iter()
                    .map(|(name, _)| (name, name.as_str(), ""))
                    .collect::<Vec<_>>(),
            )
            .initial_values(enabled_systems)
            .interact()?;

    // Update the config with new enabled/disabled status
    for (name, _) in system_status.iter() {
        if let Some(entry) = config.systems.get_mut(name) {
            entry.enabled = selected.contains(&name);
        }
    }

    config.save()?;
    cliclack::outro("System settings updated successfully")?;
    Ok(())
}

pub fn configure_systems_dialog() -> Result<(), Box<dyn Error>> {
    println!();
    println!(
        "{}",
        style("Configure will help you add systems that goose can use").dim()
    );
    println!(
        "{}",
        style("  systems provide tools and capabilities to the AI agent").dim()
    );
    println!();

    cliclack::intro(style(" goose-configure-systems ").on_cyan().black())?;

    // Load existing config or create new one
    let mut config = Config::load().unwrap_or_default();

    let system_type = cliclack::select("What type of system would you like to add?")
        .item(
            "built-in",
            "Built-in System",
            "Use a system that comes with Goose",
        )
        .item(
            "stdio",
            "Command-line System",
            "Run a local command or script",
        )
        .item("sse", "Remote System", "Connect to a remote system via SSE")
        .interact()?;

    match system_type {
        "built-in" => {
            let system = cliclack::select("Which built-in system would you like to enable?")
                .item(
                    "developer2",
                    "Developer Tools",
                    "Code editing and shell access",
                )
                .item(
                    "nondeveloper",
                    "Non Developer",
                    "AI driven scripting for non developers",
                )
                .item("jetbrains", "JetBrains", "Connect to jetbrains IDEs")
                .interact()?;

            config.systems.insert(
                system.to_string(),
                SystemEntry {
                    enabled: true,
                    config: SystemConfig::Builtin {
                        name: system.to_string(),
                    },
                },
            );

            cliclack::outro(format!("Enabled {} system", style(system).green()))?;
        }
        "stdio" => {
            let systems = config.systems.clone();
            let name: String = cliclack::input("What would you like to call this system?")
                .placeholder("my-system")
                .validate(move |input: &String| {
                    if input.is_empty() {
                        Err("Please enter a name")
                    } else if systems.contains_key(input) {
                        Err("A system with this name already exists")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            let command_str: String = cliclack::input("What command should be run?")
                .placeholder("npx -y @block/gdrive")
                .validate(|input: &String| {
                    if input.is_empty() {
                        Err("Please enter a command")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            // Split the command string into command and args
            let mut parts = command_str.split_whitespace();
            let cmd = parts.next().unwrap_or("").to_string();
            let args: Vec<String> = parts.map(String::from).collect();

            let mut envs = HashMap::new();
            if cliclack::confirm("Would you like to add environment variables?").interact()? {
                loop {
                    let key = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    envs.insert(key, value);

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            config.systems.insert(
                name.clone(),
                SystemEntry {
                    enabled: true,
                    config: SystemConfig::Stdio {
                        cmd,
                        args,
                        envs: Envs::new(envs),
                    },
                },
            );

            cliclack::outro(format!("Added {} system", style(name).green()))?;
        }
        "sse" => {
            let systems = config.systems.clone();
            let name: String = cliclack::input("What would you like to call this system?")
                .placeholder("my-remote-system")
                .validate(move |input: &String| {
                    if input.is_empty() {
                        Err("Please enter a name")
                    } else if systems.contains_key(input) {
                        Err("A system with this name already exists")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            let uri = cliclack::input("What is the SSE endpoint URI?")
                .placeholder("http://localhost:8000/events")
                .validate(|input: &String| {
                    if input.is_empty() {
                        Err("Please enter a URI")
                    } else if !input.starts_with("http") {
                        Err("URI should start with http:// or https://")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            let mut envs = HashMap::new();
            if cliclack::confirm("Would you like to add environment variables?").interact()? {
                loop {
                    let key = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    envs.insert(key, value);

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            config.systems.insert(
                name.clone(),
                SystemEntry {
                    enabled: true,
                    config: SystemConfig::Sse {
                        uri,
                        envs: Envs::new(envs),
                    },
                },
            );

            cliclack::outro(format!("Added {} system", style(name).green()))?;
        }
        _ => unreachable!(),
    };

    config.save()?;
    Ok(())
}
