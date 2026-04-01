use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, PermissionMapping,
    ACP_CURRENT_MODEL,
};
use crate::config::paths::Paths;
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::base::{ModelInfo, ProviderMetadata, ProviderType};
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use crate::providers::provider_registry::ProviderEntry;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;

static FIXED_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/providers/declarative");
static FIXED_ACP_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/providers/declarative_acp");

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
pub struct EnvVarConfig {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
    /// When true, the field is shown prominently in the UI (not collapsed).
    /// Defaults to the value of `required` if not specified.
    pub primary: Option<bool>,
    pub description: Option<String>,
    pub default: Option<String>,
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
    pub catalog_provider_id: Option<String>,
    #[serde(default)]
    pub base_path: Option<String>,
    #[serde(default)]
    pub env_vars: Option<Vec<EnvVarConfig>>,
    #[serde(default)]
    pub dynamic_models: Option<bool>,
    #[serde(default)]
    pub skip_canonical_filtering: bool,
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
}

/// Expand `${VAR_NAME}` placeholders in a template string using the given env var configs.
/// Resolves values via Config (secret if `secret`, param otherwise), falls back to `default`.
/// Returns an error if a `required` var is missing.
pub fn expand_env_vars(template: &str, env_vars: &[EnvVarConfig]) -> Result<String> {
    let config = Config::global();
    let mut result = template.to_string();
    for var in env_vars {
        let placeholder = format!("${{{}}}", var.name);
        if !result.contains(&placeholder) {
            continue;
        }
        let value = if var.secret {
            config.get_secret::<String>(&var.name).ok()
        } else {
            config.get_param::<String>(&var.name).ok()
        };
        let value = match value {
            Some(v) => v,
            None => match &var.default {
                Some(d) => d.clone(),
                None if var.required => {
                    return Err(anyhow::anyhow!(
                        "Required environment variable {} is not set",
                        var.name
                    ));
                }
                None => continue,
            },
        };
        result = result.replace(&placeholder, &value);
    }
    Ok(result)
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
    pub catalog_provider_id: Option<String>,
    pub base_path: Option<String>,
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
    pub catalog_provider_id: Option<String>,
    pub base_path: Option<String>,
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
        catalog_provider_id: params.catalog_provider_id,
        base_path: params.base_path,
        env_vars: None,
        dynamic_models: None,
        skip_canonical_filtering: false,
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
            headers: match params.headers {
                Some(h) if h.is_empty() => None,
                Some(h) => Some(h),
                None => existing_config.headers,
            },
            timeout_seconds: existing_config.timeout_seconds,
            supports_streaming: params.supports_streaming,
            requires_auth: params.requires_auth,
            catalog_provider_id: params.catalog_provider_id,
            base_path: params.base_path,
            env_vars: existing_config.env_vars,
            dynamic_models: existing_config.dynamic_models,
            skip_canonical_filtering: existing_config.skip_canonical_filtering,
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

        let config: DeclarativeProviderConfig = match serde_json::from_str(content) {
            Ok(config) => config,
            Err(_) => continue,
        };
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

        match serde_json::from_str(content) {
            Ok(config) => res.push(config),
            Err(e) => {
                tracing::warn!(
                    "Skipping invalid declarative provider {:?}: {}",
                    file.path(),
                    e
                );
            }
        }
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

/// Resolve `${VAR}` placeholders in the config's `base_url` and apply
/// runtime overrides from env_vars. Called lazily (at provider instantiation)
/// so values configured through the UI after startup are picked up.
fn resolve_config(config: &mut DeclarativeProviderConfig) -> Result<()> {
    if let Some(ref env_vars) = config.env_vars {
        config.base_url = expand_env_vars(&config.base_url, env_vars)?;

        // Check for streaming override via env_vars.
        // Config/env may store the value as a string ("true") or a native bool,
        // so try String first, then fall back to bool.
        let global_config = Config::global();
        for var in env_vars {
            if var.name.ends_with("_STREAMING") {
                let val: Option<bool> = global_config
                    .get_param::<String>(&var.name)
                    .ok()
                    .map(|s| s.to_lowercase() == "true")
                    .or_else(|| global_config.get_param::<bool>(&var.name).ok())
                    .or_else(|| var.default.as_deref().map(|d| d.to_lowercase() == "true"));
                if let Some(v) = val {
                    config.supports_streaming = Some(v);
                }
            }
        }
    }
    Ok(())
}

pub fn register_declarative_provider(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    config: DeclarativeProviderConfig,
    provider_type: ProviderType,
) {
    // Each closure needs its own owned copy of config because closures are
    // moved into the registry and may be invoked much later than registration.
    // Env var expansion happens lazily inside resolve_base_url so that values
    // configured through the UI after startup are picked up.
    match config.engine {
        ProviderEngine::OpenAI => {
            let captured = config.clone();
            registry.register_with_name::<OpenAiProvider, _>(
                &config,
                provider_type,
                move |model| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    OpenAiProvider::from_custom_config(model, cfg)
                },
            );
        }
        ProviderEngine::Ollama => {
            let captured = config.clone();
            registry.register_with_name::<OllamaProvider, _>(
                &config,
                provider_type,
                move |model| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    OllamaProvider::from_custom_config(model, cfg)
                },
            );
        }
        ProviderEngine::Anthropic => {
            let captured = config.clone();
            registry.register_with_name::<AnthropicProvider, _>(
                &config,
                provider_type,
                move |model| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    AnthropicProvider::from_custom_config(model, cfg)
                },
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeAcpConfig {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub doc_url: String,
    pub binary: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_remove: Vec<String>,
    pub modes: HashMap<String, String>,
    #[serde(default)]
    pub allow_option_id: Option<String>,
    #[serde(default)]
    pub reject_option_id: Option<String>,
    #[serde(default)]
    pub setup_steps: Vec<String>,
}

pub fn register_declarative_acp_providers(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
) -> Result<()> {
    for file in FIXED_ACP_PROVIDERS.files() {
        if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let content = file
            .contents_utf8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read file as UTF-8: {:?}", file.path()))?;
        match serde_json::from_str::<DeclarativeAcpConfig>(content) {
            Ok(config) => register_declarative_acp_provider(registry, config),
            Err(e) => {
                tracing::warn!(
                    "Skipping invalid declarative ACP provider {:?}: {}",
                    file.path(),
                    e
                );
            }
        }
    }
    Ok(())
}

fn register_declarative_acp_provider(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    config: DeclarativeAcpConfig,
) {
    let metadata = ProviderMetadata::new(
        &config.name,
        &config.display_name,
        &config.description,
        ACP_CURRENT_MODEL,
        vec![],
        &config.doc_url,
        vec![],
    )
    .with_setup_steps(config.setup_steps.iter().map(|s| s.as_str()).collect());

    let name = config.name.clone();
    registry.entries.insert(
        name,
        ProviderEntry::new(
            metadata,
            Arc::new(move |model, extensions| {
                let config = config.clone();
                Box::pin(async move {
                    let resolved_command =
                        SearchPaths::builder().with_npm().resolve(&config.binary)?;

                    let goose_mode = Config::global().get_goose_mode().unwrap_or(GooseMode::Auto);

                    let mode_mapping: HashMap<GooseMode, String> = config
                        .modes
                        .iter()
                        .filter_map(|(k, v)| {
                            GooseMode::from_str(k).ok().map(|mode| (mode, v.clone()))
                        })
                        .collect();

                    let permission_mapping = PermissionMapping {
                        allow_option_id: config.allow_option_id.clone(),
                        reject_option_id: config.reject_option_id.clone(),
                        rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
                    };

                    let provider_config = AcpProviderConfig {
                        command: resolved_command,
                        args: config.args.clone(),
                        env: vec![],
                        env_remove: config.env_remove.clone(),
                        work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                        mcp_servers: extension_configs_to_mcp_servers(&extensions),
                        session_mode_id: mode_mapping.get(&goose_mode).cloned(),
                        mode_mapping,
                        permission_mapping,
                        notification_callback: None,
                    };

                    let provider = AcpProvider::connect(
                        config.name.clone(),
                        model,
                        goose_mode,
                        provider_config,
                    )
                    .await?;
                    Ok(Arc::new(provider) as Arc<dyn crate::providers::base::Provider>)
                })
            }),
            ProviderType::Builtin,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_tanzu_json_deserializes() {
        let json = include_str!("../providers/declarative/tanzu.json");
        let config: DeclarativeProviderConfig =
            serde_json::from_str(json).expect("tanzu.json should parse");
        assert_eq!(config.name, "tanzu_ai");
        assert_eq!(config.display_name, "VMware Tanzu Platform");
        assert!(matches!(config.engine, ProviderEngine::OpenAI));
        assert_eq!(config.api_key_env, "TANZU_AI_API_KEY");
        assert_eq!(
            config.base_url,
            "${TANZU_AI_ENDPOINT}/openai/v1/chat/completions"
        );
        assert_eq!(config.dynamic_models, Some(true));
        assert_eq!(config.supports_streaming, Some(true));

        let env_vars = config.env_vars.as_ref().expect("env_vars should be set");
        assert_eq!(env_vars.len(), 2);
        assert_eq!(env_vars[0].name, "TANZU_AI_ENDPOINT");
        assert!(env_vars[0].required);
        assert!(!env_vars[0].secret);
        assert_eq!(env_vars[1].name, "TANZU_AI_STREAMING");
        assert!(!env_vars[1].required);
        assert_eq!(env_vars[1].default, Some("true".to_string()));

        assert_eq!(config.models.len(), 1);
        assert_eq!(config.models[0].name, "openai/gpt-oss-120b");
    }

    #[test]
    fn test_existing_json_files_still_deserialize_without_new_fields() {
        let json = include_str!("../providers/declarative/groq.json");
        let config: DeclarativeProviderConfig =
            serde_json::from_str(json).expect("groq.json should parse without env_vars");
        assert!(config.env_vars.is_none());
        assert!(config.dynamic_models.is_none());
    }

    #[test]
    fn test_expand_env_vars_replaces_placeholder() {
        let _guard = env_lock::lock_env([("TEST_EXPAND_HOST", Some("https://example.com/api"))]);

        let env_vars = vec![EnvVarConfig {
            name: "TEST_EXPAND_HOST".to_string(),
            required: true,
            secret: false,
            primary: None,
            description: None,
            default: None,
        }];

        let result = expand_env_vars("${TEST_EXPAND_HOST}/v1/chat/completions", &env_vars).unwrap();
        assert_eq!(result, "https://example.com/api/v1/chat/completions");
    }

    #[test]
    fn test_expand_env_vars_required_missing_errors() {
        let _guard = env_lock::lock_env([("TEST_EXPAND_MISSING", None::<&str>)]);

        let env_vars = vec![EnvVarConfig {
            name: "TEST_EXPAND_MISSING".to_string(),
            required: true,
            secret: false,
            primary: None,
            description: None,
            default: None,
        }];

        let result = expand_env_vars("${TEST_EXPAND_MISSING}/path", &env_vars);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("TEST_EXPAND_MISSING"));
    }

    #[test]
    fn test_expand_env_vars_uses_default_when_missing() {
        let _guard = env_lock::lock_env([("TEST_EXPAND_DEFAULT", None::<&str>)]);

        let env_vars = vec![EnvVarConfig {
            name: "TEST_EXPAND_DEFAULT".to_string(),
            required: false,
            secret: false,
            primary: None,
            description: None,
            default: Some("https://fallback.example.com".to_string()),
        }];

        let result =
            expand_env_vars("${TEST_EXPAND_DEFAULT}/v1/chat/completions", &env_vars).unwrap();
        assert_eq!(result, "https://fallback.example.com/v1/chat/completions");
    }

    #[test]
    fn test_expand_env_vars_no_placeholders_passthrough() {
        let env_vars = vec![EnvVarConfig {
            name: "UNUSED_VAR".to_string(),
            required: true,
            secret: false,
            primary: None,
            description: None,
            default: None,
        }];

        let result =
            expand_env_vars("https://static.example.com/v1/chat/completions", &env_vars).unwrap();
        assert_eq!(result, "https://static.example.com/v1/chat/completions");
    }

    #[test]
    fn test_expand_env_vars_empty_slice_passthrough() {
        let result = expand_env_vars("${WHATEVER}/path", &[]).unwrap();
        assert_eq!(result, "${WHATEVER}/path");
    }

    #[test]
    fn test_expand_env_vars_env_value_overrides_default() {
        let _guard = env_lock::lock_env([("TEST_EXPAND_OVERRIDE", Some("https://from-env.com"))]);

        let env_vars = vec![EnvVarConfig {
            name: "TEST_EXPAND_OVERRIDE".to_string(),
            required: false,
            secret: false,
            primary: None,
            description: None,
            default: Some("https://from-default.com".to_string()),
        }];

        let result = expand_env_vars("${TEST_EXPAND_OVERRIDE}/path", &env_vars).unwrap();
        assert_eq!(result, "https://from-env.com/path");
    }

    #[test_case(
        include_str!("../providers/declarative_acp/copilot_acp.json"),
        "copilot-acp", "copilot", &["--acp"], &[], None, None;
        "copilot_acp"
    )]
    #[test_case(
        include_str!("../providers/declarative_acp/claude_acp.json"),
        "claude-acp", "claude-agent-acp", &[], &["CLAUDECODE"], Some("allow"), Some("reject");
        "claude_acp"
    )]
    #[test_case(
        include_str!("../providers/declarative_acp/codex_acp.json"),
        "codex-acp", "codex-acp", &["-c", "sandbox_workspace_write.network_access=true"], &[], Some("approved"), Some("abort");
        "codex_acp"
    )]
    fn test_acp_json_deserializes(
        json: &str,
        name: &str,
        binary: &str,
        args: &[&str],
        env_remove: &[&str],
        allow_id: Option<&str>,
        reject_id: Option<&str>,
    ) {
        let config: DeclarativeAcpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, name);
        assert_eq!(config.binary, binary);
        assert_eq!(config.args, args);
        assert_eq!(config.env_remove, env_remove);
        assert_eq!(config.allow_option_id.as_deref(), allow_id);
        assert_eq!(config.reject_option_id.as_deref(), reject_id);
        assert_eq!(config.modes.len(), 4);
    }
}
