use crate::routes::errors::ErrorResponse;
use crate::routes::utils::check_provider_configured;
use crate::state::AppState;
use axum::routing::put;
use axum::{
    extract::Path,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, TimeZone, Utc};
use goose::config::declarative_providers::LoadedProvider;
use goose::config::paths::Paths;
use goose::config::ExtensionEntry;
use goose::config::{Config, ConfigError};
use goose::custom_requests::SourceType;
use goose::providers::base::{ModelInfo, ProviderMetadata, ProviderType};
use goose::providers::canonical::maybe_get_canonical_model;
use goose::providers::catalog::{
    get_provider_template, get_providers_by_format, ProviderCatalogEntry, ProviderFormat,
    ProviderTemplate,
};
use goose::providers::create_with_default_model;
use goose::providers::huggingface_auth;
use goose::providers::providers as get_providers;
use goose::{
    agents::execute_commands, agents::ExtensionConfig, config::permission::PermissionLevel,
    slash_commands::recipe_slash_command,
};
use goose_providers::model::ModelConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ExtensionResponse {
    pub extensions: Vec<ExtensionEntry>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtensionQuery {
    pub name: String,
    pub config: ExtensionConfig,
    pub enabled: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct UpsertConfigQuery {
    pub key: String,
    pub value: Value,
    pub is_secret: bool,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ConfigKeyQuery {
    pub key: String,
    pub is_secret: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderDetails {
    pub name: String,
    pub metadata: ProviderMetadata,
    pub is_configured: bool,
    pub provider_type: ProviderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_model: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderDetails>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ToolPermission {
    pub tool_name: String,
    pub permission: PermissionLevel,
}

#[derive(Deserialize, ToSchema)]
pub struct UpsertPermissionsQuery {
    pub tool_permissions: Vec<ToolPermission>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCustomProviderRequest {
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default = "default_requires_auth")]
    pub requires_auth: bool,
    #[serde(default)]
    pub catalog_provider_id: Option<String>,
    #[serde(default)]
    pub base_path: Option<String>,
    #[serde(default)]
    pub preserves_thinking: Option<bool>,
}

fn default_requires_auth() -> bool {
    true
}

fn normalize_custom_provider_api_key(api_key: String) -> Option<String> {
    let api_key = api_key.trim().to_string();
    (!api_key.is_empty()).then_some(api_key)
}

#[derive(Deserialize, ToSchema)]
pub struct CheckProviderRequest {
    pub provider: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SetProviderRequest {
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaskedSecret {
    pub masked_value: String,
}

#[derive(Serialize, ToSchema)]
#[serde(untagged)]
pub enum ConfigValueResponse {
    Value(Value),
    MaskedValue(MaskedSecret),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSecretStorage {
    SecretStore,
    ProviderCache,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSecretStatus {
    Valid,
    Expired,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderSecret {
    pub id: String,
    pub provider: String,
    pub provider_display_name: String,
    pub name: String,
    pub storage: ProviderSecretStorage,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: ProviderSecretStatus,
    pub configured: bool,
    pub has_secret: bool,
    pub can_delete: bool,
    pub can_configure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configure_provider: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderSecretsResponse {
    pub secrets: Vec<ProviderSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum CommandType {
    Builtin,
    Recipe,
    Skill,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlashCommand {
    pub command: String,
    pub help: String,
    pub command_type: CommandType,
}
#[derive(Serialize, ToSchema)]
pub struct SlashCommandsResponse {
    pub commands: Vec<SlashCommand>,
}

#[utoipa::path(
    post,
    path = "/config/upsert",
    request_body = UpsertConfigQuery,
    responses(
        (status = 200, description = "Configuration value upserted successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upsert_config(
    Json(query): Json<UpsertConfigQuery>,
) -> Result<Json<Value>, ErrorResponse> {
    let config = Config::global();

    // Intercept legacy keys to write structured provider config
    if query.key == "GOOSE_PROVIDER" {
        if let Some(name) = query.value.as_str() {
            // Preserve the target provider's saved model rather than copying
            // the current active provider's model into the new entry.
            let model = goose::config::get_provider_entry(config, name)
                .map(|e| e.model)
                .or_else(|| config.get_goose_model().ok())
                .unwrap_or_default();
            goose::config::set_active_provider(config, name, &model)?;
            return Ok(Json(Value::String(format!("Upserted key {}", query.key))));
        }
    }
    if query.key == "GOOSE_MODEL" {
        if let Some(model) = query.value.as_str() {
            if let Ok(provider) = config.get_goose_provider() {
                goose::config::set_active_provider(config, &provider, model)?;
                return Ok(Json(Value::String(format!("Upserted key {}", query.key))));
            }
        }
    }

    config.set(&query.key, &query.value, query.is_secret)?;
    Ok(Json(Value::String(format!("Upserted key {}", query.key))))
}

#[utoipa::path(
    post,
    path = "/config/remove",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value removed successfully", body = String),
        (status = 404, description = "Configuration key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_config(
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, ErrorResponse> {
    let config = Config::global();

    if query.is_secret {
        config.delete_secret(&query.key)?;
    } else if query.key == "GOOSE_PROVIDER" || query.key == "active_provider" {
        config.delete("active_provider")?;
        config.delete("GOOSE_PROVIDER")?;
    } else if query.key == "GOOSE_MODEL" {
        if let Ok(provider) = config.get_goose_provider() {
            goose::config::set_active_provider(config, &provider, "")?;
        }
        config.delete("GOOSE_MODEL")?;
    } else {
        config.delete(&query.key)?;
    }

    Ok(Json(format!("Removed key {}", query.key)))
}

const SECRET_MASK_SHOW_LEN: usize = 8;

fn mask_secret(secret: Value) -> String {
    let as_string = match secret {
        Value::String(s) => s,
        _ => serde_json::to_string(&secret).unwrap_or_else(|_| secret.to_string()),
    };

    let chars: Vec<_> = as_string.chars().collect();
    let show_len = std::cmp::min(chars.len() / 2, SECRET_MASK_SHOW_LEN);
    let visible: String = chars.iter().take(show_len).collect();
    let mask = "*".repeat(chars.len() - show_len);

    format!("{}{}", visible, mask)
}

const SECRET_STORE_ID_PREFIX: &str = "secret_store:";
const PROVIDER_CACHE_ID_PREFIX: &str = "provider_cache:";

fn provider_secret_status(expires_at: Option<DateTime<Utc>>) -> ProviderSecretStatus {
    match expires_at {
        Some(expires_at) if expires_at <= Utc::now() => ProviderSecretStatus::Expired,
        Some(_) => ProviderSecretStatus::Valid,
        None => ProviderSecretStatus::Unknown,
    }
}

fn parse_expiry_value(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(value) => DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| dt.with_timezone(&Utc)),
        Value::Number(value) => value
            .as_i64()
            .and_then(|timestamp| Utc.timestamp_opt(timestamp, 0).single()),
        _ => None,
    }
}

fn find_expires_at(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::Object(map) => {
            if map
                .get("refresh_token")
                .and_then(Value::as_str)
                .is_some_and(|token| !token.is_empty())
            {
                return None;
            }
            if let Some(expires_at) = map.get("expires_at").and_then(parse_expiry_value) {
                return Some(expires_at);
            }
            if let Some(expires_at) = map.get("expires_on").and_then(parse_expiry_value) {
                return Some(expires_at);
            }
            map.values().find_map(find_expires_at)
        }
        Value::Array(values) => values.iter().find_map(find_expires_at),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct ProviderCacheSecretDefinition {
    provider: &'static str,
    name: &'static str,
    path: &'static str,
    is_directory: bool,
}

const PROVIDER_CACHE_SECRET_DEFINITIONS: &[ProviderCacheSecretDefinition] = &[
    ProviderCacheSecretDefinition {
        provider: "gemini_oauth",
        name: "OAuth token",
        path: "gemini_oauth/tokens.json",
        is_directory: false,
    },
    ProviderCacheSecretDefinition {
        provider: "chatgpt_codex",
        name: "OAuth token",
        path: "chatgpt_codex/tokens.json",
        is_directory: false,
    },
    ProviderCacheSecretDefinition {
        provider: "kimi_code",
        name: "OAuth token",
        path: "kimicode/token.json",
        is_directory: false,
    },
    ProviderCacheSecretDefinition {
        provider: "github_copilot",
        name: "OAuth token",
        path: "githubcopilot",
        is_directory: true,
    },
    ProviderCacheSecretDefinition {
        provider: "xai_oauth",
        name: "OAuth token",
        path: "xai_oauth/tokens.json",
        is_directory: false,
    },
    ProviderCacheSecretDefinition {
        provider: "databricks",
        name: "OAuth token",
        path: "databricks/oauth",
        is_directory: true,
    },
    ProviderCacheSecretDefinition {
        provider: "databricks_v2",
        name: "OAuth token",
        path: "databricks/oauth",
        is_directory: true,
    },
];

fn provider_cache_definitions_for_display() -> Vec<ProviderCacheSecretDefinition> {
    let mut seen_paths = HashSet::new();
    PROVIDER_CACHE_SECRET_DEFINITIONS
        .iter()
        .copied()
        .filter(|definition| seen_paths.insert(definition.path))
        .collect()
}

fn provider_cache_definition(provider: &str) -> Option<ProviderCacheSecretDefinition> {
    PROVIDER_CACHE_SECRET_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| definition.provider == provider)
}

fn provider_cache_providers_sharing_cache(provider: &str) -> Vec<&'static str> {
    let Some(definition) = provider_cache_definition(provider) else {
        return Vec::new();
    };

    PROVIDER_CACHE_SECRET_DEFINITIONS
        .iter()
        .filter(|other| other.path == definition.path)
        .map(|definition| definition.provider)
        .collect()
}

fn read_json_file(path: &std::path::Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

fn collect_json_expiries(path: &std::path::Path, is_directory: bool) -> Vec<DateTime<Utc>> {
    if !is_directory {
        return read_json_file(path)
            .and_then(|value| find_expires_at(&value))
            .into_iter()
            .collect();
    }

    let mut expiries = Vec::new();
    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(current) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Some(expires_at) =
                read_json_file(&path).and_then(|value| find_expires_at(&value))
            {
                expiries.push(expires_at);
            }
        }
    }

    expiries
}

fn provider_cache_exists(path: &std::path::Path, is_directory: bool) -> bool {
    if !is_directory {
        return path.is_file();
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let path = entry.path();
        path.is_file() || provider_cache_exists(&path, true)
    })
}

fn provider_cache_expiry(definition: ProviderCacheSecretDefinition) -> Option<DateTime<Utc>> {
    let path = Paths::in_config_dir(definition.path);
    let expiries = collect_json_expiries(&path, definition.is_directory);
    expiries.into_iter().min()
}

fn build_provider_cache_secret(
    definition: ProviderCacheSecretDefinition,
    display_names: &HashMap<String, String>,
) -> Option<ProviderSecret> {
    let path = Paths::in_config_dir(definition.path);
    if !provider_cache_exists(&path, definition.is_directory) {
        return None;
    }

    let expires_at = provider_cache_expiry(definition);
    Some(ProviderSecret {
        id: format!("{}{}", PROVIDER_CACHE_ID_PREFIX, definition.provider),
        provider: definition.provider.to_string(),
        provider_display_name: display_names
            .get(definition.provider)
            .cloned()
            .unwrap_or_else(|| definition.provider.to_string()),
        name: definition.name.to_string(),
        storage: ProviderSecretStorage::ProviderCache,
        expires_at,
        status: provider_secret_status(expires_at),
        configured: true,
        has_secret: true,
        can_delete: true,
        can_configure: false,
        configure_provider: None,
    })
}

fn build_huggingface_oauth_secret(
    token: Option<huggingface_auth::HuggingFaceTokenData>,
) -> ProviderSecret {
    let expires_at = token.as_ref().and_then(|token| token.expires_at);
    let has_secret = token.is_some();

    ProviderSecret {
        id: format!(
            "{}{}",
            PROVIDER_CACHE_ID_PREFIX,
            huggingface_auth::HUGGINGFACE_PROVIDER_NAME
        ),
        provider: huggingface_auth::HUGGINGFACE_PROVIDER_NAME.to_string(),
        provider_display_name: huggingface_auth::HUGGINGFACE_DISPLAY_NAME.to_string(),
        name: huggingface_auth::HUGGINGFACE_OAUTH_TOKEN_NAME.to_string(),
        storage: ProviderSecretStorage::ProviderCache,
        expires_at,
        status: provider_secret_status(expires_at),
        configured: has_secret,
        has_secret,
        can_delete: has_secret,
        can_configure: true,
        configure_provider: Some(huggingface_auth::HUGGINGFACE_PROVIDER_NAME.to_string()),
    }
}

fn build_secret_store_secrets(
    stored_secrets: &HashMap<String, Value>,
    providers: &[(ProviderMetadata, ProviderType)],
) -> Vec<ProviderSecret> {
    let mut secrets = Vec::new();

    for (metadata, _) in providers {
        for config_key in metadata.config_keys.iter().filter(|key| key.secret) {
            if !stored_secrets.contains_key(&config_key.name) {
                continue;
            }
            secrets.push(ProviderSecret {
                id: format!(
                    "{}{}:{}",
                    SECRET_STORE_ID_PREFIX, metadata.name, config_key.name
                ),
                provider: metadata.name.clone(),
                provider_display_name: metadata.display_name.clone(),
                name: config_key.name.clone(),
                storage: ProviderSecretStorage::SecretStore,
                expires_at: None,
                status: ProviderSecretStatus::Unknown,
                configured: true,
                has_secret: true,
                can_delete: true,
                can_configure: false,
                configure_provider: None,
            });
        }
    }

    secrets
}

fn is_known_provider_secret(
    providers: &[(ProviderMetadata, ProviderType)],
    provider: &str,
    key: &str,
) -> bool {
    providers
        .iter()
        .filter(|(metadata, _)| metadata.name == provider)
        .flat_map(|(metadata, _)| metadata.config_keys.iter())
        .any(|config_key| config_key.secret && config_key.name == key)
}

fn unconfigure_provider(config: &Config, provider_name: &str) -> Result<(), ConfigError> {
    if let Some(mut entry) = goose::config::get_provider_entry(config, provider_name) {
        entry.configured = false;
        goose::config::set_provider_entry(config, provider_name, &entry)?;
    }

    let configured_marker = format!("{}_configured", provider_name);
    config.delete(&configured_marker)?;
    Ok(())
}

fn mark_provider_configured(config: &Config, provider_name: &str) -> Result<(), ConfigError> {
    if let Some(mut entry) = goose::config::get_provider_entry(config, provider_name) {
        entry.configured = true;
        goose::config::set_provider_entry(config, provider_name, &entry)?;
    } else {
        let model = if goose::config::get_active_provider(config).as_deref() == Some(provider_name)
        {
            config.get_goose_model().unwrap_or_default()
        } else {
            String::new()
        };
        goose::config::set_provider_entry(
            config,
            provider_name,
            &goose::config::ProviderEntry {
                enabled: true,
                model,
                configured: true,
            },
        )?;
    }

    Ok(())
}

fn parse_secret_store_id(id: &str) -> Option<(&str, &str)> {
    let rest = id.strip_prefix(SECRET_STORE_ID_PREFIX)?;
    let (provider, key) = rest.split_once(':')?;
    Some((provider, key))
}

fn parse_provider_cache_id(id: &str) -> Option<&str> {
    id.strip_prefix(PROVIDER_CACHE_ID_PREFIX)
}

fn is_valid_provider_name(provider_name: &str) -> bool {
    !provider_name.is_empty()
        && provider_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn should_unconfigure_after_secret_delete(
    provider: &str,
    key: &str,
    has_usable_huggingface_oauth_token: impl FnOnce() -> bool,
) -> bool {
    provider == huggingface_auth::HUGGINGFACE_PROVIDER_NAME
        && key == huggingface_auth::HUGGINGFACE_TOKEN_SECRET_KEY
        && !has_usable_huggingface_oauth_token()
}

#[utoipa::path(
    get,
    path = "/config/provider-secrets",
    responses(
        (status = 200, description = "Provider secrets retrieved successfully", body = ProviderSecretsResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_provider_secrets() -> Result<Json<ProviderSecretsResponse>, ErrorResponse> {
    let config = Config::global();
    let stored_secrets = config.all_secrets()?;
    let providers = get_providers().await;
    let display_names: HashMap<String, String> = providers
        .iter()
        .map(|(metadata, _)| (metadata.name.clone(), metadata.display_name.clone()))
        .collect();

    let mut secrets = build_secret_store_secrets(&stored_secrets, &providers);

    for definition in provider_cache_definitions_for_display() {
        if let Some(secret) = build_provider_cache_secret(definition, &display_names) {
            if !secrets.iter().any(|existing| existing.id == secret.id) {
                secrets.push(secret);
            }
        }
    }

    let huggingface_secret = build_huggingface_oauth_secret(huggingface_auth::load_oauth_token());
    if let Some(existing) = secrets
        .iter_mut()
        .find(|existing| existing.id == huggingface_secret.id)
    {
        *existing = huggingface_secret;
    } else {
        secrets.push(huggingface_secret);
    }

    secrets.sort_by(|a, b| {
        a.provider_display_name
            .cmp(&b.provider_display_name)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(Json(ProviderSecretsResponse { secrets }))
}

#[utoipa::path(
    delete,
    path = "/config/provider-secrets/{id}",
    params(
        ("id" = String, Path, description = "Provider secret identifier")
    ),
    responses(
        (status = 200, description = "Provider secret deleted successfully", body = String),
        (status = 400, description = "Invalid provider secret identifier"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_provider_secret(Path(id): Path<String>) -> Result<Json<String>, ErrorResponse> {
    let config = Config::global();

    if let Some((provider, key)) = parse_secret_store_id(&id) {
        let providers = get_providers().await;
        if !is_known_provider_secret(&providers, provider, key) {
            return Err(ErrorResponse::bad_request(format!(
                "Invalid provider secret id: '{}'",
                id
            )));
        }

        config.delete_secret(key)?;
        if should_unconfigure_after_secret_delete(provider, key, || {
            huggingface_auth::has_configured_token().unwrap_or(false)
        }) {
            unconfigure_provider(config, provider)?;
        }
        return Ok(Json(format!("Deleted provider secret {}", id)));
    }

    if let Some(provider) = parse_provider_cache_id(&id) {
        if provider == huggingface_auth::HUGGINGFACE_PROVIDER_NAME {
            huggingface_auth::clear_oauth_token()?;
            unconfigure_provider(config, provider)?;
            return Ok(Json(format!("Deleted provider secret {}", id)));
        }

        let cache_definition = provider_cache_definition(provider);

        if !is_valid_provider_name(provider) || cache_definition.is_none() {
            return Err(ErrorResponse::bad_request(format!(
                "Invalid provider name: '{}'",
                provider
            )));
        }
        goose::providers::cleanup_provider(provider).await?;
        for shared_provider in provider_cache_providers_sharing_cache(provider) {
            unconfigure_provider(config, shared_provider)?;
        }
        return Ok(Json(format!("Deleted provider secret {}", id)));
    }

    Err(ErrorResponse::bad_request(format!(
        "Invalid provider secret id: '{}'",
        id
    )))
}

#[utoipa::path(
    post,
    path = "/config/read",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value retrieved successfully", body = Value),
        (status = 500, description = "Unable to get the configuration value"),
    )
)]
pub async fn read_config(
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<ConfigValueResponse>, ErrorResponse> {
    let config = Config::global();

    // Intercept legacy keys to return structured provider config
    if query.key == "GOOSE_PROVIDER" || query.key == "active_provider" {
        if let Ok(val) = config.get_goose_provider() {
            return Ok(Json(ConfigValueResponse::Value(Value::String(val))));
        }
        return Ok(Json(ConfigValueResponse::Value(Value::Null)));
    }
    if query.key == "GOOSE_MODEL" {
        if let Ok(val) = config.get_goose_model() {
            return Ok(Json(ConfigValueResponse::Value(Value::String(val))));
        }
        return Ok(Json(ConfigValueResponse::Value(Value::Null)));
    }

    let response_value = match config.get(&query.key, query.is_secret) {
        Ok(value) => {
            if query.is_secret {
                ConfigValueResponse::MaskedValue(MaskedSecret {
                    masked_value: mask_secret(value),
                })
            } else {
                ConfigValueResponse::Value(value)
            }
        }
        Err(ConfigError::NotFound(_)) => ConfigValueResponse::Value(Value::Null),
        Err(e) => return Err(e.into()),
    };
    Ok(Json(response_value))
}

#[utoipa::path(
    get,
    path = "/config/extensions",
    responses(
        (status = 200, description = "All extensions retrieved successfully", body = ExtensionResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_extensions() -> Result<Json<ExtensionResponse>, ErrorResponse> {
    let extensions = goose::config::get_all_extensions()
        .into_iter()
        .filter(|ext| !goose::agents::extension_manager::is_hidden_extension(&ext.config.name()))
        .collect();
    let warnings = goose::config::get_warnings();
    Ok(Json(ExtensionResponse {
        extensions,
        warnings,
    }))
}

#[utoipa::path(
    post,
    path = "/config/extensions",
    request_body = ExtensionQuery,
    responses(
        (status = 200, description = "Extension added or updated successfully", body = String),
        (status = 400, description = "Invalid request"),
        (status = 422, description = "Could not serialize config.yaml"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn add_extension(
    Json(extension_query): Json<ExtensionQuery>,
) -> Result<Json<String>, ErrorResponse> {
    let extensions = goose::config::get_all_extensions();
    let key = goose::config::extensions::name_to_key(&extension_query.name);

    let is_update = extensions.iter().any(|e| e.config.key() == key);

    goose::config::set_extension(ExtensionEntry {
        enabled: extension_query.enabled,
        config: extension_query.config,
    });

    if is_update {
        Ok(Json(format!("Updated extension {}", extension_query.name)))
    } else {
        Ok(Json(format!("Added extension {}", extension_query.name)))
    }
}

#[utoipa::path(
    delete,
    path = "/config/extensions/{name}",
    responses(
        (status = 200, description = "Extension removed successfully", body = String),
        (status = 404, description = "Extension not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_extension(Path(name): Path<String>) -> Result<Json<String>, ErrorResponse> {
    let key = goose::config::extensions::name_to_key(&name);
    goose::config::remove_extension(&key);
    Ok(Json(format!("Removed extension {}", name)))
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = ConfigResponse)
    )
)]
pub async fn read_all_config() -> Result<Json<ConfigResponse>, ErrorResponse> {
    let config = Config::global();
    let values = config
        .all_values()
        .map_err(|e| ErrorResponse::unprocessable(e.to_string()))?;
    Ok(Json(ConfigResponse { config: values }))
}

#[utoipa::path(
    get,
    path = "/config/providers",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = [ProviderDetails])
    )
)]
pub async fn providers() -> Result<Json<Vec<ProviderDetails>>, ErrorResponse> {
    let config = Config::global();
    let providers = get_providers().await;
    let providers_response: Vec<ProviderDetails> = providers
        .into_iter()
        .map(|(metadata, provider_type)| {
            let is_configured = check_provider_configured(&metadata, provider_type);
            let saved_model = goose::config::get_provider_entry(config, &metadata.name)
                .map(|e| e.model)
                .filter(|m| !m.is_empty());

            ProviderDetails {
                name: metadata.name.clone(),
                metadata,
                is_configured,
                provider_type,
                saved_model,
            }
        })
        .collect();

    Ok(Json(providers_response))
}

#[utoipa::path(
    get,
    path = "/config/providers/{name}/models",
    params(
        ("name" = String, Path, description = "Provider name (e.g., openai)")
    ),
    responses(
        (status = 200, description = "Models fetched successfully", body = [ModelInfo]),
        (status = 400, description = "Unknown provider, provider not configured, or authentication error"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_provider_models(
    Path(name): Path<String>,
) -> Result<Json<Vec<ModelInfo>>, ErrorResponse> {
    let all = get_providers().await.into_iter().collect::<Vec<_>>();
    let Some((metadata, provider_type)) = all.into_iter().find(|(m, _)| m.name == name) else {
        return Err(ErrorResponse::bad_request(format!(
            "Unknown provider: {}",
            name
        )));
    };
    if !check_provider_configured(&metadata, provider_type) {
        return Err(ErrorResponse::bad_request(format!(
            "Provider '{}' is not configured",
            name
        )));
    }

    let model_config =
        goose::model_config::model_config_from_user_config(&name, &metadata.default_model)?;
    let provider = goose::providers::create(&name, model_config, Vec::new()).await?;

    let models_result = provider.fetch_recommended_model_info().await;

    match models_result {
        Ok(models) => Ok(Json(models)),
        Err(provider_error) => Err(provider_error.into()),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ProviderModelInfoQuery {
    pub model: String,
}

pub async fn resolve_provider_model_info(
    name: &str,
    model: &str,
) -> Result<ModelInfo, ErrorResponse> {
    let all = get_providers().await.into_iter().collect::<Vec<_>>();
    let Some((metadata, provider_type)) = all.into_iter().find(|(m, _)| m.name == name) else {
        return Err(ErrorResponse::bad_request(format!(
            "Unknown provider: {}",
            name
        )));
    };
    if !check_provider_configured(&metadata, provider_type) {
        return Err(ErrorResponse::bad_request(format!(
            "Provider '{}' is not configured",
            name
        )));
    }

    let model_config = goose::model_config::model_config_from_user_config(name, model)?;
    let provider = goose::providers::create(name, model_config.clone(), Vec::new()).await?;
    match provider.fetch_model_info(model).await {
        Ok(info) => Ok(info),
        Err(error) => {
            let mut info = ModelInfo::new(model, model_config.context_limit());
            info.reasoning = model_config.is_reasoning_model();
            tracing::debug!(
                provider = name,
                model,
                error = %error,
                "Falling back to local model metadata"
            );
            Ok(info)
        }
    }
}

#[utoipa::path(
    post,
    path = "/config/providers/{name}/model-info",
    params(
        ("name" = String, Path, description = "Provider name (e.g., openai)")
    ),
    request_body = ProviderModelInfoQuery,
    responses(
        (status = 200, description = "Model metadata fetched successfully", body = ModelInfo),
        (status = 400, description = "Unknown provider, provider not configured, or authentication error"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_provider_model_info(
    Path(name): Path<String>,
    Json(query): Json<ProviderModelInfoQuery>,
) -> Result<Json<ModelInfo>, ErrorResponse> {
    resolve_provider_model_info(&name, &query.model)
        .await
        .map(Json)
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SlashCommandsQuery {
    /// Optional working directory to discover local skills from
    pub working_dir: Option<String>,
}

#[utoipa::path(
    get,
    path = "/config/slash_commands",
    params(SlashCommandsQuery),
    responses(
        (status = 200, description = "Slash commands retrieved successfully", body = SlashCommandsResponse)
    )
)]
pub async fn get_slash_commands(
    axum::extract::Query(query): axum::extract::Query<SlashCommandsQuery>,
) -> Result<Json<SlashCommandsResponse>, ErrorResponse> {
    let mut commands: Vec<_> = recipe_slash_command::list_commands()
        .iter()
        .map(|command| SlashCommand {
            command: command.command.clone(),
            help: command.recipe_path.clone(),
            command_type: CommandType::Recipe,
        })
        .collect();

    for cmd_def in execute_commands::list_commands() {
        commands.push(SlashCommand {
            command: cmd_def.name.to_string(),
            help: cmd_def.description.to_string(),
            command_type: CommandType::Builtin,
        });
    }

    let working_dir = query.working_dir.map(std::path::PathBuf::from);
    for source in goose::skills::list_installed_skills(working_dir.as_deref()) {
        commands.push(SlashCommand {
            command: source.name,
            help: source.description,
            command_type: CommandType::Skill,
        });
    }

    let discover_dir = working_dir
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));
    for source in
        goose::agents::platform_extensions::summon::discover_filesystem_sources(discover_dir)
    {
        if matches!(
            source.source_type,
            SourceType::Agent | SourceType::Recipe | SourceType::Subrecipe
        ) && !source.content.is_empty()
        {
            commands.push(SlashCommand {
                command: source.name,
                help: source.description,
                command_type: CommandType::Agent,
            });
        }
    }

    Ok(Json(SlashCommandsResponse { commands }))
}

#[derive(Serialize, ToSchema)]
pub struct ModelInfoData {
    pub provider: String,
    pub model: String,
    pub context_limit: usize,
    pub max_output_tokens: Option<usize>,
    pub reasoning: bool,
    pub input_token_cost: Option<f64>,
    pub output_token_cost: Option<f64>,
    pub cache_read_token_cost: Option<f64>,
    pub cache_write_token_cost: Option<f64>,
    pub currency: String,
}

#[derive(Serialize, ToSchema)]
pub struct ModelInfoResponse {
    pub model_info: Option<ModelInfoData>,
    pub source: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ModelInfoQuery {
    pub provider: String,
    pub model: String,
}

#[utoipa::path(
    post,
    path = "/config/canonical-model-info",
    request_body = ModelInfoQuery,
    responses(
        (status = 200, description = "Model information retrieved successfully", body = ModelInfoResponse)
    )
)]
pub async fn get_canonical_model_info(
    Json(query): Json<ModelInfoQuery>,
) -> Json<ModelInfoResponse> {
    let canonical_model = maybe_get_canonical_model(&query.provider, &query.model);

    let model_info = canonical_model.map(|canonical_model| ModelInfoData {
        provider: query.provider.clone(),
        model: query.model.clone(),
        context_limit: canonical_model.limit.context,
        max_output_tokens: canonical_model.limit.output,
        reasoning: canonical_model
            .reasoning
            .unwrap_or_else(|| ModelConfig::new_or_fail(&query.model).is_reasoning_model()),
        // Costs are per million tokens - client handles division for display
        input_token_cost: canonical_model.cost.input,
        output_token_cost: canonical_model.cost.output,
        cache_read_token_cost: canonical_model.cost.cache_read,
        cache_write_token_cost: canonical_model.cost.cache_write,
        currency: "$".to_string(),
    });

    Json(ModelInfoResponse {
        model_info,
        source: "canonical".to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/config/permissions",
    request_body = UpsertPermissionsQuery,
    responses(
        (status = 200, description = "Permission update completed", body = String),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn upsert_permissions(
    Json(query): Json<UpsertPermissionsQuery>,
) -> Result<Json<String>, ErrorResponse> {
    let permission_manager = goose::config::PermissionManager::instance();

    for tool_permission in &query.tool_permissions {
        permission_manager.update_user_permission(
            &tool_permission.tool_name,
            tool_permission.permission.clone(),
        );
    }

    Ok(Json("Permissions updated successfully".to_string()))
}

#[utoipa::path(
    get,
    path = "/config/validate",
    responses(
        (status = 200, description = "Config validation result", body = String),
        (status = 422, description = "Config file is corrupted")
    )
)]
pub async fn validate_config() -> Result<Json<String>, ErrorResponse> {
    let config_path = Paths::config_dir().join("config.yaml");

    if !config_path.exists() {
        return Ok(Json("Config file does not exist".to_string()));
    }

    let content = std::fs::read_to_string(&config_path)?;
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .map_err(|e| ErrorResponse::unprocessable(format!("Config file is corrupted: {}", e)))?;

    Ok(Json("Config file is valid".to_string()))
}
#[derive(Serialize, ToSchema)]
pub struct CreateCustomProviderResponse {
    pub provider_name: String,
}

#[utoipa::path(
    post,
    path = "/config/custom-providers",
    request_body = UpdateCustomProviderRequest,
    responses(
        (status = 200, description = "Custom provider created successfully", body = CreateCustomProviderResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_custom_provider(
    Json(request): Json<UpdateCustomProviderRequest>,
) -> Result<Json<CreateCustomProviderResponse>, ErrorResponse> {
    let config = goose::config::declarative_providers::create_custom_provider(
        goose::config::declarative_providers::CreateCustomProviderParams {
            engine: request.engine,
            display_name: request.display_name,
            api_url: request.api_url,
            api_key: normalize_custom_provider_api_key(request.api_key),
            models: request.models,
            supports_streaming: request.supports_streaming,
            headers: request.headers,
            requires_auth: request.requires_auth,
            catalog_provider_id: request.catalog_provider_id,
            base_path: request.base_path,
            preserves_thinking: request.preserves_thinking,
        },
    )?;

    goose::providers::refresh_custom_providers().await?;

    Ok(Json(CreateCustomProviderResponse {
        provider_name: config.id().to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/config/custom-providers/{id}",
    responses(
        (status = 200, description = "Custom provider retrieved successfully", body = LoadedProvider),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_custom_provider(
    Path(id): Path<String>,
) -> Result<Json<LoadedProvider>, ErrorResponse> {
    let loaded_provider = goose::config::declarative_providers::load_provider(id.as_str())
        .map_err(|e| {
            ErrorResponse::not_found(format!("Custom provider '{}' not found: {}", id, e))
        })?;

    Ok(Json(loaded_provider))
}

#[utoipa::path(
    delete,
    path = "/config/custom-providers/{id}",
    responses(
        (status = 200, description = "Custom provider removed successfully", body = String),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_custom_provider(Path(id): Path<String>) -> Result<Json<String>, ErrorResponse> {
    goose::config::declarative_providers::remove_custom_provider(&id)?;

    goose::providers::refresh_custom_providers().await?;

    Ok(Json(format!("Removed custom provider: {}", id)))
}

#[utoipa::path(
    post,
    path = "/config/providers/{name}/cleanup",
    params(
        ("name" = String, Path, description = "Provider name (e.g., githubcopilot)")
    ),
    responses(
        (status = 200, description = "Provider cache cleaned up successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cleanup_provider_cache(
    Path(name): Path<String>,
) -> Result<Json<String>, ErrorResponse> {
    goose::providers::cleanup_provider(&name).await?;
    Ok(Json(format!("Cleaned up provider cache: {}", name)))
}

#[utoipa::path(
    put,
    path = "/config/custom-providers/{id}",
    request_body = UpdateCustomProviderRequest,
    responses(
        (status = 200, description = "Custom provider updated successfully", body = String),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_custom_provider(
    Path(id): Path<String>,
    Json(request): Json<UpdateCustomProviderRequest>,
) -> Result<Json<String>, ErrorResponse> {
    goose::config::declarative_providers::update_custom_provider(
        goose::config::declarative_providers::UpdateCustomProviderParams {
            id: id.clone(),
            engine: request.engine,
            display_name: request.display_name,
            api_url: request.api_url,
            api_key: normalize_custom_provider_api_key(request.api_key),
            models: request.models,
            supports_streaming: request.supports_streaming,
            headers: request.headers,
            requires_auth: request.requires_auth,
            catalog_provider_id: request.catalog_provider_id,
            base_path: request.base_path,
            preserves_thinking: request.preserves_thinking,
        },
    )?;

    goose::providers::refresh_custom_providers().await?;

    Ok(Json(format!("Updated custom provider: {}", id)))
}

#[utoipa::path(
    post,
    path = "/config/check_provider",
    request_body = CheckProviderRequest,
)]
pub async fn check_provider(
    Json(CheckProviderRequest { provider }): Json<CheckProviderRequest>,
) -> Result<(), ErrorResponse> {
    // Provider check does not use extensions.
    create_with_default_model(&provider, Vec::new())
        .await
        .map_err(|err| {
            ErrorResponse::bad_request(format!("Provider '{}' check failed: {}", provider, err))
        })?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/config/set_provider",
    request_body = SetProviderRequest,
)]
pub async fn set_config_provider(
    Json(SetProviderRequest { provider, model }): Json<SetProviderRequest>,
) -> Result<(), ErrorResponse> {
    // Provider validation does not use extensions.
    create_with_default_model(&provider, Vec::new())
        .await
        .and_then(|_| {
            let config = Config::global();
            goose::config::set_active_provider(config, &provider, &model)
                .map_err(|e| anyhow::anyhow!(e))
        })
        .map_err(|err| {
            ErrorResponse::bad_request(format!(
                "Failed to set provider to '{}' with model '{}': {}",
                provider, model, err
            ))
        })?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/config/provider-catalog",
    params(
        ("format" = Option<String>, Query, description = "Filter by provider format (openai, anthropic, ollama)")
    ),
    responses(
        (status = 200, description = "Provider catalog retrieved successfully", body = [ProviderCatalogEntry]),
        (status = 400, description = "Invalid format parameter")
    )
)]
pub async fn get_provider_catalog(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<ProviderCatalogEntry>>, ErrorResponse> {
    let format_str = params.get("format").map(|s| s.as_str()).unwrap_or("openai");

    let format = format_str.parse::<ProviderFormat>().map_err(|_| {
        ErrorResponse::bad_request(format!(
            "Invalid format '{}'. Must be one of: openai, anthropic, ollama",
            format_str
        ))
    })?;

    let providers = get_providers_by_format(format).await;
    Ok(Json(providers))
}

#[utoipa::path(
    get,
    path = "/config/provider-catalog/{id}",
    params(
        ("id" = String, Path, description = "Provider ID from models.dev")
    ),
    responses(
        (status = 200, description = "Provider template retrieved successfully", body = ProviderTemplate),
        (status = 404, description = "Provider not found in catalog")
    )
)]
pub async fn get_provider_catalog_template(
    Path(id): Path<String>,
) -> Result<Json<ProviderTemplate>, ErrorResponse> {
    let template = get_provider_template(&id).ok_or_else(|| {
        ErrorResponse::not_found(format!("Provider '{}' not found in catalog", id))
    })?;

    Ok(Json(template))
}

#[utoipa::path(
    post,
    path = "/config/providers/{name}/oauth",
    params(
        ("name" = String, Path, description = "Provider name")
    ),
    responses(
        (status = 200, description = "OAuth configuration completed"),
        (status = 400, description = "OAuth configuration failed")
    )
)]
pub async fn configure_provider_oauth(
    Path(provider_name): Path<String>,
) -> Result<Json<String>, ErrorResponse> {
    use goose::providers::create;

    if !is_valid_provider_name(&provider_name) {
        return Err(ErrorResponse::bad_request(format!(
            "Invalid provider name: '{}'",
            provider_name
        )));
    }

    if provider_name == huggingface_auth::HUGGINGFACE_PROVIDER_NAME {
        huggingface_auth::configure_oauth().await.map_err(|e| {
            ErrorResponse::bad_request(format!(
                "OAuth configuration failed for provider '{}': {}",
                provider_name, e
            ))
        })?;
        mark_provider_configured(goose::config::Config::global(), &provider_name)?;
        return Ok(Json("OAuth configuration completed".to_string()));
    }

    let temp_model = goose::model_config::model_config_from_user_config(&provider_name, "temp")
        .map_err(|e| {
            ErrorResponse::bad_request(format!("Failed to create temporary model config: {}", e))
        })?;

    // OAuth configuration does not use extensions.
    let provider = create(&provider_name, temp_model, Vec::new())
        .await
        .map_err(|e| {
            ErrorResponse::bad_request(format!(
                "Failed to create provider '{}': {}",
                provider_name, e
            ))
        })?;

    provider.configure_oauth().await.map_err(|e| {
        ErrorResponse::bad_request(format!(
            "OAuth configuration failed for provider '{}': {}",
            provider_name, e
        ))
    })?;

    mark_provider_configured(goose::config::Config::global(), &provider_name)?;

    Ok(Json("OAuth configuration completed".to_string()))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/provider-secrets", get(list_provider_secrets))
        .route(
            "/config/provider-secrets/{id}",
            delete(delete_provider_secret),
        )
        .route("/config/extensions", get(get_extensions))
        .route("/config/extensions", post(add_extension))
        .route("/config/extensions/{name}", delete(remove_extension))
        .route("/config/providers", get(providers))
        .route("/config/providers/{name}/models", get(get_provider_models))
        .route(
            "/config/providers/{name}/model-info",
            post(get_provider_model_info),
        )
        .route("/config/provider-catalog", get(get_provider_catalog))
        .route(
            "/config/provider-catalog/{id}",
            get(get_provider_catalog_template),
        )
        .route(
            "/config/providers/{name}/cleanup",
            post(cleanup_provider_cache),
        )
        .route("/config/slash_commands", get(get_slash_commands))
        .route(
            "/config/canonical-model-info",
            post(get_canonical_model_info),
        )
        .route("/config/validate", get(validate_config))
        .route("/config/permissions", post(upsert_permissions))
        .route("/config/custom-providers", post(create_custom_provider))
        .route(
            "/config/custom-providers/{id}",
            delete(remove_custom_provider),
        )
        .route("/config/custom-providers/{id}", put(update_custom_provider))
        .route("/config/custom-providers/{id}", get(get_custom_provider))
        .route("/config/check_provider", post(check_provider))
        .route("/config/set_provider", post(set_config_provider))
        .route(
            "/config/providers/{name}/oauth",
            post(configure_provider_oauth),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::config::ProviderEntry;
    use goose::providers::base::ConfigKey;
    use serde_json::json;

    fn new_test_config() -> Config {
        let unique = format!(
            "goose-server-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let config_path = std::env::temp_dir().join(format!("{unique}-config.yaml"));
        let secrets_path = std::env::temp_dir().join(format!("{unique}-secrets.yaml"));
        Config::new_with_file_secrets(config_path, secrets_path).unwrap()
    }

    #[test]
    fn secret_store_listing_only_includes_provider_secret_keys() {
        let metadata = ProviderMetadata::new(
            "openai",
            "OpenAI",
            "OpenAI provider",
            "gpt-4o",
            vec![],
            "https://example.com",
            vec![
                ConfigKey::new("OPENAI_API_KEY", true, true, None, true),
                ConfigKey::new("OPENAI_HOST", false, false, None, false),
            ],
        );
        let providers = vec![(metadata, ProviderType::Builtin)];
        let stored_secrets = HashMap::from([
            (
                "OPENAI_API_KEY".to_string(),
                Value::String("secret-value".to_string()),
            ),
            (
                "UNRELATED_SECRET".to_string(),
                Value::String("other-secret".to_string()),
            ),
            (
                "OPENAI_HOST".to_string(),
                Value::String("https://api.openai.com".to_string()),
            ),
        ]);

        let secrets = build_secret_store_secrets(&stored_secrets, &providers);

        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].id, "secret_store:openai:OPENAI_API_KEY");
        assert_eq!(secrets[0].provider_display_name, "OpenAI");
        assert_eq!(secrets[0].name, "OPENAI_API_KEY");
        assert_eq!(secrets[0].storage, ProviderSecretStorage::SecretStore);
        assert_eq!(secrets[0].status, ProviderSecretStatus::Unknown);
    }

    #[test]
    fn provider_secret_delete_validation_requires_provider_secret_key() {
        let metadata = ProviderMetadata::new(
            "openai",
            "OpenAI",
            "OpenAI provider",
            "gpt-4o",
            vec![],
            "https://example.com",
            vec![
                ConfigKey::new("OPENAI_API_KEY", true, true, None, true),
                ConfigKey::new("OPENAI_HOST", false, false, None, false),
            ],
        );
        let providers = vec![(metadata, ProviderType::Builtin)];

        assert!(is_known_provider_secret(
            &providers,
            "openai",
            "OPENAI_API_KEY"
        ));
        assert!(!is_known_provider_secret(
            &providers,
            "openai",
            "OPENAI_HOST"
        ));
        assert!(!is_known_provider_secret(
            &providers,
            "openai",
            "UNRELATED_SECRET"
        ));
        assert!(!is_known_provider_secret(
            &providers,
            "anthropic",
            "OPENAI_API_KEY"
        ));
    }

    #[test]
    fn expiry_extraction_handles_nested_rfc3339_values() {
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let value = json!({
            "project_id": "project",
            "token": {
                "access_token": "secret",
                "expires_at": expires_at.to_rfc3339(),
            }
        });

        let parsed = find_expires_at(&value).expect("expected expiry");

        assert_eq!(parsed.timestamp(), expires_at.timestamp());
        assert_eq!(
            provider_secret_status(Some(parsed)),
            ProviderSecretStatus::Valid
        );
    }

    #[test]
    fn expiry_extraction_ignores_refreshable_access_tokens() {
        let expires_at = Utc::now() - chrono::Duration::hours(1);
        let value = json!({
            "access_token": "access",
            "refresh_token": "refresh",
            "expires_at": expires_at.to_rfc3339(),
        });

        assert_eq!(find_expires_at(&value), None);
    }

    #[test]
    fn expiry_extraction_handles_expired_unix_timestamps() {
        let value = json!({
            "info": {
                "expires_at": 1
            }
        });

        let parsed = find_expires_at(&value).expect("expected expiry");

        assert_eq!(parsed.timestamp(), 1);
        assert_eq!(
            provider_secret_status(Some(parsed)),
            ProviderSecretStatus::Expired
        );
    }

    #[test]
    fn provider_secret_ids_parse_expected_prefixes() {
        assert_eq!(
            parse_secret_store_id("secret_store:openai:OPENAI_API_KEY"),
            Some(("openai", "OPENAI_API_KEY"))
        );
        assert_eq!(
            parse_provider_cache_id("provider_cache:gemini_oauth"),
            Some("gemini_oauth")
        );
        assert_eq!(parse_secret_store_id("provider_cache:openai"), None);
        assert_eq!(parse_provider_cache_id("secret_store:openai:key"), None);
    }

    #[test]
    fn shared_databricks_cache_is_displayed_once() {
        let databricks_definitions: Vec<_> = provider_cache_definitions_for_display()
            .into_iter()
            .filter(|definition| definition.path == "databricks/oauth")
            .collect();

        assert_eq!(databricks_definitions.len(), 1);
        assert_eq!(databricks_definitions[0].provider, "databricks");
    }

    #[test]
    fn shared_databricks_cache_unconfigures_both_providers() {
        assert_eq!(
            provider_cache_providers_sharing_cache("databricks"),
            vec!["databricks", "databricks_v2"]
        );
        assert_eq!(
            provider_cache_providers_sharing_cache("databricks_v2"),
            vec!["databricks", "databricks_v2"]
        );
    }

    #[test]
    fn unconfigure_provider_clears_structured_entry() {
        let config = new_test_config();
        goose::config::set_provider_entry(
            &config,
            "huggingface",
            &ProviderEntry {
                enabled: true,
                model: "Qwen/Qwen3-Coder-480B-A35B-Instruct".to_string(),
                configured: true,
            },
        )
        .unwrap();

        unconfigure_provider(&config, "huggingface").unwrap();

        let entry = goose::config::get_provider_entry(&config, "huggingface").unwrap();
        assert!(entry.enabled);
        assert_eq!(entry.model, "Qwen/Qwen3-Coder-480B-A35B-Instruct");
        assert!(!entry.configured);
    }

    #[test]
    fn unconfigure_provider_deletes_legacy_configured_marker() {
        let config = new_test_config();
        config.set_param("huggingface_configured", true).unwrap();

        unconfigure_provider(&config, "huggingface").unwrap();

        assert!(config.get_param::<bool>("huggingface_configured").is_err());
    }

    #[test]
    fn deleting_huggingface_token_unconfigures_without_oauth() {
        assert!(should_unconfigure_after_secret_delete(
            "huggingface",
            "HF_TOKEN",
            || false
        ));
    }

    #[test]
    fn deleting_huggingface_token_keeps_configured_with_oauth() {
        assert!(!should_unconfigure_after_secret_delete(
            "huggingface",
            "HF_TOKEN",
            || true
        ));
    }

    #[test]
    fn deleting_other_provider_secret_does_not_unconfigure_huggingface() {
        assert!(!should_unconfigure_after_secret_delete(
            "openai",
            "OPENAI_API_KEY",
            || false
        ));
    }

    #[test]
    fn huggingface_oauth_secret_is_permanent_without_token() {
        let secret = build_huggingface_oauth_secret(None);

        assert_eq!(secret.id, "provider_cache:huggingface");
        assert_eq!(secret.provider_display_name, "Hugging Face");
        assert_eq!(secret.name, "OAuth token");
        assert_eq!(secret.storage, ProviderSecretStorage::ProviderCache);
        assert_eq!(secret.status, ProviderSecretStatus::Unknown);
        assert!(!secret.configured);
        assert!(!secret.has_secret);
        assert!(!secret.can_delete);
        assert!(secret.can_configure);
        assert_eq!(secret.configure_provider.as_deref(), Some("huggingface"));
    }

    #[test]
    fn huggingface_oauth_secret_reports_cached_token_metadata() {
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let secret = build_huggingface_oauth_secret(Some(huggingface_auth::HuggingFaceTokenData {
            access_token: "hidden".to_string(),
            refresh_token: None,
            expires_at: Some(expires_at),
        }));

        assert_eq!(
            secret.expires_at.map(|value| value.timestamp()),
            Some(expires_at.timestamp())
        );
        assert_eq!(secret.status, ProviderSecretStatus::Valid);
        assert!(secret.configured);
        assert!(secret.has_secret);
        assert!(secret.can_delete);
    }
}
