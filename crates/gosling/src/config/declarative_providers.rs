use crate::config::paths::Paths;
use crate::config::Config;
use crate::providers::anthropic_def::AnthropicProviderDef;
use crate::providers::base::{ModelInfo, ProviderType};
use crate::providers::inventory::declarative_inventory_identity;
use crate::providers::ollama_def::OllamaProviderDef;
use crate::providers::openai_def::OpenAiProviderDef;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use utoipa::ToSchema;

pub use gosling_providers::declarative::*;

static FIXED_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/providers/declarative");

pub fn custom_providers_dir() -> std::path::PathBuf {
    Paths::config_dir().join("custom_providers")
}

fn should_preserve_thinking_by_default(engine: &ProviderEngine) -> bool {
    matches!(engine, ProviderEngine::OpenAI)
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

    let normalized = display_name
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
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

pub fn validate_provider_id(id: &str) -> Result<()> {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow::anyhow!(
            "Invalid provider id: provider id cannot be empty"
        ));
    };

    if !(first.is_ascii_lowercase() || first.is_ascii_digit() || first == '_') {
        return Err(anyhow::anyhow!("Invalid provider id: {}", id));
    }

    if chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-') {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid provider id: {}", id))
    }
}

fn custom_provider_file_path(id: &str) -> Result<PathBuf> {
    if id.is_empty()
        || id
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
    {
        return Err(anyhow::anyhow!(
            "Invalid provider id: {}",
            if id.is_empty() { "<empty>" } else { id }
        ));
    }

    Ok(custom_providers_dir().join(format!("{}.json", id)))
}

pub fn generate_api_key_name(id: &str) -> String {
    format!("{}_API_KEY", id.to_uppercase())
}

#[derive(Debug, Clone)]
pub struct CreateCustomProviderParams {
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: Option<String>,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub requires_auth: bool,
    pub catalog_provider_id: Option<String>,
    pub base_path: Option<String>,
    pub preserves_thinking: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct UpdateCustomProviderParams {
    pub id: String,
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: Option<String>,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub requires_auth: bool,
    pub catalog_provider_id: Option<String>,
    pub base_path: Option<String>,
    pub preserves_thinking: Option<bool>,
}

pub fn create_custom_provider(
    params: CreateCustomProviderParams,
) -> Result<DeclarativeProviderConfig> {
    let id = generate_id(&params.display_name);
    validate_provider_id(&id)?;

    let api_key_env = if params.requires_auth {
        let api_key = params
            .api_key
            .as_deref()
            .filter(|api_key| !api_key.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("apiKey cannot be empty"))?;
        let api_key_name = generate_api_key_name(&id);
        let config = Config::global();
        config.set_secret(&api_key_name, &api_key)?;
        api_key_name
    } else {
        String::new()
    };

    let model_infos: Vec<ModelInfo> = params
        .models
        .into_iter()
        .map(|name| ModelInfo::new(name, 128000))
        .collect();

    let engine = ProviderEngine::from_str(&params.engine)?;
    let preserves_thinking = params
        .preserves_thinking
        .unwrap_or_else(|| should_preserve_thinking_by_default(&engine));

    let provider_config = DeclarativeProviderConfig {
        name: id.clone(),
        engine,
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
        model_doc_link: None,
        setup_steps: vec![],
        fast_model: None,
        preserves_thinking,
    };

    let custom_providers_dir = custom_providers_dir();
    std::fs::create_dir_all(&custom_providers_dir)?;

    let json_content = serde_json::to_string_pretty(&provider_config)?;
    let file_path = custom_providers_dir.join(format!("{}.json", id));
    crate::config::base::write_secrets_file(&file_path, &json_content)?;

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
        if let Some(api_key) = params.api_key.as_deref() {
            config.set_secret(&api_key_name, &api_key)?;
        } else if config.get_secret::<String>(&api_key_name).is_err() {
            return Err(anyhow::anyhow!(
                "apiKey is required when auth is enabled and no secret is stored"
            ));
        }
        api_key_name
    } else {
        if existing_config.api_key_env == generate_api_key_name(&params.id) {
            config.delete_secret(&existing_config.api_key_env)?;
        }
        String::new()
    };

    if editable {
        let model_infos: Vec<ModelInfo> = params
            .models
            .into_iter()
            .map(|name| ModelInfo::new(name, 128000))
            .collect();

        let engine = ProviderEngine::from_str(&params.engine)?;
        let preserves_thinking = match params.preserves_thinking {
            Some(value) => value,
            None if existing_config.engine != engine => {
                should_preserve_thinking_by_default(&engine)
            }
            None => existing_config.preserves_thinking,
        };

        let updated_config = DeclarativeProviderConfig {
            name: params.id.clone(),
            engine,
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
            model_doc_link: existing_config.model_doc_link,
            setup_steps: existing_config.setup_steps,
            fast_model: existing_config.fast_model.clone(),
            preserves_thinking,
        };

        let file_path = custom_provider_file_path(&updated_config.name)?;
        let json_content = serde_json::to_string_pretty(&updated_config)?;
        crate::config::base::write_secrets_file(&file_path, &json_content)?;
    }
    Ok(())
}

pub fn remove_custom_provider(id: &str) -> Result<()> {
    let config = Config::global();
    let loaded_provider = load_provider(id)?;
    let api_key_env = loaded_provider.config.api_key_env;
    if api_key_env == generate_api_key_name(id) {
        let _ = config.delete_secret(&api_key_env);
    }

    let file_path = custom_provider_file_path(id)?;

    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }

    Ok(())
}

/// Custom-provider JSON files live in a plain directory (`custom_providers/`)
/// that anything able to write into the user's config dir can populate -
/// not just this app's own create/update API, which always sets
/// `api_key_env` to `generate_api_key_name(id)`. Without this check, a
/// dropped/synced file naming an unrelated, already-configured secret (e.g.
/// `"api_key_env": "OPENAI_API_KEY"`) would have that real secret sent to
/// whatever `base_url` the file declares.
fn validate_custom_api_key_env(config: &DeclarativeProviderConfig) -> Result<()> {
    if config.api_key_env.is_empty() {
        return Ok(());
    }
    let expected = generate_api_key_name(&config.name);
    if config.api_key_env != expected {
        return Err(anyhow::anyhow!(
            "Custom provider {:?} declares api_key_env {:?}, expected {:?} - refusing to load a config that references a secret it doesn't own",
            config.name,
            config.api_key_env,
            expected
        ));
    }
    Ok(())
}

pub fn load_provider(id: &str) -> Result<LoadedProvider> {
    let custom_file_path = custom_provider_file_path(id)?;

    if custom_file_path.exists() {
        let content = std::fs::read_to_string(&custom_file_path)?;
        let config = deserialize_provider_config(&content)?;
        validate_custom_api_key_env(&config)?;
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
            let config = deserialize_provider_config(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;
            validate_custom_api_key_env(&config)
                .map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path.display(), e))?;
            Ok(config)
        })
        .collect()
}

fn deserialize_provider_config(content: &str) -> Result<DeclarativeProviderConfig> {
    let raw: serde_json::Value = serde_json::from_str(content)?;
    let preserves_thinking_was_set = raw.get("preserves_thinking").is_some();
    let mut config: DeclarativeProviderConfig = serde_json::from_value(raw)?;

    if !preserves_thinking_was_set {
        config.preserves_thinking = should_preserve_thinking_by_default(&config.engine);
    }

    Ok(config)
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

        match deserialize_provider_config(content) {
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
            let identity_config = config.clone();
            registry.register_with_name::<OpenAiProviderDef, _, _>(
                &config,
                provider_type,
                config.dynamic_models.unwrap_or(false),
                move |tls_config| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    crate::providers::openai_def::from_custom_config(cfg, tls_config)
                },
                move || {
                    let mut cfg = identity_config.clone();
                    resolve_config(&mut cfg)?;
                    declarative_inventory_identity(&cfg)
                },
            );
        }
        ProviderEngine::Ollama => {
            let captured = config.clone();
            let identity_config = config.clone();
            registry.register_with_name::<OllamaProviderDef, _, _>(
                &config,
                provider_type,
                config.dynamic_models.unwrap_or(false),
                move |tls_config| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    crate::providers::ollama_def::from_custom_config(cfg, tls_config)
                },
                move || {
                    let mut cfg = identity_config.clone();
                    resolve_config(&mut cfg)?;
                    declarative_inventory_identity(&cfg)
                },
            );
        }
        ProviderEngine::Anthropic => {
            let captured = config.clone();
            let identity_config = config.clone();
            registry.register_with_name::<AnthropicProviderDef, _, _>(
                &config,
                provider_type,
                config.dynamic_models.unwrap_or(false),
                move |tls_config| {
                    let mut cfg = captured.clone();
                    resolve_config(&mut cfg)?;
                    crate::providers::anthropic_def::from_custom_config(cfg, tls_config)
                },
                move || {
                    let mut cfg = identity_config.clone();
                    resolve_config(&mut cfg)?;
                    declarative_inventory_identity(&cfg)
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_existing_json_files_still_deserialize_without_new_fields() {
        let json = include_str!("../providers/declarative/groq.json");
        let config =
            deserialize_provider_config(json).expect("groq.json should parse without env_vars");
        assert!(config.env_vars.is_none());
        assert!(config.dynamic_models.is_none());
        assert!(config.model_doc_link.is_none());
        assert!(config.setup_steps.is_empty());
        assert!(config.preserves_thinking);
    }

    fn placeholder_var_names(template: &str) -> Vec<String> {
        template
            .split("${")
            .skip(1)
            .filter_map(|chunk| chunk.split_once('}'))
            .map(|(name, _)| name.to_string())
            .collect()
    }

    #[test]
    fn test_all_bundled_providers_are_valid() {
        let mut seen_ids = std::collections::HashSet::new();

        for file in FIXED_PROVIDERS.files() {
            if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let path = file.path().display().to_string();
            let content = file
                .contents_utf8()
                .unwrap_or_else(|| panic!("{path} is not valid UTF-8"));
            let config = deserialize_provider_config(content)
                .unwrap_or_else(|e| panic!("{path} failed to parse: {e}"));

            validate_provider_id(config.id())
                .unwrap_or_else(|e| panic!("{path} has an invalid provider id: {e}"));
            assert!(
                seen_ids.insert(config.id().to_string()),
                "{path} has a duplicate provider id: {}",
                config.id()
            );
            assert!(!config.base_url.is_empty(), "{path} has an empty base_url");

            if config.dynamic_models == Some(false) {
                assert!(
                    !config.models.is_empty(),
                    "{path} disables dynamic_models but lists no static models"
                );
            }

            let declared: std::collections::HashSet<&str> = config
                .env_vars
                .iter()
                .flatten()
                .map(|v| v.name.as_str())
                .collect();
            let templates = std::iter::once(config.base_url.as_str())
                .chain(config.base_path.as_deref())
                .chain(
                    config
                        .headers
                        .iter()
                        .flat_map(|h| h.values())
                        .map(String::as_str),
                );
            for template in templates {
                for var in placeholder_var_names(template) {
                    assert!(
                        declared.contains(var.as_str()),
                        "{path} references ${{{var}}} but declares no matching env_var"
                    );
                }
            }
        }

        assert!(!seen_ids.is_empty(), "no bundled providers were found");
    }

    #[test]
    fn test_bundled_providers_wire_into_registry_metadata() {
        let configs = load_fixed_providers().expect("bundled providers should load");
        assert!(!configs.is_empty(), "no bundled providers were found");

        for config in configs {
            let id = config.id().to_string();
            let api_key_env = config.api_key_env.clone();
            let requires_auth = config.requires_auth;
            let env_vars = config.env_vars.clone().unwrap_or_default();

            let mut registry = crate::providers::provider_registry::ProviderRegistry::new(None);
            register_declarative_provider(&mut registry, config, ProviderType::Declarative);

            let (meta, provider_type) = registry
                .all_metadata_with_types()
                .into_iter()
                .find(|(m, _)| m.name == id)
                .unwrap_or_else(|| panic!("{id} should register"));

            assert_eq!(provider_type, ProviderType::Declarative, "{id}");
            assert!(!meta.display_name.is_empty(), "{id} has empty display_name");

            assert!(
                !meta
                    .config_keys
                    .iter()
                    .any(|k| k.name == "OPENAI_HOST" || k.name == "OPENAI_BASE_PATH"),
                "{id} leaks OpenAI engine config keys"
            );

            if !api_key_env.is_empty() {
                let key = meta
                    .config_keys
                    .iter()
                    .find(|k| k.name == api_key_env)
                    .unwrap_or_else(|| panic!("{id} should expose {api_key_env} config key"));
                assert!(key.secret, "{id}: {api_key_env} should be secret");
                assert_eq!(key.required, requires_auth, "{id}: {api_key_env} required");
            }

            for ev in &env_vars {
                let key = meta
                    .config_keys
                    .iter()
                    .find(|k| k.name == ev.name)
                    .unwrap_or_else(|| panic!("{id} should expose {} config key", ev.name));
                assert_eq!(key.required, ev.required, "{id}: {} required", ev.name);
                assert_eq!(key.secret, ev.secret, "{id}: {} secret", ev.name);
            }
        }
    }

    #[test]
    fn test_custom_openai_provider_missing_preserves_thinking_defaults_true() {
        let json = r#"{
            "name": "custom_reasoning",
            "engine": "openai",
            "display_name": "Custom Reasoning",
            "description": null,
            "api_key_env": "",
            "base_url": "https://example.com/v1",
            "models": [{"name": "reasoning-model", "context_limit": 128000}],
            "headers": null,
            "timeout_seconds": null,
            "supports_streaming": true,
            "requires_auth": false
        }"#;

        let config = deserialize_provider_config(json).expect("custom provider json should parse");

        assert!(matches!(config.engine, ProviderEngine::OpenAI));
        assert!(config.preserves_thinking);
    }

    #[test]
    fn test_custom_provider_explicit_preserves_thinking_false_is_kept() {
        let json = r#"{
            "name": "custom_strict",
            "engine": "openai",
            "display_name": "Custom Strict",
            "description": null,
            "api_key_env": "",
            "base_url": "https://example.com/v1",
            "models": [{"name": "strict-model", "context_limit": 128000}],
            "headers": null,
            "timeout_seconds": null,
            "supports_streaming": true,
            "requires_auth": false,
            "preserves_thinking": false
        }"#;

        let config = deserialize_provider_config(json).expect("custom provider json should parse");

        assert!(matches!(config.engine, ProviderEngine::OpenAI));
        assert!(!config.preserves_thinking);
    }

    #[test]
    fn test_validate_provider_id_rejects_legacy_punctuation_for_new_ids() {
        assert!(validate_provider_id("custom_z.ai").is_err());
    }

    fn write_legacy_provider_config(id: &str, display_name: &str) {
        let custom_dir = custom_providers_dir();
        std::fs::create_dir_all(&custom_dir).unwrap();
        let content = format!(
            r#"{{
  "name": "{id}",
  "engine": "openai",
  "display_name": "{display_name}",
  "description": "legacy provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [],
  "requires_auth": false
}}"#
        );
        std::fs::write(custom_dir.join(format!("{id}.json")), content).unwrap();
    }

    #[test]
    fn test_load_provider_allows_legacy_custom_id_with_punctuation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = temp_dir.path().display().to_string();
        let _guard = env_lock::lock_env([("GOSLING_PATH_ROOT", Some(temp_root.as_str()))]);

        write_legacy_provider_config("custom_z.ai", "Z.AI");

        let loaded = load_provider("custom_z.ai").unwrap();
        assert!(loaded.is_editable);
        assert_eq!(loaded.config.name, "custom_z.ai");
    }

    #[test]
    fn test_update_and_remove_provider_allow_legacy_custom_id_with_punctuation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = temp_dir.path().display().to_string();
        let _guard = env_lock::lock_env([("GOSLING_PATH_ROOT", Some(temp_root.as_str()))]);

        write_legacy_provider_config("custom_z.ai", "Z.AI");

        update_custom_provider(UpdateCustomProviderParams {
            id: "custom_z.ai".to_string(),
            engine: "openai".to_string(),
            display_name: "Z.AI Updated".to_string(),
            api_url: "https://updated.example.invalid/v1/chat/completions".to_string(),
            api_key: None,
            models: vec!["z-model".to_string()],
            supports_streaming: Some(true),
            headers: None,
            requires_auth: false,
            catalog_provider_id: None,
            base_path: None,
            preserves_thinking: None,
        })
        .unwrap();

        let updated = load_provider("custom_z.ai").unwrap();
        assert_eq!(updated.config.display_name, "Z.AI Updated");
        assert_eq!(updated.config.models[0].name, "z-model");

        remove_custom_provider("custom_z.ai").unwrap();
        assert!(!custom_providers_dir().join("custom_z.ai.json").exists());
    }

    #[test]
    fn test_load_provider_rejects_path_segments() {
        assert!(load_provider("custom_../secret").is_err());
        assert!(load_provider("custom_..\\secret").is_err());
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
}
