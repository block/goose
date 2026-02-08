use crate::config::paths::Paths;
use crate::config::Config;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::api_client::AuthMethod;
use crate::providers::base::{ModelInfo, ProviderType};
use crate::providers::dynamic_auth::{AuthHeaderStyle, CommandAuthProvider, FileAuthProvider};
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use utoipa::ToSchema;

static FIXED_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/providers/declarative");

pub fn custom_providers_dir() -> std::path::PathBuf {
    Paths::config_dir().join("custom_providers")
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    OpenAI,
    Ollama,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeclarativeProviderConfig {
    pub name: String,
    pub engine: ProviderEngine,
    pub display_name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub supports_streaming: Option<bool>,
    #[serde(default = "default_requires_auth")]
    pub requires_auth: bool,
    #[serde(default)]
    pub api_key_command: Option<String>,
    #[serde(default)]
    pub api_key_file: Option<String>,
    #[serde(default)]
    pub api_key_file_field: Option<String>,
}

fn default_requires_auth() -> bool {
    true
}

impl DeclarativeProviderConfig {
    pub fn id(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Resolve authentication method using priority:
    /// `api_key_command` > `api_key_file` > `api_key_env` > `NoAuth`
    pub fn resolve_auth_method(&self, engine: &ProviderEngine) -> Result<AuthMethod> {
        let header_style = match engine {
            ProviderEngine::Anthropic => AuthHeaderStyle::CustomHeader {
                header_name: "x-api-key".to_string(),
            },
            _ => AuthHeaderStyle::BearerToken,
        };

        if let Some(ref command) = self.api_key_command {
            let provider = CommandAuthProvider::new(command.clone(), header_style);
            return Ok(AuthMethod::Custom(Box::new(provider)));
        }

        if let Some(ref file_path) = self.api_key_file {
            let provider = FileAuthProvider::new(
                file_path.clone(),
                self.api_key_file_field.clone(),
                header_style,
            );
            return Ok(AuthMethod::Custom(Box::new(provider)));
        }

        if self.requires_auth && !self.api_key_env.is_empty() {
            let global_config = Config::global();
            if let Ok(key) = global_config.get_secret::<String>(&self.api_key_env) {
                if !key.is_empty() {
                    return Ok(match engine {
                        ProviderEngine::Anthropic => AuthMethod::ApiKey {
                            header_name: "x-api-key".to_string(),
                            key,
                        },
                        _ => AuthMethod::BearerToken(key),
                    });
                }
            }
        }

        Ok(AuthMethod::NoAuth)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoadedProvider {
    pub config: DeclarativeProviderConfig,
    pub is_editable: bool,
}

static ID_GENERATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn generate_id(display_name: &str) -> String {
    let _guard = ID_GENERATION_LOCK.lock().unwrap();

    let normalized = display_name.to_lowercase().replace(' ', "_");
    let base_id = format!("custom_{}", normalized);

    let custom_dir = custom_providers_dir();
    let mut candidate_id = base_id.clone();
    let mut counter = 1;

    while custom_dir.join(format!("{}.json", candidate_id)).exists() {
        candidate_id = format!("{}_{}", base_id, counter);
        counter += 1;
    }

    candidate_id
}

pub fn generate_api_key_name(id: &str) -> String {
    format!("{}_API_KEY", id.to_uppercase())
}

#[derive(Debug, Clone)]
pub struct CreateCustomProviderParams {
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub requires_auth: bool,
    pub api_key_command: Option<String>,
    pub api_key_file: Option<String>,
    pub api_key_file_field: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCustomProviderParams {
    pub id: String,
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub requires_auth: bool,
    pub api_key_command: Option<String>,
    pub api_key_file: Option<String>,
    pub api_key_file_field: Option<String>,
}

pub fn create_custom_provider(
    params: CreateCustomProviderParams,
) -> Result<DeclarativeProviderConfig> {
    let id = generate_id(&params.display_name);

    let api_key_env = if params.requires_auth {
        let api_key_name = generate_api_key_name(&id);
        let config = Config::global();
        config.set_secret(&api_key_name, &params.api_key)?;
        api_key_name
    } else {
        String::new()
    };

    let model_infos: Vec<ModelInfo> = params
        .models
        .into_iter()
        .map(|name| ModelInfo::new(name, 128000))
        .collect();

    let provider_config = DeclarativeProviderConfig {
        name: id.clone(),
        engine: match params.engine.as_str() {
            "openai_compatible" => ProviderEngine::OpenAI,
            "anthropic_compatible" => ProviderEngine::Anthropic,
            "ollama_compatible" => ProviderEngine::Ollama,
            _ => return Err(anyhow::anyhow!("Invalid provider type: {}", params.engine)),
        },
        display_name: params.display_name.clone(),
        description: Some(format!("Custom {} provider", params.display_name)),
        api_key_env,
        base_url: params.api_url,
        models: model_infos,
        headers: params.headers,
        timeout_seconds: None,
        supports_streaming: params.supports_streaming,
        requires_auth: params.requires_auth,
        api_key_command: params.api_key_command,
        api_key_file: params.api_key_file,
        api_key_file_field: params.api_key_file_field,
    };

    let custom_providers_dir = custom_providers_dir();
    std::fs::create_dir_all(&custom_providers_dir)?;

    let json_content = serde_json::to_string_pretty(&provider_config)?;
    let file_path = custom_providers_dir.join(format!("{}.json", id));
    std::fs::write(file_path, json_content)?;

    Ok(provider_config)
}

pub fn update_custom_provider(params: UpdateCustomProviderParams) -> Result<()> {
    let loaded_provider = load_provider(&params.id)?;
    let existing_config = loaded_provider.config;
    let editable = loaded_provider.is_editable;

    let config = Config::global();

    let api_key_env = if params.requires_auth {
        let api_key_name = if existing_config.api_key_env.is_empty() {
            generate_api_key_name(&params.id)
        } else {
            existing_config.api_key_env.clone()
        };
        if !params.api_key.is_empty() {
            config.set_secret(&api_key_name, &params.api_key)?;
        }
        api_key_name
    } else {
        String::new()
    };

    if editable {
        let model_infos: Vec<ModelInfo> = params
            .models
            .into_iter()
            .map(|name| ModelInfo::new(name, 128000))
            .collect();

        let updated_config = DeclarativeProviderConfig {
            name: params.id.clone(),
            engine: match params.engine.as_str() {
                "openai_compatible" => ProviderEngine::OpenAI,
                "anthropic_compatible" => ProviderEngine::Anthropic,
                "ollama_compatible" => ProviderEngine::Ollama,
                _ => return Err(anyhow::anyhow!("Invalid provider type: {}", params.engine)),
            },
            display_name: params.display_name,
            description: existing_config.description,
            api_key_env,
            base_url: params.api_url,
            models: model_infos,
            headers: params.headers.or(existing_config.headers),
            timeout_seconds: existing_config.timeout_seconds,
            supports_streaming: params.supports_streaming,
            requires_auth: params.requires_auth,
            api_key_command: params.api_key_command.or(existing_config.api_key_command),
            api_key_file: params.api_key_file.or(existing_config.api_key_file),
            api_key_file_field: params.api_key_file_field.or(existing_config.api_key_file_field),
        };

        let file_path = custom_providers_dir().join(format!("{}.json", updated_config.name));
        let json_content = serde_json::to_string_pretty(&updated_config)?;
        std::fs::write(file_path, json_content)?;
    }
    Ok(())
}

pub fn remove_custom_provider(id: &str) -> Result<()> {
    let config = Config::global();
    let api_key_name = generate_api_key_name(id);
    let _ = config.delete_secret(&api_key_name);

    let custom_providers_dir = custom_providers_dir();
    let file_path = custom_providers_dir.join(format!("{}.json", id));

    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }

    Ok(())
}

pub fn load_provider(id: &str) -> Result<LoadedProvider> {
    let custom_file_path = custom_providers_dir().join(format!("{}.json", id));

    if custom_file_path.exists() {
        let content = std::fs::read_to_string(&custom_file_path)?;
        let config: DeclarativeProviderConfig = serde_json::from_str(&content)?;
        return Ok(LoadedProvider {
            config,
            is_editable: true,
        });
    }

    for file in FIXED_PROVIDERS.files() {
        if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = file
            .contents_utf8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read file as UTF-8: {:?}", file.path()))?;

        let config: DeclarativeProviderConfig = serde_json::from_str(content)?;
        if config.name == id {
            return Ok(LoadedProvider {
                config,
                is_editable: false,
            });
        }
    }

    Err(anyhow::anyhow!("Provider not found: {}", id))
}
pub fn load_custom_providers(dir: &Path) -> Result<Vec<DeclarativeProviderConfig>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .map(|path| {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))
        })
        .collect()
}

fn load_fixed_providers() -> Result<Vec<DeclarativeProviderConfig>> {
    let mut res = Vec::new();
    for file in FIXED_PROVIDERS.files() {
        if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = file
            .contents_utf8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read file as UTF-8: {:?}", file.path()))?;

        let config: DeclarativeProviderConfig = serde_json::from_str(content)?;
        res.push(config)
    }

    Ok(res)
}

pub fn register_declarative_providers(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
) -> Result<()> {
    let dir = custom_providers_dir();
    let custom_providers = load_custom_providers(&dir)?;
    let fixed_providers = load_fixed_providers()?;
    for config in fixed_providers {
        register_declarative_provider(registry, config, ProviderType::Declarative);
    }

    for config in custom_providers {
        register_declarative_provider(registry, config, ProviderType::Custom);
    }

    Ok(())
}

pub fn register_declarative_provider(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    config: DeclarativeProviderConfig,
    provider_type: ProviderType,
) {
    let config_clone = config.clone();

    match config.engine {
        ProviderEngine::OpenAI => {
            registry.register_with_name::<OpenAiProvider, _>(
                &config,
                provider_type,
                move |model| OpenAiProvider::from_custom_config(model, config_clone.clone()),
            );
        }
        ProviderEngine::Ollama => {
            registry.register_with_name::<OllamaProvider, _>(
                &config,
                provider_type,
                move |model| OllamaProvider::from_custom_config(model, config_clone.clone()),
            );
        }
        ProviderEngine::Anthropic => {
            registry.register_with_name::<AnthropicProvider, _>(
                &config,
                provider_type,
                move |model| AnthropicProvider::from_custom_config(model, config_clone.clone()),
            );
        }
    }
}
