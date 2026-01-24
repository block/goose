use crate::config::paths::Paths;
use crate::config::Config;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::base::{ModelInfo, ProviderType};
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use utoipa::ToSchema;

static FIXED_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/providers/declarative");

pub fn custom_providers_dir() -> std::path::PathBuf {
    Paths::config_dir().join("custom_providers")
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    OpenAI,
    Ollama,
    Anthropic,
}

/// Configuration for matching models to routes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelMatch {
    /// Glob pattern for matching model names (e.g., "claude-*", "gpt-4*", "*")
    pub model: String,
}

/// Authentication configuration for a route.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthConfig {
    /// Authentication type (currently only "header" is supported)
    #[serde(rename = "type")]
    pub auth_type: String,
    /// Header name for the API key (e.g., "x-api-key", "api-key", "Authorization")
    pub name: String,
}

/// Configuration for a single route that maps models to an engine and endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteConfig {
    /// Pattern to match model names against
    #[serde(rename = "match")]
    pub match_pattern: ModelMatch,
    /// Engine to use for matched models
    pub engine: ProviderEngine,
    /// API endpoint path (e.g., "v1/chat/completions", "anthropic/v1/messages")
    pub path: String,
    /// Optional authentication configuration (overrides default)
    pub auth: Option<AuthConfig>,
    /// Optional extra headers to add for this route
    pub headers: Option<HashMap<String, String>>,
    /// Optional query parameters to append to the URL
    pub query: Option<HashMap<String, String>>,
}

/// Definition of a configuration key for the provider.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigKeyDef {
    /// Environment variable name
    pub name: String,
    /// Whether this key is required
    pub required: bool,
    /// Whether this key contains a secret value
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeclarativeProviderConfig {
    pub name: String,
    /// Engine for single-engine providers. Optional when routes are specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<ProviderEngine>,
    pub display_name: String,
    pub description: Option<String>,
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub supports_streaming: Option<bool>,
    /// Routes for multi-engine providers. When specified, engine field should be None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<RouteConfig>>,
    /// Explicit configuration keys for routed providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_keys: Option<Vec<ConfigKeyDef>>,
    /// Explicit default model (instead of first in models list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Optional authentication configuration (used by routed providers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
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

/// Expands environment variable references in a string.
///
/// Supports the pattern `${VAR_NAME}` where VAR_NAME consists of uppercase letters,
/// digits, and underscores, starting with a letter or underscore.
///
/// # Examples
/// ```ignore
/// // With RESOURCE=myresource set:
/// expand_env_vars("https://${RESOURCE}.example.com") // => "https://myresource.example.com"
/// ```
///
/// # Errors
/// Returns an error if a referenced environment variable is not set.
pub fn expand_env_vars(template: &str) -> Result<String> {
    let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();
    let mut result = template.to_string();
    let mut errors = Vec::new();

    for cap in re.captures_iter(template) {
        let var_name = &cap[1];
        let config = Config::global();
        match config.get_param::<String>(var_name) {
            Ok(value) => {
                let placeholder = format!("${{{}}}", var_name);
                result = result.replace(&placeholder, &value);
            }
            Err(_) => {
                errors.push(var_name.to_string());
            }
        }
    }

    if !errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Missing required environment variable(s): {}",
            errors.join(", ")
        ));
    }

    Ok(result)
}

pub fn create_custom_provider(
    engine: &str,
    display_name: String,
    api_url: String,
    api_key: String,
    models: Vec<String>,
    supports_streaming: Option<bool>,
    headers: Option<HashMap<String, String>>,
) -> Result<DeclarativeProviderConfig> {
    let id = generate_id(&display_name);
    let api_key_name = generate_api_key_name(&id);

    let config = Config::global();
    config.set_secret(&api_key_name, &api_key)?;

    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|name| ModelInfo::new(name, 128000))
        .collect();

    let provider_config = DeclarativeProviderConfig {
        name: id.clone(),
        engine: Some(match engine {
            "openai_compatible" => ProviderEngine::OpenAI,
            "anthropic_compatible" => ProviderEngine::Anthropic,
            "ollama_compatible" => ProviderEngine::Ollama,
            _ => return Err(anyhow::anyhow!("Invalid provider type: {}", engine)),
        }),
        display_name: display_name.clone(),
        description: Some(format!("Custom {} provider", display_name)),
        api_key_env: api_key_name,
        base_url: api_url,
        models: model_infos,
        headers,
        timeout_seconds: None,
        supports_streaming,
        routes: None,
        config_keys: None,
        default_model: None,
        auth: None,
    };

    let custom_providers_dir = custom_providers_dir();
    std::fs::create_dir_all(&custom_providers_dir)?;

    let json_content = serde_json::to_string_pretty(&provider_config)?;
    let file_path = custom_providers_dir.join(format!("{}.json", id));
    std::fs::write(file_path, json_content)?;

    Ok(provider_config)
}

pub fn update_custom_provider(
    id: &str,
    provider_type: &str,
    display_name: String,
    api_url: String,
    api_key: String,
    models: Vec<String>,
    supports_streaming: Option<bool>,
) -> Result<()> {
    let loaded_provider = load_provider(id)?;
    let existing_config = loaded_provider.config;
    let editable = loaded_provider.is_editable;

    let config = Config::global();
    if !api_key.is_empty() {
        config.set_secret(&existing_config.api_key_env, &api_key)?;
    }

    if editable {
        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|name| ModelInfo::new(name, 128000))
            .collect();

        let updated_config = DeclarativeProviderConfig {
            name: id.to_string(),
            engine: Some(match provider_type {
                "openai_compatible" => ProviderEngine::OpenAI,
                "anthropic_compatible" => ProviderEngine::Anthropic,
                "ollama_compatible" => ProviderEngine::Ollama,
                _ => return Err(anyhow::anyhow!("Invalid provider type: {}", provider_type)),
            }),
            display_name,
            description: existing_config.description,
            api_key_env: existing_config.api_key_env,
            base_url: api_url,
            models: model_infos,
            headers: existing_config.headers,
            timeout_seconds: existing_config.timeout_seconds,
            supports_streaming,
            routes: existing_config.routes,
            config_keys: existing_config.config_keys,
            default_model: existing_config.default_model,
            auth: existing_config.auth,
        };

        let file_path = custom_providers_dir().join(format!("{}.json", id));
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
    // Check if this is a routed provider or single-engine provider
    if config.routes.is_some() {
        register_routed_provider(registry, config, provider_type);
    } else {
        register_single_engine_provider(registry, config, provider_type);
    }
}

/// Registers a single-engine provider (backwards compatible path).
fn register_single_engine_provider(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    config: DeclarativeProviderConfig,
    provider_type: ProviderType,
) {
    let config_clone = config.clone();
    let engine = match config.engine.clone() {
        Some(e) => e,
        None => {
            tracing::error!(
                "Provider '{}' has neither 'engine' nor 'routes' defined. This should not happen.",
                config.name
            );
            panic!(
                "Single-engine provider '{}' must have engine field",
                config.name
            );
        }
    };

    match engine {
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

/// Registers a routed provider that can dispatch to different engines based on model name.
fn register_routed_provider(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    config: DeclarativeProviderConfig,
    provider_type: ProviderType,
) {
    let config_clone = config.clone();

    // For routed providers, we use register_dynamic which returns Arc<dyn Provider>
    // This allows us to return different concrete provider types based on the route
    registry.register_dynamic(&config, provider_type, move |model| {
        create_routed_provider(model, config_clone.clone())
    });
}

/// Creates a provider instance based on the model name and route configuration.
fn create_routed_provider(
    model: crate::model::ModelConfig,
    config: DeclarativeProviderConfig,
) -> Result<std::sync::Arc<dyn crate::providers::base::Provider>> {
    use std::sync::Arc;

    let routes = config
        .routes
        .as_ref()
        .expect("Routed provider must have routes");
    let model_name = &model.model_name;

    // Find the matching route
    let route = select_route(model_name, routes).ok_or_else(|| {
        anyhow::anyhow!(
            "No route found for model '{}' in provider '{}'",
            model_name,
            config.name
        )
    })?;

    // Expand environment variables in base_url
    let expanded_base_url = expand_env_vars(&config.base_url)?;

    // Build the route-specific configuration
    let route_config = build_route_config(&config, route, &expanded_base_url)?;

    // Create the appropriate provider based on the route's engine
    match route.engine {
        ProviderEngine::OpenAI => {
            let provider = OpenAiProvider::from_custom_config(model, route_config)?;
            Ok(Arc::new(provider) as Arc<dyn crate::providers::base::Provider>)
        }
        ProviderEngine::Anthropic => {
            let provider = AnthropicProvider::from_custom_config(model, route_config)?;
            Ok(Arc::new(provider) as Arc<dyn crate::providers::base::Provider>)
        }
        ProviderEngine::Ollama => {
            let provider = OllamaProvider::from_custom_config(model, route_config)?;
            Ok(Arc::new(provider) as Arc<dyn crate::providers::base::Provider>)
        }
    }
}

/// Builds a route-specific DeclarativeProviderConfig from the base config and selected route.
fn build_route_config(
    base: &DeclarativeProviderConfig,
    route: &RouteConfig,
    expanded_base_url: &str,
) -> Result<DeclarativeProviderConfig> {
    // Construct full URL: base + path + query
    let full_url = construct_full_url(expanded_base_url, &route.path, &route.query)?;

    // Merge headers: base headers + route headers
    let mut merged_headers = base.headers.clone().unwrap_or_default();
    if let Some(route_headers) = &route.headers {
        merged_headers.extend(route_headers.clone());
    }

    Ok(DeclarativeProviderConfig {
        name: base.name.clone(),
        engine: Some(route.engine.clone()),
        display_name: base.display_name.clone(),
        description: base.description.clone(),
        api_key_env: base.api_key_env.clone(),
        base_url: full_url,
        models: base.models.clone(),
        headers: Some(merged_headers),
        timeout_seconds: base.timeout_seconds,
        supports_streaming: base.supports_streaming,
        routes: None, // Single-engine config for the route
        config_keys: base.config_keys.clone(),
        default_model: base.default_model.clone(),
        auth: route.auth.clone(),
    })
}

/// Constructs a full URL from base URL, path, and optional query parameters.
fn construct_full_url(
    base: &str,
    path: &str,
    query: &Option<HashMap<String, String>>,
) -> Result<String> {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    let mut url = if path.is_empty() {
        base.to_string()
    } else {
        format!("{}/{}", base, path)
    };

    if let Some(params) = query {
        if !params.is_empty() {
            let query_string: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query_string);
        }
    }

    Ok(url)
}

/// Matches a model name against a glob pattern.
///
/// Supports:
/// - Exact match: "gpt-4" matches "gpt-4"
/// - Prefix wildcard: "claude-*" matches "claude-sonnet-4-5"
/// - Suffix wildcard: "*-preview" matches "gpt-4-preview"
/// - Contains wildcard: "gpt-*-turbo" matches "gpt-4-turbo"
/// - Catch-all: "*" matches anything
pub fn model_matches_pattern(model_name: &str, pattern: &str) -> bool {
    // Catch-all
    if pattern == "*" {
        return true;
    }

    // Exact match (no wildcards)
    if !pattern.contains('*') {
        return model_name == pattern;
    }

    // Single wildcard matching
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let suffix = parts[1];

        // Prefix-only wildcard: "claude-*"
        if suffix.is_empty() {
            return model_name.starts_with(prefix);
        }

        // Suffix-only wildcard: "*-preview"
        if prefix.is_empty() {
            return model_name.ends_with(suffix);
        }

        // Contains wildcard: "gpt-*-turbo"
        return model_name.starts_with(prefix) && model_name.ends_with(suffix);
    }

    // Multiple wildcards not supported - fall back to exact match
    model_name == pattern
}

/// Selects the first matching route for a given model name.
///
/// Routes are evaluated in order, and the first match wins.
/// Returns None if no route matches.
pub fn select_route<'a>(model_name: &str, routes: &'a [RouteConfig]) -> Option<&'a RouteConfig> {
    routes
        .iter()
        .find(|route| model_matches_pattern(model_name, &route.match_pattern.model))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ URL Templating Tests ============

    #[test]
    fn test_expand_env_vars_single_variable() {
        let _guard = env_lock::lock_env([("TEST_RESOURCE", Some("myresource"))]);

        let result = expand_env_vars("https://${TEST_RESOURCE}.example.com").unwrap();
        assert_eq!(result, "https://myresource.example.com");
    }

    #[test]
    fn test_expand_env_vars_multiple_variables() {
        let _guard = env_lock::lock_env([
            ("TEST_HOST", Some("api")),
            ("TEST_DOMAIN", Some("example.com")),
        ]);

        let result = expand_env_vars("https://${TEST_HOST}.${TEST_DOMAIN}/v1").unwrap();
        assert_eq!(result, "https://api.example.com/v1");
    }

    #[test]
    fn test_expand_env_vars_missing_variable() {
        let _guard = env_lock::lock_env([("UNRELATED_VAR", Some("value"))]);

        let result = expand_env_vars("https://${MISSING_VAR}.example.com");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("MISSING_VAR"),
            "Error should mention missing var: {}",
            err
        );
    }

    #[test]
    fn test_expand_env_vars_no_variables() {
        let result = expand_env_vars("https://api.example.com/v1").unwrap();
        assert_eq!(result, "https://api.example.com/v1");
    }

    #[test]
    fn test_expand_env_vars_invalid_syntax_passthrough() {
        // Invalid patterns should pass through unchanged
        let result = expand_env_vars("https://${lowercase}.example.com").unwrap();
        assert_eq!(result, "https://${lowercase}.example.com");
    }

    // ============ Pattern Matching Tests ============

    #[test]
    fn test_model_matches_pattern_exact() {
        assert!(model_matches_pattern("gpt-4", "gpt-4"));
        assert!(!model_matches_pattern("gpt-4", "gpt-4o"));
        assert!(!model_matches_pattern("gpt-4o", "gpt-4"));
    }

    #[test]
    fn test_model_matches_pattern_prefix_wildcard() {
        assert!(model_matches_pattern("claude-sonnet-4-5", "claude-*"));
        assert!(model_matches_pattern("claude-haiku-4-5", "claude-*"));
        assert!(model_matches_pattern("claude-", "claude-*"));
        assert!(!model_matches_pattern("gpt-4", "claude-*"));
    }

    #[test]
    fn test_model_matches_pattern_suffix_wildcard() {
        assert!(model_matches_pattern("gpt-4-preview", "*-preview"));
        assert!(model_matches_pattern("model-preview", "*-preview"));
        assert!(!model_matches_pattern("gpt-4", "*-preview"));
    }

    #[test]
    fn test_model_matches_pattern_contains_wildcard() {
        assert!(model_matches_pattern("gpt-4-turbo", "gpt-*-turbo"));
        assert!(model_matches_pattern("gpt-3.5-turbo", "gpt-*-turbo"));
        assert!(!model_matches_pattern("gpt-4", "gpt-*-turbo"));
    }

    #[test]
    fn test_model_matches_pattern_catch_all() {
        assert!(model_matches_pattern("anything", "*"));
        assert!(model_matches_pattern("gpt-4", "*"));
        assert!(model_matches_pattern("claude-sonnet-4-5", "*"));
        assert!(model_matches_pattern("", "*"));
    }

    // ============ Route Selection Tests ============

    fn create_test_routes() -> Vec<RouteConfig> {
        vec![
            RouteConfig {
                match_pattern: ModelMatch {
                    model: "claude-*".to_string(),
                },
                engine: ProviderEngine::Anthropic,
                path: "anthropic/v1/messages".to_string(),
                auth: Some(AuthConfig {
                    auth_type: "header".to_string(),
                    name: "x-api-key".to_string(),
                }),
                headers: Some(HashMap::from([(
                    "anthropic-version".to_string(),
                    "2023-06-01".to_string(),
                )])),
                query: None,
            },
            RouteConfig {
                match_pattern: ModelMatch {
                    model: "*".to_string(),
                },
                engine: ProviderEngine::OpenAI,
                path: "models/chat/completions".to_string(),
                auth: Some(AuthConfig {
                    auth_type: "header".to_string(),
                    name: "api-key".to_string(),
                }),
                headers: None,
                query: Some(HashMap::from([(
                    "api-version".to_string(),
                    "2024-05-01-preview".to_string(),
                )])),
            },
        ]
    }

    #[test]
    fn test_select_route_claude_model() {
        let routes = create_test_routes();
        let route = select_route("claude-sonnet-4-5", &routes).unwrap();
        assert_eq!(route.engine, ProviderEngine::Anthropic);
        assert_eq!(route.path, "anthropic/v1/messages");
    }

    #[test]
    fn test_select_route_openai_model() {
        let routes = create_test_routes();
        let route = select_route("gpt-4o", &routes).unwrap();
        assert_eq!(route.engine, ProviderEngine::OpenAI);
        assert_eq!(route.path, "models/chat/completions");
    }

    #[test]
    fn test_select_route_first_match_wins() {
        let routes = create_test_routes();
        // "claude-sonnet-4-5" matches both "claude-*" and "*", but first match should win
        let route = select_route("claude-sonnet-4-5", &routes).unwrap();
        assert_eq!(route.engine, ProviderEngine::Anthropic);
    }

    #[test]
    fn test_select_route_no_match() {
        let routes: Vec<RouteConfig> = vec![RouteConfig {
            match_pattern: ModelMatch {
                model: "specific-model".to_string(),
            },
            engine: ProviderEngine::OpenAI,
            path: "v1/chat".to_string(),
            auth: None,
            headers: None,
            query: None,
        }];

        let route = select_route("different-model", &routes);
        assert!(route.is_none());
    }

    // ============ URL Construction Tests ============

    #[test]
    fn test_construct_full_url_base_and_path() {
        let url = construct_full_url("https://api.example.com", "v1/chat", &None).unwrap();
        assert_eq!(url, "https://api.example.com/v1/chat");
    }

    #[test]
    fn test_construct_full_url_trailing_slash() {
        let url = construct_full_url("https://api.example.com/", "/v1/chat/", &None).unwrap();
        assert_eq!(url, "https://api.example.com/v1/chat/");
    }

    #[test]
    fn test_construct_full_url_with_query() {
        let query = Some(HashMap::from([(
            "api-version".to_string(),
            "2024-01-01".to_string(),
        )]));
        let url = construct_full_url("https://api.example.com", "v1/chat", &query).unwrap();
        assert_eq!(
            url,
            "https://api.example.com/v1/chat?api-version=2024-01-01"
        );
    }

    #[test]
    fn test_construct_full_url_empty_path() {
        let url = construct_full_url("https://api.example.com", "", &None).unwrap();
        assert_eq!(url, "https://api.example.com");
    }

    // ============ Schema Parsing Tests ============

    #[test]
    fn test_parse_simple_config_backwards_compatible() {
        let json = r#"{
            "name": "test",
            "engine": "openai",
            "display_name": "Test",
            "api_key_env": "TEST_KEY",
            "base_url": "https://api.test.com",
            "models": []
        }"#;

        let config: DeclarativeProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.engine, Some(ProviderEngine::OpenAI));
        assert!(config.routes.is_none());
    }

    #[test]
    fn test_parse_routed_config() {
        let json = r#"{
            "name": "foundry",
            "display_name": "Microsoft Foundry",
            "api_key_env": "FOUNDRY_KEY",
            "base_url": "https://${RESOURCE}.example.com",
            "models": [],
            "routes": [
                {
                    "match": {"model": "claude-*"},
                    "engine": "anthropic",
                    "path": "anthropic/v1/messages"
                },
                {
                    "match": {"model": "*"},
                    "engine": "openai",
                    "path": "v1/chat/completions"
                }
            ]
        }"#;

        let config: DeclarativeProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "foundry");
        assert!(config.engine.is_none());
        assert!(config.routes.is_some());

        let routes = config.routes.unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].engine, ProviderEngine::Anthropic);
        assert_eq!(routes[1].engine, ProviderEngine::OpenAI);
    }

    #[test]
    fn test_parse_route_with_all_fields() {
        let json = r#"{
            "match": {"model": "test-*"},
            "engine": "openai",
            "path": "v1/chat",
            "auth": {"type": "header", "name": "x-api-key"},
            "headers": {"X-Custom": "value"},
            "query": {"version": "1.0"}
        }"#;

        let route: RouteConfig = serde_json::from_str(json).unwrap();
        assert_eq!(route.match_pattern.model, "test-*");
        assert_eq!(route.engine, ProviderEngine::OpenAI);
        assert_eq!(route.path, "v1/chat");
        assert!(route.auth.is_some());
        assert!(route.headers.is_some());
        assert!(route.query.is_some());
    }

    #[test]
    fn test_load_fixed_providers_all_have_engine_or_routes() {
        let providers = super::load_fixed_providers().expect("Failed to load fixed providers");
        assert!(
            !providers.is_empty(),
            "Should have at least one fixed provider"
        );

        for config in providers {
            let has_engine = config.engine.is_some();
            let has_routes = config.routes.is_some();

            assert!(
                has_engine || has_routes,
                "Provider '{}' must have either 'engine' or 'routes' defined, but has neither",
                config.name
            );

            // If has routes, verify routes are non-empty
            if let Some(routes) = &config.routes {
                assert!(
                    !routes.is_empty(),
                    "Provider '{}' has empty routes array",
                    config.name
                );
            }
        }
    }

    #[test]
    fn test_register_declarative_providers_succeeds() {
        use crate::providers::provider_registry::ProviderRegistry;

        let mut registry = ProviderRegistry::new();
        super::register_declarative_providers(&mut registry)
            .expect("register_declarative_providers should succeed");

        // Check that microsoft_foundry is registered
        let all_metadata = registry.all_metadata_with_types();
        let has_ms_foundry = all_metadata
            .iter()
            .any(|(m, _)| m.name == "microsoft_foundry");
        assert!(
            has_ms_foundry,
            "microsoft_foundry should be registered as a provider"
        );
    }
}
