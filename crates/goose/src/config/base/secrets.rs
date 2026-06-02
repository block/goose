use super::*;
use crate::config::paths::Paths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

impl Config {
    pub fn all_secrets(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut cache = self.secrets_cache.lock().unwrap();

        let values = if let Some(ref cached_secrets) = *cache {
            cached_secrets.clone()
        } else {
            tracing::debug!("secrets cache miss, fetching from storage");

            let loaded = match &self.secrets {
                #[cfg(feature = "system-keyring")]
                SecretStorage::Keyring { service } => {
                    let result =
                        self.handle_keyring_operation(|entry| entry.get_password(), service, None);

                    match result {
                        Ok(content) => {
                            let values: HashMap<String, Value> = serde_json::from_str(&content)?;
                            values
                        }
                        Err(ConfigError::FallbackToFileStorage) => {
                            self.fallback_to_file_storage()?
                        }
                        Err(ConfigError::KeyringError(msg))
                            if msg.contains("No entry found")
                                || msg.contains("No matching entry found") =>
                        {
                            self.fallback_to_file_storage()?
                        }
                        Err(e) => return Err(e),
                    }
                }
                SecretStorage::File { path } => self.read_secrets_from_file(path)?,
            };

            *cache = Some(loaded.clone());
            loaded
        };

        Ok(values)
    }

    pub(super) fn parse_env_value(val: &str) -> Result<Value, ConfigError> {
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

    pub fn get_secret<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        let env_key = key.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            let value = Self::parse_env_value(&val)?;
            return Ok(serde_json::from_value(value)?);
        }

        let values = self.all_secrets()?;
        values
            .get(key)
            .ok_or_else(|| ConfigError::NotFound(key.to_string()))
            .and_then(|v| Ok(serde_json::from_value(v.clone())?))
    }

    pub fn get_secrets(
        &self,
        primary: &str,
        maybe_secret: &[&str],
    ) -> Result<HashMap<String, String>, ConfigError> {
        let use_env = env::var(primary.to_uppercase()).is_ok();
        let get_value = |key: &str| -> Result<String, ConfigError> {
            if use_env {
                env::var(key.to_uppercase()).map_err(|_| ConfigError::NotFound(key.to_string()))
            } else {
                self.get_secret(key)
            }
        };

        let mut result = HashMap::new();
        result.insert(primary.to_string(), get_value(primary)?);
        for &key in maybe_secret {
            if let Ok(v) = get_value(key) {
                result.insert(key.to_string(), v);
            }
        }
        Ok(result)
    }

    fn write_all_secrets(&self, values: &HashMap<String, Value>) -> Result<(), ConfigError> {
        match &self.secrets {
            #[cfg(feature = "system-keyring")]
            SecretStorage::Keyring { service } => {
                let json_value = serde_json::to_string(values)?;
                match self.handle_keyring_operation(
                    |entry| entry.set_password(&json_value),
                    service,
                    Some(values),
                ) {
                    Ok(_) => {}
                    Err(ConfigError::FallbackToFileStorage) => {}
                    Err(e) => return Err(e),
                }
            }
            SecretStorage::File { path } => {
                let yaml_value = serde_yaml::to_string(values)?;
                write_secrets_file(path, &yaml_value)?;
            }
        }

        self.invalidate_secrets_cache();
        Ok(())
    }

    fn mutate_secrets(
        &self,
        mutate: impl FnOnce(&mut HashMap<String, Value>),
    ) -> Result<(), ConfigError> {
        let _guard = self.guard.lock().unwrap();
        let mut values = self.all_secrets()?;
        mutate(&mut values);
        self.write_all_secrets(&values)
    }

    pub fn set_secret<V>(&self, key: &str, value: &V) -> Result<(), ConfigError>
    where
        V: Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.mutate_secrets(|values| {
            values.insert(key.to_string(), value);
        })
    }

    pub fn set_secret_values(&self, updates: &[(String, Value)]) -> Result<(), ConfigError> {
        if updates.is_empty() {
            return Ok(());
        }

        self.mutate_secrets(|values| {
            for (key, value) in updates {
                values.insert(key.clone(), value.clone());
            }
        })
    }

    pub fn delete_secret(&self, key: &str) -> Result<(), ConfigError> {
        self.mutate_secrets(|values| {
            values.remove(key);
        })
    }

    pub fn delete_secret_values(&self, keys: &[String]) -> Result<(), ConfigError> {
        if keys.is_empty() {
            return Ok(());
        }

        self.mutate_secrets(|values| {
            for key in keys {
                values.remove(key);
            }
        })
    }

    fn read_secrets_from_file(&self, path: &Path) -> Result<HashMap<String, Value>, ConfigError> {
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

    #[cfg(feature = "system-keyring")]
    fn secrets_file_path() -> PathBuf {
        secrets_file_path_in(&Paths::config_dir())
    }

    #[cfg(feature = "system-keyring")]
    fn fallback_to_file_storage(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let path = Self::secrets_file_path();
        self.read_secrets_from_file(&path)
    }

    #[cfg(feature = "system-keyring")]
    fn write_secrets_to_file(&self, values: &HashMap<String, Value>) -> Result<(), ConfigError> {
        std::fs::create_dir_all(Paths::config_dir())?;
        let path = Self::secrets_file_path();
        let yaml_value = serde_yaml::to_string(values)?;
        write_secrets_file(&path, &yaml_value)?;
        Ok(())
    }

    pub fn invalidate_secrets_cache(&self) {
        let mut cache = self.secrets_cache.lock().unwrap();
        *cache = None;
    }

    #[cfg(feature = "system-keyring")]
    fn is_keyring_availability_error(&self, error_str: &str) -> bool {
        let lower = error_str.to_lowercase();
        lower.contains("keyring")
            || lower.contains("dbus")
            || lower.contains("org.freedesktop.secrets")
            || lower.contains("platform secure storage")
            || lower.contains("no secret service")
    }

    #[cfg(feature = "system-keyring")]
    fn get_keyring_entry(service: &str) -> Result<keyring::Entry, keyring::Error> {
        keyring::Entry::new(service, KEYRING_USERNAME)
    }

    #[cfg(feature = "system-keyring")]
    fn handle_keyring_fallback_error<T>(
        &self,
        keyring_err: &keyring::Error,
        fallback_values: Option<&HashMap<String, Value>>,
    ) -> Result<T, ConfigError> {
        if self.is_keyring_availability_error(&keyring_err.to_string()) {
            std::env::set_var("GOOSE_DISABLE_KEYRING", "1");
            tracing::warn!("Keyring unavailable. Using file storage for secrets.");

            if let Some(values) = fallback_values {
                self.write_secrets_to_file(values)?;
                Err(ConfigError::FallbackToFileStorage)
            } else {
                Err(ConfigError::FallbackToFileStorage)
            }
        } else {
            Err(ConfigError::KeyringError(keyring_err.to_string()))
        }
    }

    #[cfg(feature = "system-keyring")]
    fn handle_keyring_operation<T>(
        &self,
        operation: impl FnOnce(keyring::Entry) -> Result<T, keyring::Error>,
        service: &str,
        fallback_values: Option<&HashMap<String, Value>>,
    ) -> Result<T, ConfigError> {
        let entry = match Self::get_keyring_entry(service) {
            Ok(entry) => entry,
            Err(keyring_err) => {
                return self.handle_keyring_fallback_error(&keyring_err, fallback_values);
            }
        };

        match operation(entry) {
            Ok(result) => Ok(result),
            Err(keyring_err) => self.handle_keyring_fallback_error(&keyring_err, fallback_values),
        }
    }
}
