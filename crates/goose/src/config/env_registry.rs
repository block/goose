use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;

/// Source of a configuration value
#[derive(Debug, Clone, PartialEq)]
pub enum ValueSource {
    /// Value came from environment variable
    Environment,
    /// Value came from config file (YAML)
    ConfigFile,
    /// Value came from keyring/secret storage
    Secret,
    /// Value is a default value
    Default,
    /// Value was not found
    NotFound,
}

/// A configuration value with its source
#[derive(Debug, Clone)]
pub struct TrackedValue {
    pub value: Value,
    pub source: ValueSource,
}

/// Centralized registry for environment variables loaded at process start.
/// This eliminates just-in-time environment variable access and provides
/// a structured way to manage all environment variables used by Goose.
pub struct EnvRegistry {
    /// All environment variables loaded at startup
    env_vars: HashMap<String, String>,
    /// Pre-parsed and validated values - all parsing done at startup
    parsed_values: HashMap<String, Value>,
    /// Track the source of each value
    value_sources: HashMap<String, ValueSource>,
}

/// Global environment registry instance
pub static ENV_REGISTRY: Lazy<EnvRegistry> = Lazy::new(EnvRegistry::new);

/// Categories of environment variables used by Goose
#[derive(Debug, Clone, PartialEq)]
pub enum EnvCategory {
    /// Provider configuration (API keys, endpoints, etc.)
    Provider,
    /// Goose core configuration
    Core,
    /// UI/CLI configuration
    Interface,
    /// Development/debugging
    Debug,
    /// System environment (HOME, PATH, etc.)
    System,
    /// Scheduler configuration
    Scheduler,
    /// Extension/MCP configuration
    Extension,
    /// Tracing/observability
    Tracing,
}

/// Expected type for an environment variable
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EnvVarType {
    /// String value (default if not specified)
    #[default]
    String,
    /// Integer value
    Integer,
    /// Float value
    Float,
    /// Boolean value
    Boolean,
    /// JSON object
    JsonObject,
    /// JSON array
    JsonArray,
}

/// Known environment variables with their categories and whether they're secrets
pub struct EnvVarSpec {
    pub name: &'static str,
    pub category: EnvCategory,
    pub is_secret: bool,
    pub description: &'static str,
    pub var_type: EnvVarType,
}

/// Registry of all known environment variables used by Goose
pub const KNOWN_ENV_VARS: &[EnvVarSpec] = &[
    // Provider secrets
    EnvVarSpec {
        name: "OPENAI_API_KEY",
        category: EnvCategory::Provider,
        is_secret: true,
        description: "OpenAI API key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "ANTHROPIC_API_KEY",
        category: EnvCategory::Provider,
        is_secret: true,
        description: "Anthropic API key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOGLE_API_KEY",
        category: EnvCategory::Provider,
        is_secret: true,
        description: "Google API key",
        var_type: EnvVarType::String,
    },
    // Provider configuration
    EnvVarSpec {
        name: "GOOSE_MODEL",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Default model to use",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_LEAD_MODEL",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Lead model for multi-model setup",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_LEAD_PROVIDER",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Lead provider for multi-model setup",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_LEAD_TURNS",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Number of turns for lead model",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_LEAD_FAILURE_THRESHOLD",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Failure threshold for lead model",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_LEAD_FALLBACK_TURNS",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Fallback turns for lead model",
        var_type: EnvVarType::Integer,
    },
    // Core configuration
    EnvVarSpec {
        name: "GOOSE_CONTEXT_LIMIT",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Context limit for models",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_WORKER_CONTEXT_LIMIT",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Context limit for worker models",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_TEMPERATURE",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Temperature setting for models",
        var_type: EnvVarType::Float,
    },
    EnvVarSpec {
        name: "GOOSE_MODE",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Operating mode",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_DISABLE_KEYRING",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Disable keyring for secret storage",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "GOOSE_CACHE_DIR",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Cache directory path",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_WORKING_DIR",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Working directory path",
        var_type: EnvVarType::String,
    },
    // Toolshim configuration
    EnvVarSpec {
        name: "GOOSE_TOOLSHIM",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Enable toolshim functionality",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "GOOSE_TOOLSHIM_OLLAMA_MODEL",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Ollama model for toolshim",
        var_type: EnvVarType::String,
    },
    // Router configuration
    EnvVarSpec {
        name: "GOOSE_ROUTER_TOOL_SELECTION_STRATEGY",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Tool selection strategy for router",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_EMBEDDING_MODEL_PROVIDER",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Embedding model provider",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_EMBEDDING_MODEL",
        category: EnvCategory::Provider,
        is_secret: false,
        description: "Embedding model name",
        var_type: EnvVarType::String,
    },
    // Interface configuration
    EnvVarSpec {
        name: "GOOSE_CLI_THEME",
        category: EnvCategory::Interface,
        is_secret: false,
        description: "CLI theme",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_CLI_SHOW_THINKING",
        category: EnvCategory::Interface,
        is_secret: false,
        description: "Show thinking in CLI",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "GOOSE_CLI_MIN_PRIORITY",
        category: EnvCategory::Interface,
        is_secret: false,
        description: "Minimum priority for CLI display",
        var_type: EnvVarType::Integer,
    },
    // Scheduler configuration
    EnvVarSpec {
        name: "GOOSE_SCHEDULER_TYPE",
        category: EnvCategory::Scheduler,
        is_secret: false,
        description: "Scheduler type (temporal/legacy)",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_TEMPORAL_BIN",
        category: EnvCategory::Scheduler,
        is_secret: false,
        description: "Path to temporal binary",
        var_type: EnvVarType::String,
    },
    // Debug configuration
    EnvVarSpec {
        name: "CLAUDE_THINKING_ENABLED",
        category: EnvCategory::Debug,
        is_secret: false,
        description: "Enable Claude thinking mode",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "CLAUDE_THINKING_BUDGET",
        category: EnvCategory::Debug,
        is_secret: false,
        description: "Claude thinking token budget",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_GEMINI_CLI_DEBUG",
        category: EnvCategory::Debug,
        is_secret: false,
        description: "Enable Gemini CLI debug mode",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "GOOSE_CLAUDE_CODE_DEBUG",
        category: EnvCategory::Debug,
        is_secret: false,
        description: "Enable Claude Code debug mode",
        var_type: EnvVarType::Boolean,
    },
    // Tracing configuration
    EnvVarSpec {
        name: "LANGFUSE_PUBLIC_KEY",
        category: EnvCategory::Tracing,
        is_secret: true,
        description: "Langfuse public key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "LANGFUSE_SECRET_KEY",
        category: EnvCategory::Tracing,
        is_secret: true,
        description: "Langfuse secret key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "LANGFUSE_URL",
        category: EnvCategory::Tracing,
        is_secret: false,
        description: "Langfuse URL",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "LANGFUSE_INIT_PROJECT_PUBLIC_KEY",
        category: EnvCategory::Tracing,
        is_secret: true,
        description: "Langfuse init project public key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "LANGFUSE_INIT_PROJECT_SECRET_KEY",
        category: EnvCategory::Tracing,
        is_secret: true,
        description: "Langfuse init project secret key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "OTEL_EXPORTER_OTLP_ENDPOINT",
        category: EnvCategory::Tracing,
        is_secret: false,
        description: "OTLP exporter endpoint",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "OTEL_EXPORTER_OTLP_TIMEOUT",
        category: EnvCategory::Tracing,
        is_secret: false,
        description: "OTLP exporter timeout",
        var_type: EnvVarType::Integer,
    },
    // System environment
    EnvVarSpec {
        name: "HOME",
        category: EnvCategory::System,
        is_secret: false,
        description: "User home directory",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "USER",
        category: EnvCategory::System,
        is_secret: false,
        description: "Current user name",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "USERNAME",
        category: EnvCategory::System,
        is_secret: false,
        description: "Current user name (Windows)",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "PATH",
        category: EnvCategory::System,
        is_secret: false,
        description: "System PATH",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "TEMP",
        category: EnvCategory::System,
        is_secret: false,
        description: "Temporary directory",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "DISPLAY",
        category: EnvCategory::System,
        is_secret: false,
        description: "X11 display",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "WAYLAND_DISPLAY",
        category: EnvCategory::System,
        is_secret: false,
        description: "Wayland display",
        var_type: EnvVarType::String,
    },
    // Extension/MCP configuration
    EnvVarSpec {
        name: "GOOGLE_DRIVE_OAUTH_PATH",
        category: EnvCategory::Extension,
        is_secret: false,
        description: "Google Drive OAuth path",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOGLE_DRIVE_CREDENTIALS_PATH",
        category: EnvCategory::Extension,
        is_secret: false,
        description: "Google Drive credentials path",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOGLE_DRIVE_OAUTH_CONFIG",
        category: EnvCategory::Extension,
        is_secret: true,
        description: "Google Drive OAuth config",
        var_type: EnvVarType::JsonObject,
    },
    EnvVarSpec {
        name: "GOOSE_EDITOR_API_KEY",
        category: EnvCategory::Extension,
        is_secret: true,
        description: "Goose editor API key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_EDITOR_HOST",
        category: EnvCategory::Extension,
        is_secret: false,
        description: "Goose editor host",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_EDITOR_MODEL",
        category: EnvCategory::Extension,
        is_secret: false,
        description: "Goose editor model",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "CONTEXT_FILE_NAMES",
        category: EnvCategory::Extension,
        is_secret: false,
        description: "Context file names for developer extension",
        var_type: EnvVarType::String,
    },
    // Testing configuration
    EnvVarSpec {
        name: "GOOSE_TEST_PROVIDER",
        category: EnvCategory::Debug,
        is_secret: false,
        description: "Provider for testing",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GITHUB_ACTIONS",
        category: EnvCategory::System,
        is_secret: false,
        description: "GitHub Actions environment indicator",
        var_type: EnvVarType::String,
    },
    // Recipe configuration
    EnvVarSpec {
        name: "GOOSE_RECIPE_PATH",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Recipe search path",
        var_type: EnvVarType::String,
    },
    // Subagent configuration
    EnvVarSpec {
        name: "GOOSE_SUBAGENT_MAX_TURNS",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Maximum turns for subagents",
        var_type: EnvVarType::Integer,
    },
    // Server configuration
    EnvVarSpec {
        name: "GOOSE_SERVER__SECRET_KEY",
        category: EnvCategory::Core,
        is_secret: true,
        description: "Server secret key",
        var_type: EnvVarType::String,
    },
    EnvVarSpec {
        name: "GOOSE_SERVER__MEMORY",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Enable server memory",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "GOOSE_SERVER__COMPUTER_CONTROLLER",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Enable computer controller",
        var_type: EnvVarType::Boolean,
    },
    EnvVarSpec {
        name: "PORT",
        category: EnvCategory::System,
        is_secret: false,
        description: "Server port",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_PORT",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Goose server port",
        var_type: EnvVarType::Integer,
    },
    EnvVarSpec {
        name: "GOOSE_API_HOST",
        category: EnvCategory::Core,
        is_secret: false,
        description: "Goose API host",
        var_type: EnvVarType::String,
    },
];

/// Automatically discover environment variables from provider metadata
/// Uses the structured provider factory to get actual config keys
pub fn discover_provider_env_vars() -> Vec<EnvVarSpec> {
    let mut discovered = Vec::new();

    // Get all provider metadata from the factory
    let providers = crate::providers::providers();

    for provider_metadata in providers {
        for config_key in &provider_metadata.config_keys {
            discovered.push(EnvVarSpec {
                name: Box::leak(config_key.name.clone().into_boxed_str()),
                category: EnvCategory::Provider,
                is_secret: config_key.secret,
                description: Box::leak(
                    format!("{} - {}", provider_metadata.display_name, config_key.name)
                        .into_boxed_str(),
                ),
                var_type: EnvVarType::String, // Default to string for provider config
            });
        }
    }

    discovered
}

/// Automatically discover environment variables from extension configurations
/// Uses the structured YAML configuration to extract env_keys from extensions
pub fn discover_extension_env_vars() -> Vec<String> {
    let mut discovered = Vec::new();

    // Get all configured extensions and extract their env_keys from the structured config
    if let Ok(config) = super::Config::global().load_values() {
        if let Some(extensions_value) = config.get("extensions") {
            if let Ok(extensions) = serde_json::from_value::<HashMap<String, serde_json::Value>>(
                extensions_value.clone(),
            ) {
                for (_extension_key, extension_value) in extensions {
                    // Extract env_keys from extension configuration
                    if let Some(env_keys) = extension_value.get("env_keys") {
                        if let Ok(keys) = serde_json::from_value::<Vec<String>>(env_keys.clone()) {
                            for key in keys {
                                if !discovered.contains(&key) {
                                    discovered.push(key);
                                }
                            }
                        }
                    }

                    // Also check for envs map (legacy support)
                    if let Some(envs) = extension_value.get("envs") {
                        if let Ok(env_map) =
                            serde_json::from_value::<HashMap<String, String>>(envs.clone())
                        {
                            for key in env_map.keys() {
                                if !discovered.contains(key) {
                                    discovered.push(key.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    discovered
}

impl Default for EnvRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvRegistry {
    /// Create a new environment registry by loading all environment variables at startup
    pub fn new() -> Self {
        let env_vars: HashMap<String, String> = env::vars().collect();

        // Parse all environment variables at startup
        let mut parsed_values = HashMap::new();
        let mut value_sources = HashMap::new();

        for (key, value) in &env_vars {
            if let Ok(parsed) = EnvRegistry::parse_env_value(value) {
                parsed_values.insert(key.clone(), parsed);
                value_sources.insert(key.clone(), ValueSource::Environment);

                // Also add uppercase version for flexible lookup
                let upper_key = key.to_uppercase();
                if upper_key != *key {
                    parsed_values.insert(
                        upper_key.clone(),
                        EnvRegistry::parse_env_value(value).unwrap(),
                    );
                    value_sources.insert(upper_key, ValueSource::Environment);
                }
            }
        }

        Self {
            env_vars,
            parsed_values,
            value_sources,
        }
    }

    /// Create a new environment registry with custom values (for testing)
    pub fn with_values(values: HashMap<String, String>) -> Self {
        let mut parsed_values = HashMap::new();
        let mut value_sources = HashMap::new();

        for (key, value) in &values {
            if let Ok(parsed) = EnvRegistry::parse_env_value(value) {
                parsed_values.insert(key.clone(), parsed);
                value_sources.insert(key.clone(), ValueSource::Environment);

                // Also add uppercase version for flexible lookup
                let upper_key = key.to_uppercase();
                if upper_key != *key {
                    parsed_values.insert(
                        upper_key.clone(),
                        EnvRegistry::parse_env_value(value).unwrap(),
                    );
                    value_sources.insert(upper_key, ValueSource::Environment);
                }
            }
        }

        Self {
            env_vars: values,
            parsed_values,
            value_sources,
        }
    }

    /// Get an environment variable value as a string
    pub fn get_raw(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    /// Get an environment variable value with case-insensitive lookup
    /// First tries exact match, then uppercase
    pub fn get_raw_flexible(&self, key: &str) -> Option<&String> {
        // First try exact match
        if let Some(value) = self.env_vars.get(key) {
            return Some(value);
        }

        // Then try uppercase
        let upper_key = key.to_uppercase();
        self.env_vars.get(&upper_key)
    }

    /// Get a pre-parsed environment variable value - thin wrapper, all parsing done at startup
    pub fn get_parsed(&self, key: &str) -> Option<Value> {
        // First try exact match
        if let Some(value) = self.parsed_values.get(key) {
            return Some(value.clone());
        }

        // Then try uppercase
        let upper_key = key.to_uppercase();
        self.parsed_values.get(&upper_key).cloned()
    }

    /// Get a typed value from the environment registry with type validation
    pub fn get_typed<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        let value = self.get_parsed(key)?;

        // Check if this is a known env var with a specific type
        if let Some(spec) = self.find_spec(key) {
            // Validate the type matches what's expected
            if !self.validate_type(&value, &spec.var_type) {
                tracing::warn!(
                    "Environment variable {} expected type {:?} but got incompatible value",
                    key,
                    spec.var_type
                );
            }
        }

        serde_json::from_value(value).ok()
    }

    /// Find the spec for a given environment variable
    fn find_spec(&self, key: &str) -> Option<&'static EnvVarSpec> {
        let upper_key = key.to_uppercase();

        // Use iterator instead of manual loop
        KNOWN_ENV_VARS
            .iter()
            .find(|&spec| spec.name == key || spec.name == upper_key)
    }

    /// Validate that a value matches the expected type
    fn validate_type(&self, value: &Value, expected_type: &EnvVarType) -> bool {
        match (value, expected_type) {
            (Value::String(_), EnvVarType::String) => true,
            (Value::Number(n), EnvVarType::Integer) => n.is_i64() || n.is_u64(),
            (Value::Number(_), EnvVarType::Float) => true,
            (Value::Bool(_), EnvVarType::Boolean) => true,
            (Value::Object(_), EnvVarType::JsonObject) => true,
            (Value::Array(_), EnvVarType::JsonArray) => true,
            // Allow strings for any type (they can be parsed)
            (Value::String(_), _) => true,
            _ => false,
        }
    }

    /// Get a value with its source information
    pub fn get_tracked(&self, key: &str) -> TrackedValue {
        // First try exact match
        if let Some(value) = self.parsed_values.get(key) {
            let source = self
                .value_sources
                .get(key)
                .cloned()
                .unwrap_or(ValueSource::Environment);
            return TrackedValue {
                value: value.clone(),
                source,
            };
        }

        // Then try uppercase
        let upper_key = key.to_uppercase();
        if let Some(value) = self.parsed_values.get(&upper_key) {
            let source = self
                .value_sources
                .get(&upper_key)
                .cloned()
                .unwrap_or(ValueSource::Environment);
            return TrackedValue {
                value: value.clone(),
                source,
            };
        }

        // Not found
        TrackedValue {
            value: Value::Null,
            source: ValueSource::NotFound,
        }
    }

    /// Get the source of a value
    pub fn get_source(&self, key: &str) -> ValueSource {
        // First try exact match
        if let Some(source) = self.value_sources.get(key) {
            return source.clone();
        }

        // Then try uppercase
        let upper_key = key.to_uppercase();
        if let Some(source) = self.value_sources.get(&upper_key) {
            return source.clone();
        }

        ValueSource::NotFound
    }

    /// Check if an environment variable exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.env_vars.contains_key(key) || self.env_vars.contains_key(&key.to_uppercase())
    }

    /// Get all environment variables in a specific category
    pub fn get_by_category(&self, _category: EnvCategory) -> HashMap<String, String> {
        let mut result = HashMap::new();

        for spec in KNOWN_ENV_VARS {
            if std::mem::discriminant(&spec.category) == std::mem::discriminant(&_category) {
                if let Some(value) = self.get_raw(spec.name) {
                    result.insert(spec.name.to_string(), value.clone());
                }
            }
        }

        result
    }

    /// Get all secret environment variables
    pub fn get_secrets(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();

        for spec in KNOWN_ENV_VARS {
            if spec.is_secret {
                if let Some(value) = self.get_raw(spec.name) {
                    result.insert(spec.name.to_string(), value.clone());
                }
            }
        }

        result
    }

    /// Get all non-secret environment variables
    pub fn get_params(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();

        for spec in KNOWN_ENV_VARS {
            if !spec.is_secret {
                if let Some(value) = self.get_raw(spec.name) {
                    result.insert(spec.name.to_string(), value.clone());
                }
            }
        }

        result
    }

    /// Parse an environment variable value into a JSON Value
    /// This mirrors the logic from Config::parse_env_value
    fn parse_env_value(val: &str) -> Result<Value, serde_json::Error> {
        // First try JSON parsing - this handles quoted strings, objects, arrays, etc.
        if let Ok(json_value) = serde_json::from_str(val) {
            return Ok(json_value);
        }

        let trimmed = val.trim();

        match trimmed.to_lowercase().as_str() {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            _ => {}
        }

        if let Ok(int_val) = trimmed.parse::<i64>() {
            return Ok(Value::Number(int_val.into()));
        }

        if let Ok(float_val) = trimmed.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                return Ok(Value::Number(num));
            }
        }

        Ok(Value::String(val.to_string()))
    }

    /// Get diagnostic information about environment variables
    pub fn get_diagnostics(&self) -> EnvDiagnostics {
        let mut known_found = Vec::new();
        let mut known_missing = Vec::new();
        let mut unknown_goose_vars = Vec::new();

        // Check known variables
        for spec in KNOWN_ENV_VARS {
            if self.contains_key(spec.name) {
                known_found.push(spec.name);
            } else {
                known_missing.push(spec.name);
            }
        }

        // Check discovered provider variables
        let discovered_providers = discover_provider_env_vars();
        for spec in &discovered_providers {
            if self.contains_key(spec.name) && !known_found.contains(&spec.name) {
                // Add to found if not already in the static list
                known_found.push(spec.name);
            }
        }

        // Check discovered extension variables
        let discovered_extensions = discover_extension_env_vars();
        for key in &discovered_extensions {
            if self.contains_key(key) && !known_found.iter().any(|&k| k == key) {
                // Convert to static str for compatibility (this is a limitation of the current design)
                // In a real implementation, we'd want to change the diagnostics structure
            }
        }

        // Find unknown GOOSE_* variables
        for key in self.env_vars.keys() {
            if key.starts_with("GOOSE_")
                && !KNOWN_ENV_VARS.iter().any(|spec| spec.name == key)
                && !discovered_providers.iter().any(|spec| spec.name == key)
            {
                unknown_goose_vars.push(key.clone());
            }
        }

        EnvDiagnostics {
            total_env_vars: self.env_vars.len(),
            known_found,
            known_missing,
            unknown_goose_vars,
        }
    }
}

/// Diagnostic information about environment variables
#[derive(Debug)]
pub struct EnvDiagnostics {
    pub total_env_vars: usize,
    pub known_found: Vec<&'static str>,
    pub known_missing: Vec<&'static str>,
    pub unknown_goose_vars: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_registry_creation() {
        let registry = EnvRegistry::new();
        assert!(!registry.env_vars.is_empty());
    }

    #[test]
    fn test_flexible_lookup() {
        env::set_var("TEST_FLEXIBLE_UPPER", "upper_value");
        let registry = EnvRegistry::new();

        // Should find with exact match
        assert_eq!(
            registry.get_raw_flexible("TEST_FLEXIBLE_UPPER"),
            Some(&"upper_value".to_string())
        );

        // Should find with case conversion
        assert_eq!(
            registry.get_raw_flexible("test_flexible_upper"),
            Some(&"upper_value".to_string())
        );

        env::remove_var("TEST_FLEXIBLE_UPPER");
    }

    #[test]
    fn test_value_parsing() {
        let registry = EnvRegistry::new();

        // Test boolean parsing
        let parsed = EnvRegistry::parse_env_value("true").unwrap();
        assert_eq!(parsed, Value::Bool(true));

        // Test number parsing
        let parsed = EnvRegistry::parse_env_value("42").unwrap();
        assert_eq!(parsed, Value::Number(42.into()));

        // Test string parsing
        let parsed = EnvRegistry::parse_env_value("hello").unwrap();
        assert_eq!(parsed, Value::String("hello".to_string()));
    }

    #[test]
    fn test_category_filtering() {
        env::set_var("GOOSE_MODEL", "test_model");
        let registry = EnvRegistry::new();

        let provider_vars = registry.get_by_category(EnvCategory::Provider);
        assert!(provider_vars.contains_key("GOOSE_MODEL"));

        env::remove_var("GOOSE_MODEL");
    }

    #[test]
    fn test_secrets_filtering() {
        env::set_var("OPENAI_API_KEY", "test_key");
        let registry = EnvRegistry::new();

        let secrets = registry.get_secrets();
        assert!(secrets.contains_key("OPENAI_API_KEY"));

        let params = registry.get_params();
        assert!(!params.contains_key("OPENAI_API_KEY"));

        env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn test_diagnostics() {
        env::set_var("GOOSE_MODEL", "test_model");
        env::set_var("GOOSE_UNKNOWN_VAR", "unknown");
        let registry = EnvRegistry::new();

        let diagnostics = registry.get_diagnostics();
        assert!(diagnostics.known_found.contains(&"GOOSE_MODEL"));
        assert!(diagnostics
            .unknown_goose_vars
            .iter()
            .any(|s| s == "GOOSE_UNKNOWN_VAR"));

        env::remove_var("GOOSE_MODEL");
        env::remove_var("GOOSE_UNKNOWN_VAR");
    }

    #[test]
    fn test_parsed_values_at_startup() {
        let env_vars = HashMap::from([
            ("TEST_STRING".to_string(), "hello".to_string()),
            ("TEST_NUMBER".to_string(), "42".to_string()),
            ("TEST_BOOL".to_string(), "true".to_string()),
            ("TEST_JSON".to_string(), r#"{"key": "value"}"#.to_string()),
        ]);

        let registry = EnvRegistry::with_values(env_vars);

        // Test that values are pre-parsed correctly
        assert_eq!(
            registry.get_typed::<String>("TEST_STRING"),
            Some("hello".to_string())
        );
        assert_eq!(registry.get_typed::<i32>("TEST_NUMBER"), Some(42));
        assert_eq!(registry.get_typed::<bool>("TEST_BOOL"), Some(true));

        // Test JSON parsing
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            key: String,
        }
        assert_eq!(
            registry.get_typed::<TestStruct>("TEST_JSON"),
            Some(TestStruct {
                key: "value".to_string()
            })
        );
    }

    #[test]
    fn test_structured_provider_discovery() {
        let discovered = discover_provider_env_vars();

        // Should find actual provider config keys from the structured metadata
        let key_names: Vec<&str> = discovered.iter().map(|spec| spec.name).collect();

        // These should be found from the actual provider metadata
        assert!(key_names.contains(&"ANTHROPIC_API_KEY"));
        assert!(key_names.contains(&"ANTHROPIC_HOST"));
        assert!(key_names.contains(&"OPENAI_API_KEY"));

        // Verify proper categorization and secret detection
        for spec in &discovered {
            assert_eq!(spec.category, EnvCategory::Provider);
            // API keys should be marked as secrets
            if spec.name.ends_with("_API_KEY") {
                assert!(
                    spec.is_secret,
                    "API key {} should be marked as secret",
                    spec.name
                );
            }
        }

        // Should have more than just a few hardcoded values
        assert!(
            discovered.len() > 10,
            "Should discover many provider config keys, found {}",
            discovered.len()
        );
    }

    #[test]
    fn test_structured_extension_discovery() {
        // This test may not find anything if no extensions are configured,
        // but it should not panic and should return a valid (possibly empty) vector
        let discovered = discover_extension_env_vars();

        // Should return a valid vector (may be empty if no extensions configured)
        assert!(discovered.len() >= 0);

        // All discovered keys should be non-empty strings
        for key in &discovered {
            assert!(!key.is_empty(), "Extension env key should not be empty");
        }
    }

    #[test]
    fn test_value_source_tracking() {
        let env_vars = HashMap::from([
            ("TEST_ENV_VAR".to_string(), "env_value".to_string()),
            ("TEST_NUMBER".to_string(), "42".to_string()),
        ]);

        let registry = EnvRegistry::with_values(env_vars);

        // Test getting tracked value for existing env var
        let tracked = registry.get_tracked("TEST_ENV_VAR");
        assert_eq!(tracked.source, ValueSource::Environment);
        assert_eq!(tracked.value, Value::String("env_value".to_string()));

        // Test getting tracked value for non-existent var
        let tracked = registry.get_tracked("NON_EXISTENT");
        assert_eq!(tracked.source, ValueSource::NotFound);
        assert_eq!(tracked.value, Value::Null);

        // Test getting source directly
        assert_eq!(
            registry.get_source("TEST_ENV_VAR"),
            ValueSource::Environment
        );
        assert_eq!(registry.get_source("TEST_NUMBER"), ValueSource::Environment);
        assert_eq!(registry.get_source("NON_EXISTENT"), ValueSource::NotFound);

        // Test case-insensitive lookup for source
        assert_eq!(
            registry.get_source("test_env_var"),
            ValueSource::Environment
        );
    }

    #[test]
    fn test_value_source_with_uppercase() {
        let env_vars = HashMap::from([("test_lower".to_string(), "lower_value".to_string())]);

        let registry = EnvRegistry::with_values(env_vars);

        // Test that uppercase lookup also gets correct source
        let tracked = registry.get_tracked("TEST_LOWER");
        assert_eq!(tracked.source, ValueSource::Environment);
        assert_eq!(tracked.value, Value::String("lower_value".to_string()));

        // Test source lookup with different cases
        assert_eq!(registry.get_source("test_lower"), ValueSource::Environment);
        assert_eq!(registry.get_source("TEST_LOWER"), ValueSource::Environment);
    }

    #[test]
    fn test_type_validation() {
        // Test that type validation works for known env vars
        let env_vars = HashMap::from([
            // Integer type
            ("GOOSE_CONTEXT_LIMIT".to_string(), "1000".to_string()),
            // Boolean type
            ("GOOSE_DISABLE_KEYRING".to_string(), "true".to_string()),
            // Float type
            ("GOOSE_TEMPERATURE".to_string(), "0.7".to_string()),
            // String type
            ("GOOSE_MODEL".to_string(), "gpt-4".to_string()),
        ]);

        let registry = EnvRegistry::with_values(env_vars);

        // Test integer retrieval
        let limit: Option<i32> = registry.get_typed("GOOSE_CONTEXT_LIMIT");
        assert_eq!(limit, Some(1000));

        // Test boolean retrieval
        let disabled: Option<bool> = registry.get_typed("GOOSE_DISABLE_KEYRING");
        assert_eq!(disabled, Some(true));

        // Test float retrieval
        let temp: Option<f64> = registry.get_typed("GOOSE_TEMPERATURE");
        assert_eq!(temp, Some(0.7));

        // Test string retrieval
        let model: Option<String> = registry.get_typed("GOOSE_MODEL");
        assert_eq!(model, Some("gpt-4".to_string()));
    }
}
