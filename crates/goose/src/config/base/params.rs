use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;

impl Config {
    pub fn get(&self, key: &str, is_secret: bool) -> Result<Value, ConfigError> {
        if is_secret {
            self.get_secret(key)
        } else {
            self.get_param(key)
        }
    }

    pub fn set<V>(&self, key: &str, value: &V, is_secret: bool) -> Result<(), ConfigError>
    where
        V: Serialize,
    {
        if is_secret {
            self.set_secret(key, value)
        } else {
            self.set_param(key, value)
        }
    }

    pub fn get_param<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        let env_key = key.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            let value = Self::parse_env_value(&val)?;
            return Ok(serde_json::from_value(value)?);
        }

        let values = self.load()?;
        let value = values
            .get(key)
            .ok_or_else(|| ConfigError::NotFound(key.to_string()))?;

        match serde_yaml::from_value(value.clone()) {
            Ok(value) => Ok(value),
            Err(yaml_err) => {
                let Some(string_value) = value.as_str() else {
                    return Err(yaml_err.into());
                };
                let parsed = Self::parse_env_value(string_value)?;
                serde_json::from_value(parsed).map_err(|_| yaml_err.into())
            }
        }
    }

    pub fn update_param<T, V, F>(&self, key: &str, f: F) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Default,
        V: Serialize,
        F: FnOnce(T) -> V,
    {
        let _guard = self.guard.lock().unwrap();
        let mut values = self.load_write_config()?;
        let current: T = values
            .get(key)
            .and_then(|v| serde_yaml::from_value(v.clone()).ok())
            .unwrap_or_default();
        let updated = f(current);
        values.insert(serde_yaml::to_value(key)?, serde_yaml::to_value(updated)?);
        self.save_values(&values)
    }

    pub fn set_param<V: Serialize>(&self, key: &str, value: V) -> Result<(), ConfigError> {
        let _guard = self.guard.lock().unwrap();
        let mut values = self.load_write_config()?;
        values.insert(serde_yaml::to_value(key)?, serde_yaml::to_value(value)?);
        self.save_values(&values)
    }

    pub fn set_param_values(&self, updates: &[(String, Value)]) -> Result<(), ConfigError> {
        if updates.is_empty() {
            return Ok(());
        }

        let _guard = self.guard.lock().unwrap();
        let mut values = self.load_write_config()?;
        for (key, value) in updates {
            values.insert(serde_yaml::to_value(key)?, serde_yaml::to_value(value)?);
        }
        self.save_values(&values)
    }

    pub fn delete(&self, key: &str) -> Result<(), ConfigError> {
        let _guard = self.guard.lock().unwrap();
        let mut values = self.load_write_config()?;
        values.shift_remove(key);
        self.save_values(&values)
    }
}
