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
            let file_content = std::fs::read_to_string(&self.config_path)?;
            // Parse YAML into JSON Value for consistent internal representation
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

    // Save current values to the config file
    pub fn save_values(&self, values: HashMap<String, Value>) -> Result<(), ConfigError> {
        // Convert to YAML for storage
        let yaml_value = serde_yaml::to_string(&values)?;

        // Ensure the directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::DirectoryError(e.to_string()))?;
        }

        // Open the file with write permissions, create if it doesn't exist
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.config_path)?;

        // Acquire an exclusive lock
        file.lock_exclusive()
            .map_err(|e| ConfigError::LockError(e.to_string()))?;

        // Write the contents using the same file handle
        file.write_all(yaml_value.as_bytes())?;
        file.sync_all()?;

        // Unlock is handled automatically when file is dropped
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
            // Parse the environment variable value into a serde_json::Value
            let value: Value = serde_json::from_str(&val).unwrap_or(Value::String(val));
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
        let mut values = self.load_values()?;
        values.insert(key.to_string(), value);

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
            let value: Value = serde_json::from_str(&val).unwrap_or(Value::String(val));
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
    fn test_concurrent_extension_writes() -> Result<(), ConfigError> {
        use std::sync::{Arc, Barrier};
        use std::thread;
        use std::time::Duration;

        let temp_file = NamedTempFile::new().unwrap();
        let config = Arc::new(Config::new(temp_file.path(), TEST_KEYRING_SERVICE)?);
        let barrier = Arc::new(Barrier::new(3)); // For 3 concurrent threads
        let mut handles = vec![];

        // Initialize with empty values
        config.save_values(HashMap::new())?;

        // Spawn 3 threads that will try to write extension configs simultaneously
        for i in 0..3 {
            let config = Arc::clone(&config);
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || -> Result<(), ConfigError> {
                // Wait for all threads to reach this point
                barrier.wait();

                // Add a small random delay to increase chance of concurrent access
                thread::sleep(Duration::from_millis(i * 10));

                let extension_key = format!("extension_{}", i);
                let mut values = config.load_values()?;

                values.insert(
                    extension_key.clone(),
                    serde_json::json!({
                        "name": format!("test_extension_{}", i),
                        "version": format!("1.0.{}", i),
                        "enabled": true,
                        "settings": {
                            "option1": format!("value{}", i),
                            "option2": i
                        }
                    }),
                );

                // Write all values atomically
                config.save_values(values)?;
                Ok(())
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap()?;
        }

        // Verify all extension configs were written correctly
        let final_values = config.load_values()?;

        // Print the final values for debugging
        println!("Final values: {:?}", final_values);

        assert_eq!(
            final_values.len(),
            3,
            "Expected 3 extension configs, got {}",
            final_values.len()
        );

        for i in 0..3 {
            let extension_key = format!("extension_{}", i);

            let config = final_values
                .get(&extension_key)
                .expect(&format!("Missing extension config for {}", extension_key));

            // Verify the structure matches what we wrote
            let config_obj = config.as_object().unwrap();
            assert_eq!(
                config_obj.get("name").unwrap().as_str().unwrap(),
                format!("test_extension_{}", i)
            );
            assert_eq!(
                config_obj.get("version").unwrap().as_str().unwrap(),
                format!("1.0.{}", i)
            );
            assert!(config_obj.get("enabled").unwrap().as_bool().unwrap());

            let settings = config_obj.get("settings").unwrap().as_object().unwrap();
            assert_eq!(
                settings.get("option1").unwrap().as_str().unwrap(),
                format!("value{}", i)
            );
            assert_eq!(settings.get("option2").unwrap().as_i64().unwrap() as i32, i);
        }

        Ok(())
    }
}
