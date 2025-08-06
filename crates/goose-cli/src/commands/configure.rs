use cliclack::spinner;
use console::style;
use goose::agents::extension::ToolInfo;
use goose::agents::extension_manager::get_parameter_names;
use goose::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
};
use goose::agents::Agent;
use goose::agents::{extension::Envs, ExtensionConfig};
use goose::config::extensions::name_to_key;
use goose::config::permission::PermissionLevel;
use goose::config::{
    Config, ConfigError, ExperimentManager, ExtensionConfigManager, ExtensionEntry,
    PermissionManager,
};
use goose::conversation::message::Message;
use goose::providers::{create, providers};
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

use crate::recipes::github_recipe::GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY;

// useful for light themes where there is no dicernible colour contrast between
// cursor-selected and cursor-unselected items.
const MULTISELECT_VISIBILITY_HINT: &str = "<";

fn get_display_name(extension_id: &str) -> String {
    match extension_id {
        "developer" => "Developer Tools".to_string(),
        "computercontroller" => "Computer Controller".to_string(),
        "googledrive" => "Google Drive".to_string(),
        "memory" => "Memory".to_string(),
        "tutorial" => "Tutorial".to_string(),
        "jetbrains" => "JetBrains".to_string(),
        // Add other extensions as needed
        _ => {
            extension_id
                .chars()
                .next()
                .unwrap_or_default()
                .to_uppercase()
                .collect::<String>()
                + &extension_id[1..]
        }
    }
}

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

        // Check if user wants to use OpenRouter login or manual configuration
        let setup_method = cliclack::select("How would you like to set up your provider?")
            .item(
                "openrouter",
                "OpenRouter Login (Recommended)",
                "Sign in with OpenRouter to automatically configure models",
            )
            .item(
                "manual",
                "Manual Configuration",
                "Choose a provider and enter credentials manually",
            )
            .interact()?;

        match setup_method {
            "openrouter" => {
                match handle_openrouter_auth().await {
                    Ok(_) => {
                        // OpenRouter auth already handles everything including enabling developer extension
                    }
                    Err(e) => {
                        let _ = config.clear();
                        println!(
                            "\n  {} OpenRouter authentication failed: {} \n  Please try again or use manual configuration",
                            style("Error").red().italic(),
                            e,
                        );
                    }
                }
            }
            "manual" => {
                match configure_provider_dialog().await {
                    Ok(true) => {
                        println!(
                            "\n  {}: Run '{}' again to adjust your config or add extensions",
                            style("Tip").green().italic(),
                            style("goose configure").cyan()
                        );
                        // Since we are setting up for the first time, we'll also enable the developer system
                        // This operation is best-effort and errors are ignored
                        ExtensionConfigManager::set(ExtensionEntry {
                            enabled: true,
                            config: ExtensionConfig::Builtin {
                                name: "developer".to_string(),
                                display_name: Some(goose::config::DEFAULT_DISPLAY_NAME.to_string()),
                                timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
                                bundled: Some(true),
                                description: None,
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
                                #[cfg(target_os = "macos")]
                                println!(
                                    "\n  {} Failed to access secure storage (keyring): {} \n  Please check your system keychain and run '{}' again. \n  If your system is unable to use the keyring, please try setting secret key(s) via environment variables.",
                                    style("Error").red().italic(),
                                    msg,
                                    style("goose configure").cyan()
                                );

                                #[cfg(target_os = "windows")]
                                println!(
                                    "\n  {} Failed to access Windows Credential Manager: {} \n  Please check Windows Credential Manager and run '{}' again. \n  If your system is unable to use the Credential Manager, please try setting secret key(s) via environment variables.",
                                    style("Error").red().italic(),
                                    msg,
                                    style("goose configure").cyan()
                                );

                                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                                println!(
                                    "\n  {} Failed to access secure storage: {} \n  Please check your system's secure storage and run '{}' again. \n  If your system is unable to use secure storage, please try setting secret key(s) via environment variables.",
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
            }
            _ => unreachable!(),
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
            .item("add", "Add Extension", "Connect to a new extension")
            .item(
                "toggle",
                "Toggle Extensions",
                "Enable or disable connected extensions",
            )
            .item("remove", "Remove Extension", "Remove an extension")
            .item(
                "settings",
                "Goose Settings",
                "Set the Goose Mode, Tool Output, Tool Permissions, Experiment, Goose recipe github repo and more",
            )
            .interact()?;

        match action {
            "toggle" => toggle_extensions_dialog(),
            "add" => configure_extensions_dialog(),
            "remove" => remove_extension_dialog(),
            "settings" => configure_settings_dialog().await.and(Ok(())),
            "providers" => configure_provider_dialog().await.and(Ok(())),
            _ => unreachable!(),
        }
    }
}

/// Helper function to handle OAuth configuration for a provider
async fn handle_oauth_configuration(
    provider_name: &str,
    key_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use goose::model::ModelConfig;
    use goose::providers::create;

    let _ = cliclack::log::info(format!(
        "Configuring {} using OAuth device code flow...",
        key_name
    ));

    // Create a temporary provider instance to handle OAuth
    let temp_model = ModelConfig::new("temp")?;
    match create(provider_name, temp_model) {
        Ok(provider) => match provider.configure_oauth().await {
            Ok(_) => {
                let _ = cliclack::log::success("OAuth authentication completed successfully!");
                Ok(())
            }
            Err(e) => {
                let _ = cliclack::log::error(format!("Failed to authenticate: {}", e));
                Err(format!("OAuth authentication failed for {}: {}", key_name, e).into())
            }
        },
        Err(e) => {
            let _ = cliclack::log::error(format!("Failed to create provider for OAuth: {}", e));
            Err(format!("Failed to create provider for OAuth: {}", e).into())
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
    let current_provider: Option<String> = config.get_param("GOOSE_PROVIDER").ok();
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
                        config.set_param(&key.name, Value::String(env_value))?;
                    }
                    let _ = cliclack::log::info(format!("Saved {} to config file", key.name));
                }
            }
            None => {
                // No env var, check config/secret storage
                let existing: Result<String, _> = if key.secret {
                    config.get_secret(&key.name)
                } else {
                    config.get_param(&key.name)
                };

                match existing {
                    Ok(_) => {
                        let _ = cliclack::log::info(format!("{} is already configured", key.name));
                        if cliclack::confirm("Would you like to update this value?").interact()? {
                            // Check if this key uses OAuth flow
                            if key.oauth_flow {
                                handle_oauth_configuration(provider_name, &key.name).await?;
                            } else {
                                // Non-OAuth key, use manual entry
                                let value: String = if key.secret {
                                    cliclack::password(format!("Enter new value for {}", key.name))
                                        .mask('▪')
                                        .interact()?
                                } else {
                                    let mut input = cliclack::input(format!(
                                        "Enter new value for {}",
                                        key.name
                                    ));
                                    if key.default.is_some() {
                                        input = input.default_input(&key.default.clone().unwrap());
                                    }
                                    input.interact()?
                                };

                                if key.secret {
                                    config.set_secret(&key.name, Value::String(value))?;
                                } else {
                                    config.set_param(&key.name, Value::String(value))?;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Check if this key uses OAuth flow
                        if key.oauth_flow {
                            handle_oauth_configuration(provider_name, &key.name).await?;
                        } else {
                            // Non-OAuth key, use manual entry
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
                                config.set_param(&key.name, Value::String(value))?;
                            }
                        }
                    }
                }
            }
        }
    }

    // Attempt to fetch supported models for this provider
    let spin = spinner();
    spin.start("Attempting to fetch supported models...");
    let models_res = {
        let temp_model_config = goose::model::ModelConfig::new(&provider_meta.default_model)?;
        let temp_provider = create(provider_name, temp_model_config)?;
        temp_provider.fetch_supported_models().await
    };
    spin.stop(style("Model fetch complete").green());

    // Select a model: on fetch error show styled error and abort; if Some(models), show list; if None, free-text input
    let model: String = match models_res {
        Err(e) => {
            // Provider hook error
            cliclack::outro(style(e.to_string()).on_red().white())?;
            return Ok(false);
        }
        Ok(Some(models)) => cliclack::select("Select a model:")
            .items(
                &models
                    .iter()
                    .map(|m| (m, m.as_str(), ""))
                    .collect::<Vec<_>>(),
            )
            .filter_mode() // enable "fuzzy search" filtering for the list of models
            .interact()?
            .to_string(),
        Ok(None) => {
            let default_model =
                std::env::var("GOOSE_MODEL").unwrap_or(provider_meta.default_model.clone());
            cliclack::input("Enter a model from that provider:")
                .default_input(&default_model)
                .interact()?
        }
    };

    // Test the configuration
    let spin = spinner();
    spin.start("Checking your configuration...");

    // Create model config with env var settings
    let toolshim_enabled = std::env::var("GOOSE_TOOLSHIM")
        .map(|val| val == "1" || val.to_lowercase() == "true")
        .unwrap_or(false);

    let model_config = goose::model::ModelConfig::new(&model)?
        .with_max_tokens(Some(50))
        .with_toolshim(toolshim_enabled)
        .with_toolshim_model(std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL").ok());

    let provider = create(provider_name, model_config)?;

    let messages =
        vec![Message::user().with_text("What is the weather like in San Francisco today?")];
    // Only add the sample tool if toolshim is not enabled
    let tools = if !toolshim_enabled {
        let sample_tool = Tool::new(
            "get_weather".to_string(),
            "Get current temperature for a given location.".to_string(),
            object!({
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Get weather".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });
        vec![sample_tool]
    } else {
        vec![]
    };

    let result = provider
        .complete(
            "You are an AI agent called Goose. You use tools of connected extensions to solve problems.",
            &messages,
            &tools.into_iter().collect::<Vec<_>>()
        ).await;

    match result {
        Ok((_message, _usage)) => {
            // Update config with new values only if the test succeeds
            config.set_param("GOOSE_PROVIDER", Value::String(provider_name.to_string()))?;
            config.set_param("GOOSE_MODEL", Value::String(model.clone()))?;
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
    let extensions = ExtensionConfigManager::get_all()?;

    if extensions.is_empty() {
        cliclack::outro(
            "No extensions configured yet. Run configure and add some extensions first.",
        )?;
        return Ok(());
    }

    // Create a list of extension names and their enabled status
    let mut extension_status: Vec<(String, bool)> = extensions
        .iter()
        .map(|entry| (entry.config.name().to_string(), entry.enabled))
        .collect();

    // Sort extensions alphabetically by name
    extension_status.sort_by(|a, b| a.0.cmp(&b.0));

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
            .map(|(name, _)| (name, name.as_str(), MULTISELECT_VISIBILITY_HINT))
            .collect::<Vec<_>>(),
    )
    .initial_values(enabled_extensions)
    .interact()?;

    // Update enabled status for each extension
    for name in extension_status.iter().map(|(name, _)| name) {
        ExtensionConfigManager::set_enabled(
            &name_to_key(name),
            selected.iter().any(|s| s.as_str() == name),
        )?;
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
            "Remote Extension (SSE)",
            "Connect to a remote extension via Server-Sent Events",
        )
        .item(
            "streamable_http",
            "Remote Extension (Streaming HTTP)",
            "Connect to a remote extension via MCP Streaming HTTP",
        )
        .interact()?;

    match extension_type {
        // TODO we'll want a place to collect all these options, maybe just an enum in goose-mcp
        "built-in" => {
            let extension = cliclack::select("Which built-in extension would you like to enable?")
                .item(
                    "computercontroller",
                    "Computer Controller",
                    "controls for webscraping, file caching, and automations",
                )
                .item(
                    "developer",
                    "Developer Tools",
                    "Code editing and shell access",
                )
                .item(
                    "googledrive",
                    "Google Drive",
                    "Search and read content from google drive - additional config required",
                )
                .item("jetbrains", "JetBrains", "Connect to jetbrains IDEs")
                .item(
                    "memory",
                    "Memory",
                    "Tools to save and retrieve durable memories",
                )
                .item(
                    "tutorial",
                    "Tutorial",
                    "Access interactive tutorials and guides",
                )
                .interact()?
                .to_string();

            let timeout: u64 = cliclack::input("Please set the timeout for this tool (in secs):")
                .placeholder(&goose::config::DEFAULT_EXTENSION_TIMEOUT.to_string())
                .validate(|input: &String| match input.parse::<u64>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Please enter a valid timeout"),
                })
                .interact()?;

            let display_name = get_display_name(&extension);

            ExtensionConfigManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Builtin {
                    name: extension.clone(),
                    display_name: Some(display_name),
                    timeout: Some(timeout),
                    bundled: Some(true),
                    description: None,
                },
            })?;

            cliclack::outro(format!("Enabled {} extension", style(extension).green()))?;
        }
        "stdio" => {
            let extensions = ExtensionConfigManager::get_all_names()?;
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

            let timeout: u64 = cliclack::input("Please set the timeout for this tool (in secs):")
                .placeholder(&goose::config::DEFAULT_EXTENSION_TIMEOUT.to_string())
                .validate(|input: &String| match input.parse::<u64>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Please enter a valid timeout"),
                })
                .interact()?;

            // Split the command string into command and args
            // TODO: find a way to expose this to the frontend so we dont need to re-write code
            let mut parts = command_str.split_whitespace();
            let cmd = parts.next().unwrap_or("").to_string();
            let args: Vec<String> = parts.map(String::from).collect();

            let add_desc = cliclack::confirm("Would you like to add a description?").interact()?;

            let description = if add_desc {
                let desc = cliclack::input("Enter a description for this extension:")
                    .placeholder("Description")
                    .validate(|input: &String| match input.parse::<String>() {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Please enter a valid description"),
                    })
                    .interact()?;
                Some(desc)
            } else {
                None
            };

            let add_env =
                cliclack::confirm("Would you like to add environment variables?").interact()?;

            let mut envs = HashMap::new();
            let mut env_keys = Vec::new();
            let config = Config::global();

            if add_env {
                loop {
                    let key: String = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value: String = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    // Try to store in keychain
                    let keychain_key = key.to_string();
                    match config.set_secret(&keychain_key, Value::String(value.clone())) {
                        Ok(_) => {
                            // Successfully stored in keychain, add to env_keys
                            env_keys.push(keychain_key);
                        }
                        Err(_) => {
                            // Failed to store in keychain, store directly in envs
                            envs.insert(key, value);
                        }
                    }

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            ExtensionConfigManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Stdio {
                    name: name.clone(),
                    cmd,
                    args,
                    envs: Envs::new(envs),
                    env_keys,
                    description,
                    timeout: Some(timeout),
                    bundled: None,
                },
            })?;

            cliclack::outro(format!("Added {} extension", style(name).green()))?;
        }
        "sse" => {
            let extensions = ExtensionConfigManager::get_all_names()?;
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

            let timeout: u64 = cliclack::input("Please set the timeout for this tool (in secs):")
                .placeholder(&goose::config::DEFAULT_EXTENSION_TIMEOUT.to_string())
                .validate(|input: &String| match input.parse::<u64>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Please enter a valid timeout"),
                })
                .interact()?;

            let add_desc = cliclack::confirm("Would you like to add a description?").interact()?;

            let description = if add_desc {
                let desc = cliclack::input("Enter a description for this extension:")
                    .placeholder("Description")
                    .validate(|input: &String| match input.parse::<String>() {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Please enter a valid description"),
                    })
                    .interact()?;
                Some(desc)
            } else {
                None
            };

            let add_env =
                cliclack::confirm("Would you like to add environment variables?").interact()?;

            let mut envs = HashMap::new();
            let mut env_keys = Vec::new();
            let config = Config::global();

            if add_env {
                loop {
                    let key: String = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value: String = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    // Try to store in keychain
                    let keychain_key = key.to_string();
                    match config.set_secret(&keychain_key, Value::String(value.clone())) {
                        Ok(_) => {
                            // Successfully stored in keychain, add to env_keys
                            env_keys.push(keychain_key);
                        }
                        Err(_) => {
                            // Failed to store in keychain, store directly in envs
                            envs.insert(key, value);
                        }
                    }

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            ExtensionConfigManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::Sse {
                    name: name.clone(),
                    uri,
                    envs: Envs::new(envs),
                    env_keys,
                    description,
                    timeout: Some(timeout),
                    bundled: None,
                },
            })?;

            cliclack::outro(format!("Added {} extension", style(name).green()))?;
        }
        "streamable_http" => {
            let extensions = ExtensionConfigManager::get_all_names()?;
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

            let uri: String = cliclack::input("What is the Streaming HTTP endpoint URI?")
                .placeholder("http://localhost:8000/messages")
                .validate(|input: &String| {
                    if input.is_empty() {
                        Err("Please enter a URI")
                    } else if !(input.starts_with("http://") || input.starts_with("https://")) {
                        Err("URI should start with http:// or https://")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;

            let timeout: u64 = cliclack::input("Please set the timeout for this tool (in secs):")
                .placeholder(&goose::config::DEFAULT_EXTENSION_TIMEOUT.to_string())
                .validate(|input: &String| match input.parse::<u64>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Please enter a valid timeout"),
                })
                .interact()?;

            let add_desc = cliclack::confirm("Would you like to add a description?").interact()?;

            let description = if add_desc {
                let desc = cliclack::input("Enter a description for this extension:")
                    .placeholder("Description")
                    .validate(|input: &String| {
                        if input.trim().is_empty() {
                            Err("Please enter a valid description")
                        } else {
                            Ok(())
                        }
                    })
                    .interact()?;
                Some(desc)
            } else {
                None
            };

            let add_headers =
                cliclack::confirm("Would you like to add custom headers?").interact()?;

            let mut headers = HashMap::new();
            if add_headers {
                loop {
                    let key: String = cliclack::input("Header name:")
                        .placeholder("Authorization")
                        .interact()?;

                    let value: String = cliclack::input("Header value:")
                        .placeholder("Bearer token123")
                        .interact()?;

                    headers.insert(key, value);

                    if !cliclack::confirm("Add another header?").interact()? {
                        break;
                    }
                }
            }

            let add_env = false; // No env prompt for Streaming HTTP

            let mut envs = HashMap::new();
            let mut env_keys = Vec::new();
            let config = Config::global();

            if add_env {
                loop {
                    let key: String = cliclack::input("Environment variable name:")
                        .placeholder("API_KEY")
                        .interact()?;

                    let value: String = cliclack::password("Environment variable value:")
                        .mask('▪')
                        .interact()?;

                    // Try to store in keychain
                    let keychain_key = key.to_string();
                    match config.set_secret(&keychain_key, Value::String(value.clone())) {
                        Ok(_) => {
                            // Successfully stored in keychain, add to env_keys
                            env_keys.push(keychain_key);
                        }
                        Err(_) => {
                            // Failed to store in keychain, store directly in envs
                            envs.insert(key, value);
                        }
                    }

                    if !cliclack::confirm("Add another environment variable?").interact()? {
                        break;
                    }
                }
            }

            ExtensionConfigManager::set(ExtensionEntry {
                enabled: true,
                config: ExtensionConfig::StreamableHttp {
                    name: name.clone(),
                    uri,
                    envs: Envs::new(envs),
                    env_keys,
                    headers,
                    description,
                    timeout: Some(timeout),
                    bundled: None,
                },
            })?;

            cliclack::outro(format!("Added {} extension", style(name).green()))?;
        }
        _ => unreachable!(),
    };

    Ok(())
}

pub fn remove_extension_dialog() -> Result<(), Box<dyn Error>> {
    let extensions = ExtensionConfigManager::get_all()?;

    // Create a list of extension names and their enabled status
    let mut extension_status: Vec<(String, bool)> = extensions
        .iter()
        .map(|entry| (entry.config.name().to_string(), entry.enabled))
        .collect();

    // Sort extensions alphabetically by name
    extension_status.sort_by(|a, b| a.0.cmp(&b.0));

    if extensions.is_empty() {
        cliclack::outro(
            "No extensions configured yet. Run configure and add some extensions first.",
        )?;
        return Ok(());
    }

    // Check if all extensions are enabled
    if extension_status.iter().all(|(_, enabled)| *enabled) {
        cliclack::outro(
            "All extensions are currently enabled. You must first disable extensions before removing them.",
        )?;
        return Ok(());
    }

    // Filter out only disabled extensions
    let disabled_extensions: Vec<_> = extensions
        .iter()
        .filter(|entry| !entry.enabled)
        .map(|entry| (entry.config.name().to_string(), entry.enabled))
        .collect();

    let selected = cliclack::multiselect("Select extensions to remove (note: you can only remove disabled extensions - use \"space\" to toggle and \"enter\" to submit)")
        .required(false)
        .items(
            &disabled_extensions
                .iter()
                .filter(|(_, enabled)| !enabled)
                .map(|(name, _)| (name, name.as_str(), MULTISELECT_VISIBILITY_HINT))
                .collect::<Vec<_>>(),
        )
        .interact()?;

    for name in selected {
        ExtensionConfigManager::remove(&name_to_key(name))?;
        let mut permission_manager = PermissionManager::default();
        permission_manager.remove_extension(&name_to_key(name));
        cliclack::outro(format!("Removed {} extension", style(name).green()))?;
    }

    Ok(())
}

pub async fn configure_settings_dialog() -> Result<(), Box<dyn Error>> {
    let setting_type = cliclack::select("What setting would you like to configure?")
        .item("goose_mode", "Goose Mode", "Configure Goose mode")
        .item(
            "goose_router_strategy",
            "Router Tool Selection Strategy",
            "Configure the strategy for selecting tools to use",
        )
        .item(
            "tool_permission",
            "Tool Permission",
            "Set permission for individual tool of enabled extensions",
        )
        .item(
            "tool_output",
            "Tool Output",
            "Show more or less tool output",
        )
        .item(
            "max_turns",
            "Max Turns",
            "Set maximum number of turns without user input",
        )
        .item(
            "experiment",
            "Toggle Experiment",
            "Enable or disable an experiment feature",
        )
        .item(
            "recipe",
            "Goose recipe github repo",
            "Goose will pull recipes from this repo if not found locally.",
        )
        .item(
            "scheduler",
            "Scheduler Type",
            "Choose between built-in cron scheduler or Temporal workflow engine",
        )
        .interact()?;

    match setting_type {
        "goose_mode" => {
            configure_goose_mode_dialog()?;
        }
        "goose_router_strategy" => {
            configure_goose_router_strategy_dialog()?;
        }
        "tool_permission" => {
            configure_tool_permissions_dialog().await.and(Ok(()))?;
        }
        "tool_output" => {
            configure_tool_output_dialog()?;
        }
        "max_turns" => {
            configure_max_turns_dialog()?;
        }
        "experiment" => {
            toggle_experiments_dialog()?;
        }
        "recipe" => {
            configure_recipe_dialog()?;
        }
        "scheduler" => {
            configure_scheduler_dialog()?;
        }
        _ => unreachable!(),
    };

    Ok(())
}

pub fn configure_goose_mode_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    // Check if GOOSE_MODE is set as an environment variable
    if std::env::var("GOOSE_MODE").is_ok() {
        let _ = cliclack::log::info("Notice: GOOSE_MODE environment variable is set and will override the configuration here.");
    }

    let mode = cliclack::select("Which Goose mode would you like to configure?")
        .item(
            "auto",
            "Auto Mode",
            "Full file modification, extension usage, edit, create and delete files freely"
        )
        .item(
            "approve",
            "Approve Mode",
            "All tools, extensions and file modifications will require human approval"
        )
        .item(
            "smart_approve",
            "Smart Approve Mode",
            "Editing, creating, deleting files and using extensions will require human approval"
        )
        .item(
            "chat",
            "Chat Mode",
            "Engage with the selected provider without using tools, extensions, or file modification"
        )
        .interact()?;

    match mode {
        "auto" => {
            config.set_param("GOOSE_MODE", Value::String("auto".to_string()))?;
            cliclack::outro("Set to Auto Mode - full file modification enabled")?;
        }
        "approve" => {
            config.set_param("GOOSE_MODE", Value::String("approve".to_string()))?;
            cliclack::outro("Set to Approve Mode - all tools and modifications require approval")?;
        }
        "smart_approve" => {
            config.set_param("GOOSE_MODE", Value::String("smart_approve".to_string()))?;
            cliclack::outro("Set to Smart Approve Mode - modifications require approval")?;
        }
        "chat" => {
            config.set_param("GOOSE_MODE", Value::String("chat".to_string()))?;
            cliclack::outro("Set to Chat Mode - no tools or modifications enabled")?;
        }
        _ => unreachable!(),
    };
    Ok(())
}

pub fn configure_goose_router_strategy_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    // Check if GOOSE_ROUTER_STRATEGY is set as an environment variable
    if std::env::var("GOOSE_ROUTER_TOOL_SELECTION_STRATEGY").is_ok() {
        let _ = cliclack::log::info("Notice: GOOSE_ROUTER_TOOL_SELECTION_STRATEGY environment variable is set. Configuration will override this.");
    }

    let strategy = cliclack::select("Which router strategy would you like to use?")
        .item(
            "vector",
            "Vector Strategy",
            "Use vector-based similarity to select tools",
        )
        .item(
            "default",
            "Default Strategy",
            "Use the default tool selection strategy",
        )
        .interact()?;

    match strategy {
        "vector" => {
            config.set_param(
                "GOOSE_ROUTER_TOOL_SELECTION_STRATEGY",
                Value::String("vector".to_string()),
            )?;
            cliclack::outro(
                "Set to Vector Strategy - using vector-based similarity for tool selection",
            )?;
        }
        "default" => {
            config.set_param(
                "GOOSE_ROUTER_TOOL_SELECTION_STRATEGY",
                Value::String("default".to_string()),
            )?;
            cliclack::outro("Set to Default Strategy - using default tool selection")?;
        }
        _ => unreachable!(),
    };
    Ok(())
}

pub fn configure_tool_output_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();
    // Check if GOOSE_CLI_MIN_PRIORITY is set as an environment variable
    if std::env::var("GOOSE_CLI_MIN_PRIORITY").is_ok() {
        let _ = cliclack::log::info("Notice: GOOSE_CLI_MIN_PRIORITY environment variable is set and will override the configuration here.");
    }
    let tool_log_level = cliclack::select("Which tool output would you like to show?")
        .item("high", "High Importance", "")
        .item("medium", "Medium Importance", "Ex. results of file-writes")
        .item("all", "All (default)", "Ex. shell command output")
        .interact()?;

    match tool_log_level {
        "high" => {
            config.set_param("GOOSE_CLI_MIN_PRIORITY", Value::from(0.8))?;
            cliclack::outro("Showing tool output of high importance only.")?;
        }
        "medium" => {
            config.set_param("GOOSE_CLI_MIN_PRIORITY", Value::from(0.2))?;
            cliclack::outro("Showing tool output of medium importance.")?;
        }
        "all" => {
            config.set_param("GOOSE_CLI_MIN_PRIORITY", Value::from(0.0))?;
            cliclack::outro("Showing all tool output.")?;
        }
        _ => unreachable!(),
    };

    Ok(())
}

/// Configure experiment features that can be used with goose
/// Dialog for toggling which experiments are enabled/disabled
pub fn toggle_experiments_dialog() -> Result<(), Box<dyn Error>> {
    let experiments = ExperimentManager::get_all()?;

    if experiments.is_empty() {
        cliclack::outro("No experiments supported yet.")?;
        return Ok(());
    }

    // Get currently enabled experiments for the selection
    let enabled_experiments: Vec<&String> = experiments
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| name)
        .collect();

    // Let user toggle experiments
    let selected = cliclack::multiselect(
        "enable experiments: (use \"space\" to toggle and \"enter\" to submit)",
    )
    .required(false)
    .items(
        &experiments
            .iter()
            .map(|(name, _)| (name, name.as_str(), MULTISELECT_VISIBILITY_HINT))
            .collect::<Vec<_>>(),
    )
    .initial_values(enabled_experiments)
    .interact()?;

    // Update enabled status for each experiments
    for name in experiments.iter().map(|(name, _)| name) {
        ExperimentManager::set_enabled(name, selected.iter().any(|&s| s.as_str() == name))?;
    }

    cliclack::outro("Experiments settings updated successfully")?;
    Ok(())
}

pub async fn configure_tool_permissions_dialog() -> Result<(), Box<dyn Error>> {
    let mut extensions: Vec<String> = ExtensionConfigManager::get_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|ext| ext.enabled)
        .map(|ext| ext.config.name().clone())
        .collect();
    extensions.push("platform".to_string());

    // Sort extensions alphabetically by name
    extensions.sort();

    let selected_extension_name = cliclack::select("Choose an extension to configure tools")
        .items(
            &extensions
                .iter()
                .map(|ext| (ext.clone(), ext.clone(), ""))
                .collect::<Vec<_>>(),
        )
        .interact()?;

    // Fetch tools for the selected extension
    // Load config and get provider/model
    let config = Config::global();

    let provider_name: String = config
        .get_param("GOOSE_PROVIDER")
        .expect("No provider configured. Please set model provider first");

    let model: String = config
        .get_param("GOOSE_MODEL")
        .expect("No model configured. Please set model first");
    let model_config = goose::model::ModelConfig::new(&model)?;

    // Create the agent
    let agent = Agent::new();
    let new_provider = create(&provider_name, model_config)?;
    agent.update_provider(new_provider).await?;
    if let Ok(Some(config)) = ExtensionConfigManager::get_config_by_name(&selected_extension_name) {
        agent
            .add_extension(config.clone())
            .await
            .unwrap_or_else(|_| {
                println!(
                    "{} Failed to check extension: {}",
                    style("Error").red().italic(),
                    config.name()
                );
            });
    } else {
        println!(
            "{} Configuration not found for extension: {}",
            style("Warning").yellow().italic(),
            selected_extension_name
        );
        return Ok(());
    }

    let mut permission_manager = PermissionManager::default();
    let selected_tools = agent
        .list_tools(Some(selected_extension_name.clone()))
        .await
        .into_iter()
        .filter(|tool| {
            tool.name != PLATFORM_LIST_RESOURCES_TOOL_NAME
                && tool.name != PLATFORM_READ_RESOURCE_TOOL_NAME
        })
        .map(|tool| {
            ToolInfo::new(
                &tool.name,
                tool.description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default(),
                get_parameter_names(&tool),
                permission_manager.get_user_permission(&tool.name),
            )
        })
        .collect::<Vec<ToolInfo>>();

    let tool_name = cliclack::select("Choose a tool to update permission")
        .items(
            &selected_tools
                .iter()
                .map(|tool| {
                    let first_description = tool
                        .description
                        .split('.')
                        .next()
                        .unwrap_or("No description available")
                        .trim();
                    (tool.name.clone(), tool.name.clone(), first_description)
                })
                .collect::<Vec<_>>(),
        )
        .interact()?;

    // Find the selected tool
    let tool = selected_tools
        .iter()
        .find(|tool| tool.name == tool_name)
        .unwrap();

    // Display tool description and current permission level
    let current_permission = match tool.permission {
        Some(PermissionLevel::AlwaysAllow) => "Always Allow",
        Some(PermissionLevel::AskBefore) => "Ask Before",
        Some(PermissionLevel::NeverAllow) => "Never Allow",
        None => "Not Set",
    };

    // Allow user to set the permission level
    let permission = cliclack::select(format!(
        "Set permission level for tool {}, current permission level: {}",
        tool.name, current_permission
    ))
    .item(
        "always_allow",
        "Always Allow",
        "Allow this tool to execute without asking",
    )
    .item(
        "ask_before",
        "Ask Before",
        "Prompt before executing this tool",
    )
    .item(
        "never_allow",
        "Never Allow",
        "Prevent this tool from executing",
    )
    .interact()?;

    let permission_label = match permission {
        "always_allow" => "Always Allow",
        "ask_before" => "Ask Before",
        "never_allow" => "Never Allow",
        _ => unreachable!(),
    };

    // Update the permission level in the configuration
    let new_permission = match permission {
        "always_allow" => PermissionLevel::AlwaysAllow,
        "ask_before" => PermissionLevel::AskBefore,
        "never_allow" => PermissionLevel::NeverAllow,
        _ => unreachable!(),
    };

    permission_manager.update_user_permission(&tool.name, new_permission);

    cliclack::outro(format!(
        "Updated permission level for tool {} to {}.",
        tool.name, permission_label
    ))?;

    Ok(())
}

fn configure_recipe_dialog() -> Result<(), Box<dyn Error>> {
    let key_name = GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY;
    let config = Config::global();
    let default_recipe_repo = std::env::var(key_name)
        .ok()
        .or_else(|| config.get_param(key_name).unwrap_or(None));
    let mut recipe_repo_input = cliclack::input(
        "Enter your Goose Recipe Github repo (owner/repo): eg: my_org/goose-recipes",
    )
    .required(false);
    if let Some(recipe_repo) = default_recipe_repo {
        recipe_repo_input = recipe_repo_input.default_input(&recipe_repo);
    }
    let input_value: String = recipe_repo_input.interact()?;
    // if input is blank, it clears the recipe github repo settings in the config file
    if input_value.clone().trim().is_empty() {
        config.delete(key_name)?;
    } else {
        config.set_param(key_name, Value::String(input_value))?;
    }
    Ok(())
}

fn configure_scheduler_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    // Check if GOOSE_SCHEDULER_TYPE is set as an environment variable
    if std::env::var("GOOSE_SCHEDULER_TYPE").is_ok() {
        let _ = cliclack::log::info("Notice: GOOSE_SCHEDULER_TYPE environment variable is set and will override the configuration here.");
    }

    // Get current scheduler type from config for display
    let current_scheduler: String = config
        .get_param("GOOSE_SCHEDULER_TYPE")
        .unwrap_or_else(|_| "legacy".to_string());

    println!(
        "Current scheduler type: {}",
        style(&current_scheduler).cyan()
    );

    let scheduler_type = cliclack::select("Which scheduler type would you like to use?")
        .items(&[
            ("legacy", "Built-in Cron (Default)", "Uses Goose's built-in cron scheduler. Simple and reliable for basic scheduling needs."),
            ("temporal", "Temporal", "Uses Temporal workflow engine for advanced scheduling features. Requires Temporal CLI to be installed.")
        ])
        .interact()?;

    match scheduler_type {
        "legacy" => {
            config.set_param("GOOSE_SCHEDULER_TYPE", Value::String("legacy".to_string()))?;
            cliclack::outro(
                "Set to Built-in Cron scheduler - simple and reliable for basic scheduling",
            )?;
        }
        "temporal" => {
            config.set_param(
                "GOOSE_SCHEDULER_TYPE",
                Value::String("temporal".to_string()),
            )?;
            cliclack::outro(
                "Set to Temporal scheduler - advanced workflow engine for complex scheduling",
            )?;
            println!();
            println!("📋 {}", style("Note:").bold());
            println!("  • Temporal scheduler requires Temporal CLI to be installed");
            println!("  • macOS: brew install temporal");
            println!("  • Linux/Windows: https://github.com/temporalio/cli/releases");
            println!("  • If Temporal is unavailable, Goose will automatically fall back to the built-in scheduler");
            println!("  • The scheduling engines do not share the list of schedules");
        }
        _ => unreachable!(),
    };

    Ok(())
}

pub fn configure_max_turns_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    let current_max_turns: u32 = config.get_param("GOOSE_MAX_TURNS").unwrap_or(1000);

    let max_turns_input: String =
        cliclack::input("Set maximum number of agent turns without user input:")
            .placeholder(&current_max_turns.to_string())
            .default_input(&current_max_turns.to_string())
            .validate(|input: &String| match input.parse::<u32>() {
                Ok(value) => {
                    if value < 1 {
                        Err("Value must be at least 1")
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err("Please enter a valid number"),
            })
            .interact()?;

    let max_turns: u32 = max_turns_input.parse()?;
    config.set_param("GOOSE_MAX_TURNS", Value::from(max_turns))?;

    cliclack::outro(format!(
        "Set maximum turns to {} - Goose will ask for input after {} consecutive actions",
        max_turns, max_turns
    ))?;

    Ok(())
}

/// Handle OpenRouter authentication
pub async fn handle_openrouter_auth() -> Result<(), Box<dyn Error>> {
    use goose::config::{configure_openrouter, signup_openrouter::OpenRouterAuth};
    use goose::conversation::message::Message;
    use goose::providers::create;

    // Use the OpenRouter authentication flow
    let mut auth_flow = OpenRouterAuth::new()?;
    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            println!("\nAuthentication complete!");

            // Get config instance
            let config = Config::global();

            // Use the existing configure_openrouter function to set everything up
            println!("\nConfiguring OpenRouter...");
            if let Err(e) = configure_openrouter(config, api_key) {
                eprintln!("Failed to configure OpenRouter: {}", e);
                return Err(e.into());
            }

            println!("✓ OpenRouter configuration complete");
            println!("✓ Models configured successfully");

            // Test configuration - get the model that was configured
            println!("\nTesting configuration...");
            let configured_model: String = config.get_param("GOOSE_MODEL")?;
            let model_config = match goose::model::ModelConfig::new(&configured_model) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("⚠️  Invalid model configuration: {}", e);
                    eprintln!(
                        "Your settings have been saved. Please check your model configuration."
                    );
                    return Ok(());
                }
            };

            match create("openrouter", model_config) {
                Ok(provider) => {
                    // Simple test request
                    let test_result = provider
                        .complete(
                            "You are Goose, an AI assistant.",
                            &[Message::user().with_text("Say 'Configuration test successful!'")],
                            &[],
                        )
                        .await;

                    match test_result {
                        Ok(_) => {
                            println!("✓ Configuration test passed!");

                            // Enable the developer extension by default if not already enabled
                            let entries = ExtensionConfigManager::get_all()?;
                            let has_developer = entries
                                .iter()
                                .any(|e| e.config.name() == "developer" && e.enabled);

                            if !has_developer {
                                match ExtensionConfigManager::set(ExtensionEntry {
                                    enabled: true,
                                    config: ExtensionConfig::Builtin {
                                        name: "developer".to_string(),
                                        display_name: Some(
                                            goose::config::DEFAULT_DISPLAY_NAME.to_string(),
                                        ),
                                        timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
                                        bundled: Some(true),
                                        description: None,
                                    },
                                }) {
                                    Ok(_) => println!("✓ Developer extension enabled"),
                                    Err(e) => {
                                        eprintln!("⚠️  Failed to enable developer extension: {}", e)
                                    }
                                }
                            }

                            cliclack::outro("OpenRouter setup complete! You can now use Goose.")?;
                        }
                        Err(e) => {
                            eprintln!("⚠️  Configuration test failed: {}", e);
                            eprintln!("Your settings have been saved, but there may be an issue with the connection.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to create provider for testing: {}", e);
                    eprintln!("Your settings have been saved. Please check your configuration.");
                }
            }
        }
        Err(e) => {
            eprintln!("Authentication failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
