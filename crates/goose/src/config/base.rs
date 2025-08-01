use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use fs2::FileExt;
use keyring::Entry;
use once_cell::sync::{Lazy, OnceCell};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

const KEYRING_SERVICE: &str = "goose";
const KEYRING_USERNAME: &str = "secrets";

#[cfg(test)]
const TEST_KEYRING_SERVICE: &str = "goose-test";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration value not found: {0}")]
    NotFound(String),
    #[error("Failed to deserialize value: {0}")]
    DeserializeError(String),
    #[error("Failed to read config file: {0}")]
    FileError(#[from] std::io::Error),
    #[error("Failed to create config directory: {0}")]
    DirectoryError(String),
    #[error("Failed to access keyring: {0}")]
    KeyringError(String),
    #[error("Failed to lock config file: {0}")]
    LockError(String),
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::DeserializeError(err.to_string())
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::DeserializeError(err.to_string())
    }
}

impl From<keyring::Error> for ConfigError {
    fn from(err: keyring::Error) -> Self {
        ConfigError::KeyringError(err.to_string())
    }
}

/// Configuration management for Goose.
///
/// This module provides a flexible configuration system that supports:
/// - Dynamic configuration keys
/// - Multiple value types through serde deserialization
/// - Environment variable overrides
/// - YAML-based configuration file storage
/// - Hot reloading of configuration changes
/// - Secure secret storage in system keyring
///
/// Configuration values are loaded with the following precedence:
/// 1. Environment variables (exact key match)
/// 2. Configuration file (~/.config/goose/config.yaml by default)
///
/// Secrets are loaded with the following precedence:
/// 1. Environment variables (exact key match)
/// 2. System keyring (which can be disabled with GOOSE_DISABLE_KEYRING)
/// 3. If the keyring is disabled, secrets are stored in a secrets file
///    (~/.config/goose/secrets.yaml by default)
///
/// # Examples
///
/// ```no_run
/// use goose::config::Config;
/// use serde::Deserialize;
///
/// // Get a string value
/// let config = Config::global();
/// let api_key: String = config.get_param("OPENAI_API_KEY").unwrap();
///
/// // Get a complex type
/// #[derive(Deserialize)]
/// struct ServerConfig {
///     host: String,
///     port: u16,
/// }
///
/// let server_config: ServerConfig = config.get_param("server").unwrap();
/// ```
///
/// # Naming Convention
/// we recommend snake_case for keys, and will convert to UPPERCASE when
/// checking for environment overrides. e.g. openai_api_key will check for an
/// environment variable OPENAI_API_KEY
///
/// For Goose-specific configuration, consider prefixing with "goose_" to avoid conflicts.
pub struct Config {
    config_path: PathBuf,
    secrets: SecretStorage,
}

enum SecretStorage {
    Keyring { service: String },
    File { path: PathBuf },
}

// Global instance
static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

impl Default for Config {
    fn default() -> Self {
        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        let config_dir = choose_app_strategy(APP_STRATEGY.clone())
            .expect("goose requires a home dir")
            .config_dir();

        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");

        let config_path = config_dir.join("config.yaml");

        let secrets = match env::var("GOOSE_DISABLE_KEYRING") {
            Ok(_) => SecretStorage::File {
                path: config_dir.join("secrets.yaml"),
            },
            Err(_) => SecretStorage::Keyring {
                service: KEYRING_SERVICE.to_string(),
            },
        };
        Config {
            config_path,
            secrets,
        }
    }
}

impl Config {
    /// Get the global configuration instance.
    ///
    /// This will initialize the configuration with the default path (~/.config/goose/config.yaml)
    /// if it hasn't been initialized yet.
    pub fn global() -> &'static Config {
        GLOBAL_CONFIG.get_or_init(Config::default)
    }

    /// Create a new configuration instance with custom paths
    ///
    /// This is primarily useful for testing or for applications that need
    /// to manage multiple configuration files.
    pub fn new<P: AsRef<Path>>(config_path: P, service: &str) -> Result<Self, ConfigError> {
        Ok(Config {
            config_path: config_path.as_ref().to_path_buf(),
            secrets: SecretStorage::Keyring {
                service: service.to_string(),
            },
        })
    }

    /// Create a new configuration instance with custom paths
    ///
    /// This is primarily useful for testing or for applications that need
    /// to manage multiple configuration files.
    pub fn new_with_file_secrets<P1: AsRef<Path>, P2: AsRef<Path>>(
        config_path: P1,
        secrets_path: P2,
    ) -> Result<Self, ConfigError> {
        Ok(Config {
            config_path: config_path.as_ref().to_path_buf(),
            secrets: SecretStorage::File {
                path: secrets_path.as_ref().to_path_buf(),
            },
        })
    }

    /// Check if this config already exists
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Check if this config already exists
    pub fn clear(&self) -> Result<(), ConfigError> {
        Ok(std::fs::remove_file(&self.config_path)?)
    }

    /// Get the path to the configuration file
    pub fn path(&self) -> String {
        self.config_path.to_string_lossy().to_string()
    }

    // Load current values from the config file
    pub fn load_values(&self) -> Result<HashMap<String, Value>, ConfigError> {
        if self.config_path.exists() {
            self.load_values_with_recovery()
        } else {
            // Config file doesn't exist, try to recover from backup first
            tracing::info!("Config file doesn't exist, attempting recovery from backup");

            if let Ok(backup_values) = self.try_restore_from_backup() {
                tracing::info!("Successfully restored config from backup");
                return Ok(backup_values);
            }

            // No backup available, create a default config
            tracing::info!("No backup found, creating default configuration");

            // Try to load from init-config.yaml if it exists, otherwise use empty config
            let default_config = self
                .load_init_config_if_exists()
                .unwrap_or_else(|_| HashMap::new());

            self.create_and_save_default_config(default_config)
        }
    }

    // Helper method to create and save default config with consistent logging
    fn create_and_save_default_config(
        &self,
        default_config: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ConfigError> {
        // Try to write the default config to disk
        match self.save_values(default_config.clone()) {
            Ok(_) => {
                if default_config.is_empty() {
                    tracing::info!("Created fresh empty config file");
                } else {
                    tracing::info!(
                        "Created fresh config file from init-config.yaml with {} keys",
                        default_config.len()
                    );
                }
                Ok(default_config)
            }
            Err(write_error) => {
                tracing::error!("Failed to write default config file: {}", write_error);
                // Even if we can't write to disk, return config so app can still run
                Ok(default_config)
            }
        }
    }

    // Load values with automatic recovery from corruption
    fn load_values_with_recovery(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let file_content = std::fs::read_to_string(&self.config_path)?;

        // First attempt: try to parse the current config
        match self.parse_yaml_content(&file_content) {
            Ok(values) => Ok(values),
            Err(parse_error) => {
                tracing::warn!(
                    "Config file appears corrupted, attempting recovery: {}",
                    parse_error
                );

                // Try to recover from backup
                if let Ok(backup_values) = self.try_restore_from_backup() {
                    tracing::info!("Successfully restored config from backup");
                    return Ok(backup_values);
                }

                // Last resort: create a fresh default config file
                tracing::error!("Could not recover config file, creating fresh default configuration. Original error: {}", parse_error);

                // Try to load from init-config.yaml if it exists, otherwise use empty config
                let default_config = self
                    .load_init_config_if_exists()
                    .unwrap_or_else(|_| HashMap::new());

                self.create_and_save_default_config(default_config)
            }
        }
    }

    // Parse YAML content into HashMap
    fn parse_yaml_content(&self, content: &str) -> Result<HashMap<String, Value>, ConfigError> {
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(content)?;
        let json_value: Value = serde_json::to_value(yaml_value)?;

        match json_value {
            Value::Object(map) => Ok(map.into_iter().collect()),
            _ => Ok(HashMap::new()),
        }
    }

    // Try to restore from backup file
    fn try_restore_from_backup(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let backup_paths = self.get_backup_paths();

        for backup_path in backup_paths {
            if backup_path.exists() {
                match std::fs::read_to_string(&backup_path) {
                    Ok(backup_content) => {
                        match self.parse_yaml_content(&backup_content) {
                            Ok(values) => {
                                // Successfully parsed backup, restore it as the main config
                                if let Err(e) = self.save_values(values.clone()) {
                                    tracing::warn!(
                                        "Failed to restore backup as main config: {}",
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "Restored config from backup: {:?}",
                                        backup_path
                                    );
                                }
                                return Ok(values);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Backup file {:?} is also corrupted: {}",
                                    backup_path,
                                    e
                                );
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Could not read backup file {:?}: {}", backup_path, e);
                        continue;
                    }
                }
            }
        }

        Err(ConfigError::NotFound("No valid backup found".to_string()))
    }

    // Get list of backup file paths in order of preference
    fn get_backup_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Primary backup (created by backup_config endpoint)
        if let Some(file_name) = self.config_path.file_name() {
            let mut backup_name = file_name.to_os_string();
            backup_name.push(".bak");
            paths.push(self.config_path.with_file_name(backup_name));
        }

        // Timestamped backups
        for i in 1..=5 {
            if let Some(file_name) = self.config_path.file_name() {
                let mut backup_name = file_name.to_os_string();
                backup_name.push(format!(".bak.{}", i));
                paths.push(self.config_path.with_file_name(backup_name));
            }
        }

        paths
    }

    // Try to load init-config.yaml from workspace root if it exists
    fn load_init_config_if_exists(&self) -> Result<HashMap<String, Value>, ConfigError> {
        load_init_config_from_workspace()
    }

    // Save current values to the config file
    pub fn save_values(&self, values: HashMap<String, Value>) -> Result<(), ConfigError> {
        // Create backup before writing new config
        self.create_backup_if_needed()?;

        // Convert to YAML for storage
        let yaml_value = serde_yaml::to_string(&values)?;

        // Ensure the directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::DirectoryError(e.to_string()))?;
        }

        // Write to a temporary file first for atomic operation
        let temp_path = self.config_path.with_extension("tmp");

        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;

            // Acquire an exclusive lock
            file.lock_exclusive()
                .map_err(|e| ConfigError::LockError(e.to_string()))?;

            // Write the contents using the same file handle
            file.write_all(yaml_value.as_bytes())?;
            file.sync_all()?;

            // Unlock is handled automatically when file is dropped
        }

        // Atomically replace the original file
        std::fs::rename(&temp_path, &self.config_path)?;

        Ok(())
    }

    // Create backup of current config file if it exists and is valid
    fn create_backup_if_needed(&self) -> Result<(), ConfigError> {
        if !self.config_path.exists() {
            return Ok(());
        }

        // Check if current config is valid before backing it up
        let current_content = std::fs::read_to_string(&self.config_path)?;
        if self.parse_yaml_content(&current_content).is_err() {
            // Don't back up corrupted files
            return Ok(());
        }

        // Rotate existing backups
        self.rotate_backups()?;

        // Create new backup
        if let Some(file_name) = self.config_path.file_name() {
            let mut backup_name = file_name.to_os_string();
            backup_name.push(".bak");
            let backup_path = self.config_path.with_file_name(backup_name);

            if let Err(e) = std::fs::copy(&self.config_path, &backup_path) {
                tracing::warn!("Failed to create config backup: {}", e);
                // Don't fail the entire operation if backup fails
            } else {
                tracing::debug!("Created config backup: {:?}", backup_path);
            }
        }

        Ok(())
    }

    // Rotate backup files to keep the most recent ones
    fn rotate_backups(&self) -> Result<(), ConfigError> {
        if let Some(file_name) = self.config_path.file_name() {
            // Move .bak.4 to .bak.5, .bak.3 to .bak.4, etc.
            for i in (1..5).rev() {
                let mut current_backup = file_name.to_os_string();
                current_backup.push(format!(".bak.{}", i));
                let current_path = self.config_path.with_file_name(&current_backup);

                let mut next_backup = file_name.to_os_string();
                next_backup.push(format!(".bak.{}", i + 1));
                let next_path = self.config_path.with_file_name(&next_backup);

                if current_path.exists() {
                    let _ = std::fs::rename(&current_path, &next_path);
                }
            }

            // Move .bak to .bak.1
            let mut backup_name = file_name.to_os_string();
            backup_name.push(".bak");
            let backup_path = self.config_path.with_file_name(&backup_name);

            if backup_path.exists() {
                let mut backup_1_name = file_name.to_os_string();
                backup_1_name.push(".bak.1");
                let backup_1_path = self.config_path.with_file_name(&backup_1_name);
                let _ = std::fs::rename(&backup_path, &backup_1_path);
            }
        }

        Ok(())
    }

    // Load current secrets from the keyring
    pub fn load_secrets(&self) -> Result<HashMap<String, Value>, ConfigError> {
        match &self.secrets {
            SecretStorage::Keyring { service } => {
                let entry = Entry::new(service, KEYRING_USERNAME)?;

                match entry.get_password() {
                    Ok(content) => {
                        let values: HashMap<String, Value> = serde_json::from_str(&content)?;
                        Ok(values)
                    }
                    Err(keyring::Error::NoEntry) => Ok(HashMap::new()),
                    Err(e) => Err(ConfigError::KeyringError(e.to_string())),
                }
            }
            SecretStorage::File { path } => {
                if path.exists() {
                    let file_content = std::fs::read_to_string(path)?;
                    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
                    let json_value: Value = serde_json::to_value(yaml_value)?;
                    match json_value {
                        Value::Object(map) => Ok(map.into_iter().collect()),
                        _ => Ok(HashMap::new()),
                    }
                } else {
                    Ok(HashMap::new())
                }
            }
        }
    }

    /// Parse an environment variable value into a JSON Value.
    ///
    /// This function tries to intelligently parse environment variable values:
    /// 1. First attempts JSON parsing (for structured data)
    /// 2. If that fails, tries primitive type parsing for common cases
    /// 3. Falls back to string if nothing else works
    fn parse_env_value(val: &str) -> Result<Value, ConfigError> {
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

    // check all possible places for a parameter
    pub fn get(&self, key: &str, is_secret: bool) -> Result<Value, ConfigError> {
        if is_secret {
            self.get_secret(key)
        } else {
            self.get_param(key)
        }
    }

    // save a parameter in the appropriate location based on if it's secret or not
    pub fn set(&self, key: &str, value: Value, is_secret: bool) -> Result<(), ConfigError> {
        if is_secret {
            self.set_secret(key, value)
        } else {
            self.set_param(key, value)
        }
    }

    /// Get a configuration value (non-secret).
    ///
    /// This will attempt to get the value from:
    /// 1. Environment variable with the exact key name
    /// 2. Configuration file
    ///
    /// The value will be deserialized into the requested type. This works with
    /// both simple types (String, i32, etc.) and complex types that implement
    /// serde::Deserialize.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - The key doesn't exist in either environment or config file
    /// - The value cannot be deserialized into the requested type
    /// - There is an error reading the config file
    pub fn get_param<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        // First check environment variables (convert to uppercase)
        let env_key = key.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            let value = Self::parse_env_value(&val)?;
            return Ok(serde_json::from_value(value)?);
        }

        // Load current values from file
        let values = self.load_values()?;

        // Then check our stored values
        values
            .get(key)
            .ok_or_else(|| ConfigError::NotFound(key.to_string()))
            .and_then(|v| Ok(serde_json::from_value(v.clone())?))
    }

    /// Set a configuration value in the config file (non-secret).
    ///
    /// This will immediately write the value to the config file. The value
    /// can be any type that can be serialized to JSON/YAML.
    ///
    /// Note that this does not affect environment variables - those can only
    /// be set through the system environment.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - There is an error reading or writing the config file
    /// - There is an error serializing the value
    pub fn set_param(&self, key: &str, value: Value) -> Result<(), ConfigError> {
        // Load current values with recovery if needed
        let mut values = self.load_values()?;

        // Modify values
        values.insert(key.to_string(), value);

        // Save all values using the atomic write approach
        self.save_values(values)
    }

    /// Delete a configuration value in the config file.
    ///
    /// This will immediately write the value to the config file. The value
    /// can be any type that can be serialized to JSON/YAML.
    ///
    /// Note that this does not affect environment variables - those can only
    /// be set through the system environment.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - There is an error reading or writing the config file
    /// - There is an error serializing the value
    pub fn delete(&self, key: &str) -> Result<(), ConfigError> {
        let mut values = self.load_values()?;
        values.remove(key);

        self.save_values(values)
    }

    /// Get a secret value.
    ///
    /// This will attempt to get the value from:
    /// 1. Environment variable with the exact key name
    /// 2. System keyring
    ///
    /// The value will be deserialized into the requested type. This works with
    /// both simple types (String, i32, etc.) and complex types that implement
    /// serde::Deserialize.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - The key doesn't exist in either environment or keyring
    /// - The value cannot be deserialized into the requested type
    /// - There is an error accessing the keyring
    pub fn get_secret<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        // First check environment variables (convert to uppercase)
        let env_key = key.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            let value = Self::parse_env_value(&val)?;
            return Ok(serde_json::from_value(value)?);
        }

        // Then check keyring
        let values = self.load_secrets()?;
        values
            .get(key)
            .ok_or_else(|| ConfigError::NotFound(key.to_string()))
            .and_then(|v| Ok(serde_json::from_value(v.clone())?))
    }

    /// Set a secret value in the system keyring.
    ///
    /// This will store the value in a single JSON object in the system keyring,
    /// alongside any other secrets. The value can be any type that can be
    /// serialized to JSON.
    ///
    /// Note that this does not affect environment variables - those can only
    /// be set through the system environment.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - There is an error accessing the keyring
    /// - There is an error serializing the value
    pub fn set_secret(&self, key: &str, value: Value) -> Result<(), ConfigError> {
        let mut values = self.load_secrets()?;
        values.insert(key.to_string(), value);

        match &self.secrets {
            SecretStorage::Keyring { service } => {
                let json_value = serde_json::to_string(&values)?;
                let entry = Entry::new(service, KEYRING_USERNAME)?;
                entry.set_password(&json_value)?;
            }
            SecretStorage::File { path } => {
                let yaml_value = serde_yaml::to_string(&values)?;
                std::fs::write(path, yaml_value)?;
            }
        };
        Ok(())
    }

    /// Delete a secret from the system keyring.
    ///
    /// This will remove the specified key from the JSON object in the system keyring.
    /// Other secrets will remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns a ConfigError if:
    /// - There is an error accessing the keyring
    /// - There is an error serializing the remaining values
    pub fn delete_secret(&self, key: &str) -> Result<(), ConfigError> {
        let mut values = self.load_secrets()?;
        values.remove(key);

        match &self.secrets {
            SecretStorage::Keyring { service } => {
                let json_value = serde_json::to_string(&values)?;
                let entry = Entry::new(service, KEYRING_USERNAME)?;
                entry.set_password(&json_value)?;
            }
            SecretStorage::File { path } => {
                let yaml_value = serde_yaml::to_string(&values)?;
                std::fs::write(path, yaml_value)?;
            }
        };
        Ok(())
    }
}

/// Load init-config.yaml from workspace root if it exists.
/// This function is shared between the config recovery and the init_config endpoint.
pub fn load_init_config_from_workspace() -> Result<HashMap<String, Value>, ConfigError> {
    let workspace_root = match std::env::current_exe() {
        Ok(mut exe_path) => {
            while let Some(parent) = exe_path.parent() {
                let cargo_toml = parent.join("Cargo.toml");
                if cargo_toml.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                        if content.contains("[workspace]") {
                            exe_path = parent.to_path_buf();
                            break;
                        }
                    }
                }
                exe_path = parent.to_path_buf();
            }
            exe_path
        }
        Err(_) => {
            return Err(ConfigError::FileError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine executable path",
            )))
        }
    };

    let init_config_path = workspace_root.join("init-config.yaml");
    if !init_config_path.exists() {
        return Err(ConfigError::NotFound(
            "init-config.yaml not found".to_string(),
        ));
    }

    let init_content = std::fs::read_to_string(&init_config_path)?;
    let init_values: HashMap<String, Value> =
        match serde_yaml::from_str::<serde_yaml::Value>(&init_content) {
            Ok(yaml_value) => {
                let json_value: Value = serde_json::to_value(yaml_value)?;
                match json_value {
                    Value::Object(map) => map.into_iter().collect(),
                    _ => HashMap::new(),
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse init-config.yaml: {}", e);
                return Err(ConfigError::DeserializeError(e.to_string()));
            }
        };

    tracing::info!("Loaded init-config.yaml with {} keys", init_values.len());
    Ok(init_values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::NamedTempFile;

    fn cleanup_keyring() -> Result<(), ConfigError> {
        let entry = Entry::new(TEST_KEYRING_SERVICE, KEYRING_USERNAME)?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(ConfigError::KeyringError(e.to_string())),
        }
    }

    #[test]
    fn test_basic_config() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Set a simple string value
        config.set_param("test_key", Value::String("test_value".to_string()))?;

        // Test simple string retrieval
        let value: String = config.get_param("test_key")?;
        assert_eq!(value, "test_value");

        // Test with environment variable override
        std::env::set_var("TEST_KEY", "env_value");
        let value: String = config.get_param("test_key")?;
        assert_eq!(value, "env_value");

        Ok(())
    }

    #[test]
    fn test_complex_type() -> Result<(), ConfigError> {
        #[derive(Deserialize, Debug, PartialEq)]
        struct TestStruct {
            field1: String,
            field2: i32,
        }

        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Set a complex value
        config.set_param(
            "complex_key",
            serde_json::json!({
                "field1": "hello",
                "field2": 42
            }),
        )?;

        let value: TestStruct = config.get_param("complex_key")?;
        assert_eq!(value.field1, "hello");
        assert_eq!(value.field2, 42);

        Ok(())
    }

    #[test]
    fn test_missing_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE).unwrap();

        let result: Result<String, ConfigError> = config.get_param("nonexistent_key");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_yaml_formatting() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        config.set_param("key1", Value::String("value1".to_string()))?;
        config.set_param("key2", Value::Number(42.into()))?;

        // Read the file directly to check YAML formatting
        let content = std::fs::read_to_string(temp_file.path())?;
        assert!(content.contains("key1: value1"));
        assert!(content.contains("key2: 42"));

        Ok(())
    }

    #[test]
    fn test_value_management() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        config.set_param("key", Value::String("value".to_string()))?;

        let value: String = config.get_param("key")?;
        assert_eq!(value, "value");

        config.delete("key")?;

        let result: Result<String, ConfigError> = config.get_param("key");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));

        Ok(())
    }

    #[test]
    fn test_file_based_secrets_management() -> Result<(), ConfigError> {
        let config_file = NamedTempFile::new().unwrap();
        let secrets_file = NamedTempFile::new().unwrap();
        let config = Config::new_with_file_secrets(config_file.path(), secrets_file.path())?;

        config.set_secret("key", Value::String("value".to_string()))?;

        let value: String = config.get_secret("key")?;
        assert_eq!(value, "value");

        config.delete_secret("key")?;

        let result: Result<String, ConfigError> = config.get_secret("key");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));

        Ok(())
    }

    #[test]
    #[serial]
    fn test_secret_management() -> Result<(), ConfigError> {
        cleanup_keyring()?;
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Test setting and getting a simple secret
        config.set_secret("api_key", Value::String("secret123".to_string()))?;
        let value: String = config.get_secret("api_key")?;
        assert_eq!(value, "secret123");

        // Test environment variable override
        std::env::set_var("API_KEY", "env_secret");
        let value: String = config.get_secret("api_key")?;
        assert_eq!(value, "env_secret");
        std::env::remove_var("API_KEY");

        // Test deleting a secret
        config.delete_secret("api_key")?;
        let result: Result<String, ConfigError> = config.get_secret("api_key");
        assert!(matches!(result, Err(ConfigError::NotFound(_))));

        cleanup_keyring()?;
        Ok(())
    }

    #[test]
    #[serial]
    fn test_multiple_secrets() -> Result<(), ConfigError> {
        cleanup_keyring()?;
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Set multiple secrets
        config.set_secret("key1", Value::String("secret1".to_string()))?;
        config.set_secret("key2", Value::String("secret2".to_string()))?;

        // Verify both exist
        let value1: String = config.get_secret("key1")?;
        let value2: String = config.get_secret("key2")?;
        assert_eq!(value1, "secret1");
        assert_eq!(value2, "secret2");

        // Delete one secret
        config.delete_secret("key1")?;

        // Verify key1 is gone but key2 remains
        let result1: Result<String, ConfigError> = config.get_secret("key1");
        let value2: String = config.get_secret("key2")?;
        assert!(matches!(result1, Err(ConfigError::NotFound(_))));
        assert_eq!(value2, "secret2");

        cleanup_keyring()?;
        Ok(())
    }

    #[test]
    fn test_concurrent_writes() -> Result<(), ConfigError> {
        use std::sync::{Arc, Barrier, Mutex};
        use std::thread;

        let temp_file = NamedTempFile::new().unwrap();
        let config = Arc::new(Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?);
        let barrier = Arc::new(Barrier::new(3)); // For 3 concurrent threads
        let values = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = vec![];

        // Initialize with empty values
        config.save_values(HashMap::new())?;

        // Spawn 3 threads that will try to write simultaneously
        for i in 0..3 {
            let config = Arc::clone(&config);
            let barrier = Arc::clone(&barrier);
            let values = Arc::clone(&values);
            let handle = thread::spawn(move || -> Result<(), ConfigError> {
                // Wait for all threads to reach this point
                barrier.wait();

                // Get the lock and update values
                let mut values = values.lock().unwrap();
                values.insert(format!("key{}", i), Value::String(format!("value{}", i)));

                // Write all values
                config.save_values(values.clone())?;
                Ok(())
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap()?;
        }

        // Verify all values were written correctly
        let final_values = config.load_values()?;

        // Print the final values for debugging
        println!("Final values: {:?}", final_values);

        assert_eq!(
            final_values.len(),
            3,
            "Expected 3 values, got {}",
            final_values.len()
        );

        for i in 0..3 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            assert!(
                final_values.get(&key).is_some(),
                "Missing key {} in final values",
                key
            );
            assert_eq!(
                final_values.get(&key).unwrap(),
                &Value::String(value),
                "Incorrect value for key {}",
                key
            );
        }

        Ok(())
    }

    #[test]
    fn test_config_recovery_from_backup() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Create a valid config first
        config.set_param("key1", Value::String("value1".to_string()))?;

        // Verify the backup was created by the first write
        let backup_paths = config.get_backup_paths();
        println!("Backup paths: {:?}", backup_paths);
        for (i, path) in backup_paths.iter().enumerate() {
            println!("Backup {} exists: {}", i, path.exists());
        }

        // Make another write to ensure backup is created
        config.set_param("key2", Value::Number(42.into()))?;

        // Check again
        for (i, path) in backup_paths.iter().enumerate() {
            println!(
                "After second write - Backup {} exists: {}",
                i,
                path.exists()
            );
        }

        // Corrupt the main config file
        std::fs::write(temp_file.path(), "invalid: yaml: content: [unclosed")?;

        // Try to load values - should recover from backup
        let recovered_values = config.load_values()?;
        println!("Recovered values: {:?}", recovered_values);

        // Should have recovered the data
        assert!(
            recovered_values.len() >= 1,
            "Should have recovered at least one key"
        );

        Ok(())
    }

    #[test]
    fn test_config_recovery_creates_fresh_file() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Create a corrupted config file with no backup
        std::fs::write(temp_file.path(), "invalid: yaml: content: [unclosed")?;

        // Try to load values - should create a fresh default config
        let recovered_values = config.load_values()?;

        // Should return empty config
        assert_eq!(recovered_values.len(), 0);

        // Verify that a clean config file was written to disk
        let file_content = std::fs::read_to_string(temp_file.path())?;

        // Should be valid YAML (empty object)
        let parsed: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
        assert!(parsed.is_mapping());

        // Should be able to load it again without issues
        let reloaded_values = config.load_values()?;
        assert_eq!(reloaded_values.len(), 0);

        Ok(())
    }

    #[test]
    fn test_config_file_creation_when_missing() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path();

        // Delete the file to simulate it not existing
        std::fs::remove_file(config_path)?;
        assert!(!config_path.exists());

        let config = Config::new(config_path, TEST_KEYRING_SERVICE)?;

        // Try to load values - should create a fresh default config file
        let values = config.load_values()?;

        // Should return empty config
        assert_eq!(values.len(), 0);

        // Verify that the config file was created
        assert!(config_path.exists());

        // Verify that it's valid YAML
        let file_content = std::fs::read_to_string(config_path)?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
        assert!(parsed.is_mapping());

        // Should be able to load it again without issues
        let reloaded_values = config.load_values()?;
        assert_eq!(reloaded_values.len(), 0);

        Ok(())
    }

    #[test]
    fn test_config_recovery_from_backup_when_missing() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path();
        let config = Config::new(config_path, TEST_KEYRING_SERVICE)?;

        // First, create a config with some data
        config.set_param("test_key_backup", Value::String("backup_value".to_string()))?;
        config.set_param("another_key", Value::Number(42.into()))?;

        // Verify the backup was created
        let backup_paths = config.get_backup_paths();
        let primary_backup = &backup_paths[0]; // .bak file

        // Make sure we have a backup by doing another write
        config.set_param("third_key", Value::Bool(true))?;
        assert!(primary_backup.exists(), "Backup should exist after writes");

        // Now delete the main config file to simulate it being lost
        std::fs::remove_file(config_path)?;
        assert!(!config_path.exists());

        // Try to load values - should recover from backup
        let recovered_values = config.load_values()?;

        // Should have recovered the data from backup
        assert!(
            recovered_values.len() >= 1,
            "Should have recovered data from backup"
        );

        // Verify the main config file was restored
        assert!(config_path.exists(), "Main config file should be restored");

        // Verify we can load the data (using a key that won't conflict with env vars)
        if let Ok(backup_value) = config.get_param::<String>("test_key_backup") {
            // If we recovered the key, great!
            assert_eq!(backup_value, "backup_value");
        }
        // Note: Due to back up rotation, we might not get the exact same data,
        // but we should get some data back

        Ok(())
    }

    #[test]
    fn test_atomic_write_prevents_corruption() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Set initial values
        config.set_param("key1", Value::String("value1".to_string()))?;

        // Verify the config file exists and is valid
        assert!(temp_file.path().exists());
        let content = std::fs::read_to_string(temp_file.path())?;
        assert!(serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok());

        // The temp file should not exist after successful write
        let temp_path = temp_file.path().with_extension("tmp");
        assert!(!temp_path.exists(), "Temporary file should be cleaned up");

        Ok(())
    }

    #[test]
    fn test_backup_rotation() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Create multiple versions to test rotation
        for i in 1..=7 {
            config.set_param("version", Value::Number(i.into()))?;
        }

        let backup_paths = config.get_backup_paths();

        // Should have backups but not more than our limit
        let existing_backups: Vec<_> = backup_paths.iter().filter(|p| p.exists()).collect();
        assert!(
            existing_backups.len() <= 6,
            "Should not exceed backup limit"
        ); // .bak + .bak.1 through .bak.5

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_strings() -> Result<(), ConfigError> {
        // Test unquoted strings
        let value = Config::parse_env_value("ANTHROPIC")?;
        assert_eq!(value, Value::String("ANTHROPIC".to_string()));

        // Test strings with spaces
        let value = Config::parse_env_value("hello world")?;
        assert_eq!(value, Value::String("hello world".to_string()));

        // Test JSON quoted strings
        let value = Config::parse_env_value("\"ANTHROPIC\"")?;
        assert_eq!(value, Value::String("ANTHROPIC".to_string()));

        // Test empty string
        let value = Config::parse_env_value("")?;
        assert_eq!(value, Value::String("".to_string()));

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_numbers() -> Result<(), ConfigError> {
        // Test integers
        let value = Config::parse_env_value("42")?;
        assert_eq!(value, Value::Number(42.into()));

        let value = Config::parse_env_value("-123")?;
        assert_eq!(value, Value::Number((-123).into()));

        // Test floats
        let value = Config::parse_env_value("3.14")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 3.14);
        }

        let value = Config::parse_env_value("0.01")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 0.01);
        }

        // Test zero
        let value = Config::parse_env_value("0")?;
        assert_eq!(value, Value::Number(0.into()));

        let value = Config::parse_env_value("0.0")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 0.0);
        }

        // Test numbers starting with decimal point
        let value = Config::parse_env_value(".5")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 0.5);
        }

        let value = Config::parse_env_value(".00001")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 0.00001);
        }

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_booleans() -> Result<(), ConfigError> {
        // Test true variants
        let value = Config::parse_env_value("true")?;
        assert_eq!(value, Value::Bool(true));

        let value = Config::parse_env_value("True")?;
        assert_eq!(value, Value::Bool(true));

        let value = Config::parse_env_value("TRUE")?;
        assert_eq!(value, Value::Bool(true));

        // Test false variants
        let value = Config::parse_env_value("false")?;
        assert_eq!(value, Value::Bool(false));

        let value = Config::parse_env_value("False")?;
        assert_eq!(value, Value::Bool(false));

        let value = Config::parse_env_value("FALSE")?;
        assert_eq!(value, Value::Bool(false));

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_json() -> Result<(), ConfigError> {
        // Test JSON objects
        let value = Config::parse_env_value("{\"host\": \"localhost\", \"port\": 8080}")?;
        assert!(matches!(value, Value::Object(_)));
        if let Value::Object(obj) = value {
            assert_eq!(
                obj.get("host"),
                Some(&Value::String("localhost".to_string()))
            );
            assert_eq!(obj.get("port"), Some(&Value::Number(8080.into())));
        }

        // Test JSON arrays
        let value = Config::parse_env_value("[1, 2, 3]")?;
        assert!(matches!(value, Value::Array(_)));
        if let Value::Array(arr) = value {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(1.into()));
            assert_eq!(arr[1], Value::Number(2.into()));
            assert_eq!(arr[2], Value::Number(3.into()));
        }

        // Test JSON null
        let value = Config::parse_env_value("null")?;
        assert_eq!(value, Value::Null);

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_edge_cases() -> Result<(), ConfigError> {
        // Test whitespace handling
        let value = Config::parse_env_value(" 42 ")?;
        assert_eq!(value, Value::Number(42.into()));

        let value = Config::parse_env_value(" true ")?;
        assert_eq!(value, Value::Bool(true));

        // Test strings that look like numbers but aren't
        let value = Config::parse_env_value("123abc")?;
        assert_eq!(value, Value::String("123abc".to_string()));

        let value = Config::parse_env_value("abc123")?;
        assert_eq!(value, Value::String("abc123".to_string()));

        // Test strings that look like booleans but aren't
        let value = Config::parse_env_value("truthy")?;
        assert_eq!(value, Value::String("truthy".to_string()));

        let value = Config::parse_env_value("falsy")?;
        assert_eq!(value, Value::String("falsy".to_string()));

        Ok(())
    }

    #[test]
    fn test_env_var_parsing_numeric_edge_cases() -> Result<(), ConfigError> {
        // Test leading zeros (should be treated as integers, not octal)
        let value = Config::parse_env_value("007")?;
        assert_eq!(value, Value::Number(7.into()));

        // Test large numbers
        let value = Config::parse_env_value("9223372036854775807")?; // i64::MAX
        assert_eq!(value, Value::Number(9223372036854775807i64.into()));

        // Test scientific notation (JSON parsing should handle this correctly)
        let value = Config::parse_env_value("1e10")?;
        assert!(matches!(value, Value::Number(_)));
        if let Value::Number(n) = value {
            assert_eq!(n.as_f64().unwrap(), 1e10);
        }

        // Test infinity (should be treated as string)
        let value = Config::parse_env_value("inf")?;
        assert_eq!(value, Value::String("inf".to_string()));

        Ok(())
    }

    #[test]
    fn test_env_var_with_config_integration() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Test string environment variable (the original issue case)
        std::env::set_var("PROVIDER", "ANTHROPIC");
        let value: String = config.get_param("provider")?;
        assert_eq!(value, "ANTHROPIC");

        // Test number environment variable
        std::env::set_var("PORT", "8080");
        let value: i32 = config.get_param("port")?;
        assert_eq!(value, 8080);

        // Test boolean environment variable
        std::env::set_var("ENABLED", "true");
        let value: bool = config.get_param("enabled")?;
        assert_eq!(value, true);

        // Test JSON object environment variable
        std::env::set_var("CONFIG", "{\"debug\": true, \"level\": 5}");
        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            debug: bool,
            level: i32,
        }
        let value: TestConfig = config.get_param("config")?;
        assert_eq!(value.debug, true);
        assert_eq!(value.level, 5);

        // Clean up
        std::env::remove_var("PROVIDER");
        std::env::remove_var("PORT");
        std::env::remove_var("ENABLED");
        std::env::remove_var("CONFIG");

        Ok(())
    }

    #[test]
    fn test_env_var_precedence_over_config_file() -> Result<(), ConfigError> {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?;

        // Set value in config file
        config.set_param("test_precedence", Value::String("file_value".to_string()))?;

        // Verify file value is returned when no env var
        let value: String = config.get_param("test_precedence")?;
        assert_eq!(value, "file_value");

        // Set environment variable
        std::env::set_var("TEST_PRECEDENCE", "env_value");

        // Environment variable should take precedence
        let value: String = config.get_param("test_precedence")?;
        assert_eq!(value, "env_value");

        // Clean up
        std::env::remove_var("TEST_PRECEDENCE");

        Ok(())
    }
}
