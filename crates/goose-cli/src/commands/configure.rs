use cliclack::spinner;
use console::style;
use goose::agents::{extension::Envs, ExtensionConfig};
use goose::config::{Config, ConfigError, ExtensionEntry, ExtensionManager};
use goose::message::Message;
use goose::providers::{create, providers};
use mcp_core::Tool;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;

pub async fn handle_configure() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    if !config.exists() {
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
        match configure_provider_dialog().await {
            Ok(true) => {
                println!(
                    "\n  {}: Run '{}' again to adjust your config or add extensions",
                    style("Tip").green().italic(),
                    style("goose configure").cyan()
                );
                // Since we are setting up for the first time, we'll also enable the developer system
                // This operation is best-effort and errors are ignored
                ExtensionManager::set(ExtensionEntry {
                    enabled: true,
                    config: ExtensionConfig::Builtin {
                        name: "developer".to_string(),
                    },
                })?;
            }
            Ok(false) => {
                let _ = config.clear();
                println!(
                    "\n  {}: We did not save your config, inspect your credentials\n   and run '{}' again to ensure goose can connect",
                    style("Warning").yellow().italic(),
                    style("goose configure").cyan()
                );
            }
            Err(e) => {
                let _ = config.clear();

                match e.downcast_ref::<ConfigError>() {
                    Some(ConfigError::NotFound(key)) => {
                        println!(
                            "\n  {} Required configuration key '{}' not found \n  Please provide this value and run '{}' again",
                            style("Error").red().italic(),
                            key,
                            style("goose configure").cyan()
                        );
                    }
                    Some(ConfigError::KeyringError(msg)) => {
                        println!(
                            "\n  {} Failed to access secure storage (keyring): {} \n  Please check your system keychain and run '{}' again. \n  If your system is unable to use the keyring, please try setting secret key(s) via environment variables.",
                            style("Error").red().italic(),
                            msg,
                            style("goose configure").cyan()
                        );
                    }
                    Some(ConfigError::DeserializeError(msg)) => {
                        println!(
                            "\n  {} Invalid configuration value: {} \n  Please check your input and run '{}' again",
                            style("Error").red().italic(),
                            msg,
                            style("goose configure").cyan()
                        );
                    }
                    Some(ConfigError::FileError(e)) => {
                        println!(
                            "\n  {} Failed to access config file: {} \n  Please check file permissions and run '{}' again",
                            style("Error").red().italic(),
                            e,
                            style("goose configure").cyan()
                        );
                    }
                    Some(ConfigError::DirectoryError(msg)) => {
                        println!(
                            "\n  {} Failed to access config directory: {} \n  Please check directory permissions and run '{}' again",
                            style("Error").red().italic(),
                            msg,
                            style("goose configure").cyan()
                        );
                    }
                    // handle all other nonspecific errors
                    _ => {
                        println!(
                            "\n  {} {} \n  We did not save your config, inspect your credentials\n   and run '{}' again to ensure goose can connect",
                            style("Error").red().italic(),
                            e,
                            style("goose configure").cyan()
                        );
                    }
                }
            }
        }
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
            config.path()
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
                "Toggle Extensions",
                "Enable or disable connected extensions",
            )
            .item("add", "Add Extension", "Connect to a new extension")
            .interact()?;

        match action {
            "toggle" => toggle_extensions_dialog(),
            "add" => configure_extensions_dialog(),
            "providers" => configure_provider_dialog().await.and(Ok(())),
            _ => unreachable!(),
        }
    }
}

/// Dialog for configuring the AI provider and model
pub async fn configure_provider_dialog() -> Result<bool, Box<dyn Error>> {
    // Get global config instance
    let config = Config::global();

    // Get all available providers and their metadata
    let available_providers = providers();

    // Create selection items from provider metadata
    let provider_items: Vec<(&String, &str, &str)> = available_providers
        .iter()
        .map(|p| (&p.name, p.display_name.as_str(), p.description.as_str()))
        .collect();

    // Get current default provider if it exists
    let current_provider: Option<String> = config.get("GOOSE_PROVIDER").ok();
    let default_provider = current_provider.unwrap_or_default();

    // Select provider
    let provider_name = cliclack::select("Which model provider should we use?")
        .initial_value(&default_provider)
        .items(&provider_items)
        .interact()?;

    // Get the selected provider's metadata
    let provider_meta = available_providers
        .iter()
        .find(|p| &p.name == provider_name)
        .expect("Selected provider must exist in metadata");

    // Configure required provider keys
    for key in &provider_meta.config_keys {
        if !key.required {
            continue;
        }

        // First check if the value is set via environment variable
        let from_env = std::env::var(&key.name).ok();

        match from_env {
            Some(env_value) => {
                let _ =
                    cliclack::log::info(format!("{} is set via environment variable", key.name));
                if cliclack::confirm("Would you like to save this value to your keyring?")
                    .initial_value(true)
                    .interact()?
                {
                    if key.secret {
                        config.set_secret(&key.name, Value::String(env_value))?;
                    } else {
                        config.set(&key.name, Value::String(env_value))?;
                    }
                    let _ = cliclack::log::info(format!("Saved {} to config file", key.name));
                }
            }
            None => {
                // No env var, check config/secret storage
                let existing: Result<String, _> = if key.secret {
                    config.get_secret(&key.name)
                } else {
                    config.get(&key.name)
                };

                match existing {
                    Ok(_) => {
                        let _ = cliclack::log::info(format!("{} is already configured", key.name));
                        if cliclack::confirm("Would you like to update this value?").interact()? {
                            let new_value: String = if key.secret {
                                cliclack::password(format!("Enter new value for {}", key.name))
                                    .mask('▪')
                                    .interact()?
                            } else {
                                let mut input =
                                    cliclack::input(format!("Enter new value for {}", key.name));
                                if key.default.is_some() {
                                    input = input.default_input(&key.default.clone().unwrap());
                                }
                                input.interact()?
                            };

                            if key.secret {
                                config.set_secret(&key.name, Value::String(new_value))?;
                            } else {
                                config.set(&key.name, Value::String(new_value))?;
                            }
                        }
                    }
                    Err(_) => {
                        let value: String = if key.secret {
                            cliclack::password(format!(
                                "Provider {} requires {}, please enter a value",
                                provider_meta.display_name, key.name
                            ))
                            .mask('▪')
                            .interact()?
                        } else {
                            let mut input = cliclack::input(format!(
                                "Provider {} requires {}, please enter a value",
                                provider_meta.display_name, key.name
                            ));
                            if key.default.is_some() {
                                input = input.default_input(&key.default.clone().unwrap());
                            }
                            input.interact()?
                        };

                        if key.secret {
                            config.set_secret(&key.name, Value::String(value))?;
                        } else {
                            config.set(&key.name, Value::String(value))?;
                        }
                    }
                }
            }
        }
    }

    // Select model, defaulting to the provider's recommended model UNLESS there is an env override
    let default_model = std::env::var("GOOSE_MODEL").unwrap_or(provider_meta.default_model.clone());
    let model: String = cliclack::input("Enter a model from that provider:")
        .default_input(&default_model)
        .interact()?;

    // Update config with new values
    config.set("GOOSE_PROVIDER", Value::String(provider_name.to_string()))?;
    config.set("GOOSE_MODEL", Value::String(model.clone()))?;

    // Test the configuration
    let spin = spinner();
    spin.start("Checking your configuration...");

    // Use max tokens to speed up the provider test.
    let model_config = goose::model::ModelConfig::new(model.clone()).with_max_tokens(Some(50));
    let provider = create(provider_name, model_config)?;

    let messages =
        vec![Message::user().with_text("What is the weather like in San Francisco today?")];
    let sample_tool = Tool::new(
        "get_weather".to_string(),
        "Get current temperature for a given location.".to_string(),
        json!({
            "type": "object",
            "required": ["location"],
            "properties": {
                "location": {"type": "string"}
            }
        }),
    );

    let result = provider
        .complete(
            "You are an AI agent called Goose. You use tools of connected extensions to solve problems.",
            &messages,
            &[sample_tool]
        )
        .await;

    match result {
        Ok((_message, _usage)) => {
            cliclack::outro("Configuration saved successfully")?;
            Ok(true)
        }
        Err(e) => {
            spin.stop(style(e.to_string()).red());
            cliclack::outro(style("Failed to configure provider: init chat completion request with tool did not succeed.").on_red().white())?;
            Ok(false)
        }
    }
}

/// Configure extensions that can be used with goose
/// Dialog for toggling which extensions are enabled/disabled
pub fn toggle_extensions_dialog() -> Result<(), Box<dyn Error>> {
    let extensions = ExtensionManager::get_all()?;

    if extensions.is_empty() {
        cliclack::outro(
            "No extensions configured yet. Run configure and add some extensions first.",
        )?;
        return Ok(());
    }

    // Create a list of extension names and their enabled status
    let extension_status: Vec<(String, bool)> = extensions
        .iter()
        .map(|entry| (entry.config.name().to_string(), entry.enabled))
        .collect();

    // Get currently enabled extensions for the selection
    let enabled_extensions: Vec<&String> = extension_status
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| name)
        .collect();

    // Let user toggle extensions
    let selected = cliclack::multiselect(
        "enable extensions: (use \"space\" to toggle and \"enter\" to submit)",
    )
    .required(false)
    .items(
        &extension_status
            .iter()
            .map(|(name, _)| (name, name.as_str(), ""))
            .collect::<Vec<_>>(),
    )
    .initial_values(enabled_extensions)
    .interact()?;

    // Update enabled status for each extension
    for name in extension_status.iter().map(|(name, _)| name) {
        ExtensionManager::set_enabled(name, selected.iter().any(|s| s.as_str() == name))?;
    }

    cliclack::outro("Extension settings updated successfully")?;
    Ok(())
}

pub fn configure_extensions_dialog() -> Result<(), Box<dyn Error>> {
    let extension_type = cliclack::select("What type of extension would you like to add?")
        .item(
            "built-in",
            "Built-in Extension",
            "Use an extension that comes with Goose",
        )
        .item(
            "stdio",
            "Command-line Extension",
            "Run a local command or script",
        )
        .item(
            "sse",
            "Remote Extension",
            "Connect to a remote extension via SSE",
        )
        .interact()?;

    match extension_type {
        // TODO we'll want a place to collect all these options, maybe just an enum in goose-mcp
        "built-in" => {
            let extension = cliclack::select("Which built-in extension would you like to enable?")
                .item(
                    "developer",
                    "Developer Tools",
                    "Code editing and shell access",
                )
                .item(
                    "computercontroller",
                    "Computer Controller",
                    "controls for webscraping, file caching, and automations",
                )
                .item(
                    "google_drive",
                    "Google Drive",
                    "Search and read content from google drive - additional config required",
                )
                .item(
                    "memory",
                    "Memory",
                    "Tools to save and retrieve durable memories",
                )
                .item("jetbrains", "JetBrains", "Connect to jetbrains IDEs")
                .interact()?
                .to_string();

            ExtensionManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Builtin {
                    name: extension.clone(),
                },
            })?;

            cliclack::outro(format!("Enabled {} extension", style(extension).green()))?;
        }
        "stdio" => {
            let extensions = ExtensionManager::get_all_names()?;
            let name: String = cliclack::input("What would you like to call this extension?")
                .placeholder("my-extension")
                .validate(move |input: &String| {
                    if input.is_empty() {
                        Err("Please enter a name")
                    } else if extensions.contains(input) {
                        Err("An extension with this name already exists")
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

            let add_env =
                cliclack::confirm("Would you like to add environment variables?").interact()?;

            let mut envs = HashMap::new();
            if add_env {
                loop {
                    let key: String = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value: String = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    envs.insert(key, value);

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            ExtensionManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Stdio {
                    name: name.clone(),
                    cmd,
                    args,
                    envs: Envs::new(envs),
                },
            })?;

            cliclack::outro(format!("Added {} extension", style(name).green()))?;
        }
        "sse" => {
            let extensions = ExtensionManager::get_all_names()?;
            let name: String = cliclack::input("What would you like to call this extension?")
                .placeholder("my-remote-extension")
                .validate(move |input: &String| {
                    if input.is_empty() {
                        Err("Please enter a name")
                    } else if extensions.contains(input) {
                        Err("An extension with this name already exists")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            let uri: String = cliclack::input("What is the SSE endpoint URI?")
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

            let add_env =
                cliclack::confirm("Would you like to add environment variables?").interact()?;

            let mut envs = HashMap::new();
            if add_env {
                loop {
                    let key: String = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value: String = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    envs.insert(key, value);

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            ExtensionManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Sse {
                    name: name.clone(),
                    uri,
                    envs: Envs::new(envs),
                },
            })?;

            cliclack::outro(format!("Added {} extension", style(name).green()))?;
        }
        _ => unreachable!(),
    };

    Ok(())
}
