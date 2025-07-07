use super::{KeyringBackend, KeyringError};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FileKeyringBackend {
    secrets_path: PathBuf,
}

impl FileKeyringBackend {
    pub fn new(secrets_path: PathBuf) -> Self {
        Self { secrets_path }
    }

    fn load_all_secrets(&self) -> Result<HashMap<String, String>> {
        if self.secrets_path.exists() {
            let file_content = std::fs::read_to_string(&self.secrets_path)?;
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&file_content)?;
            let json_value: Value = serde_json::to_value(yaml_value)?;
            match json_value {
                Value::Object(map) => {
                    let mut result = HashMap::new();

                    // Check if this is the new format (has "goose:secrets" key)
                    let service_key = Self::make_key("goose", "secrets");
                    if let Some(service_value) = map.get(&service_key) {
                        // New format: decode JSON from the service key
                        if let Some(json_str) = service_value.as_str() {
                            if let Ok(secrets_map) =
                                serde_json::from_str::<HashMap<String, Value>>(json_str)
                            {
                                for (key, value) in secrets_map {
                                    if let Some(string_value) = value.as_str() {
                                        result.insert(key, string_value.to_string());
                                    } else {
                                        result.insert(key, serde_json::to_string(&value)?);
                                    }
                                }
                                return Ok(result);
                            }
                        }
                    }

                    // Legacy format: direct key-value mapping (read-only)
                    for (key, value) in &map {
                        if let Some(string_value) = value.as_str() {
                            result.insert(key.clone(), string_value.to_string());
                        } else {
                            result.insert(key.clone(), serde_json::to_string(&value)?);
                        }
                    }

                    Ok(result)
                }
                _ => Ok(HashMap::new()),
            }
        } else {
            Ok(HashMap::new())
        }
    }

    fn save_all_secrets(&self, secrets: &HashMap<String, String>) -> Result<()> {
        // Convert strings back to appropriate JSON values
        let mut json_map = serde_json::Map::new();
        for (key, value) in secrets {
            // Try to parse as JSON first, fall back to string
            let json_value =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()));
            json_map.insert(key.clone(), json_value);
        }

        let yaml_value = serde_yaml::to_string(&json_map)?;

        // Ensure parent directory exists
        if let Some(parent) = self.secrets_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&self.secrets_path, yaml_value)?;
        Ok(())
    }

    fn make_key(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

impl KeyringBackend for FileKeyringBackend {
    fn get_password(&self, service: &str, username: &str) -> Result<String> {
        let key = Self::make_key(service, username);
        let secrets = self.load_all_secrets()?;

        secrets.get(&key).cloned().ok_or_else(|| {
            KeyringError::NotFound {
                service: service.to_string(),
                username: username.to_string(),
            }
            .into()
        })
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        let mut secrets = self.load_all_secrets()?;
        secrets.insert(key, password.to_string());
        self.save_all_secrets(&secrets)?;
        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        let mut secrets = self.load_all_secrets()?;
        secrets.remove(&key);
        self.save_all_secrets(&secrets)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_basic_operations() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Test setting a password
        backend.set_password("test_service", "test_user", "test_password")?;

        // Test getting the password
        let password = backend.get_password("test_service", "test_user")?;
        assert_eq!(password, "test_password");

        // Test deleting the password
        backend.delete_password("test_service", "test_user")?;

        // Test that getting deleted password returns NotFound error
        let result = backend.get_password("test_service", "test_user");
        assert!(result.is_err());
        if let Err(e) = result {
            if let Some(keyring_err) = e.downcast_ref::<KeyringError>() {
                assert!(matches!(keyring_err, KeyringError::NotFound { .. }));
            } else {
                panic!("Expected KeyringError::NotFound, got: {:?}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_multiple_services() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Set passwords for different services
        backend.set_password("service1", "user1", "password1")?;
        backend.set_password("service2", "user2", "password2")?;
        backend.set_password("service1", "user2", "password3")?;

        // Verify all passwords can be retrieved correctly
        assert_eq!(backend.get_password("service1", "user1")?, "password1");
        assert_eq!(backend.get_password("service2", "user2")?, "password2");
        assert_eq!(backend.get_password("service1", "user2")?, "password3");

        // Delete one password and verify others remain
        backend.delete_password("service1", "user1")?;
        assert!(backend.get_password("service1", "user1").is_err());
        assert_eq!(backend.get_password("service2", "user2")?, "password2");
        assert_eq!(backend.get_password("service1", "user2")?, "password3");

        Ok(())
    }

    #[test]
    fn test_password_update() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Set initial password
        backend.set_password("service", "user", "old_password")?;
        assert_eq!(backend.get_password("service", "user")?, "old_password");

        // Update password
        backend.set_password("service", "user", "new_password")?;
        assert_eq!(backend.get_password("service", "user")?, "new_password");

        Ok(())
    }

    #[test]
    fn test_nonexistent_file() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file); // Delete the file

        let backend = FileKeyringBackend::new(file_path);

        // Getting from non-existent file should return NotFound
        let result = backend.get_password("service", "user");
        assert!(result.is_err());
        if let Err(e) = result {
            if let Some(keyring_err) = e.downcast_ref::<KeyringError>() {
                assert!(matches!(keyring_err, KeyringError::NotFound { .. }));
            }
        }

        // Setting should create the file
        backend.set_password("service", "user", "password")?;
        assert_eq!(backend.get_password("service", "user")?, "password");

        Ok(())
    }

    #[test]
    fn test_empty_password() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Test setting and getting empty password
        backend.set_password("service", "user", "")?;
        assert_eq!(backend.get_password("service", "user")?, "");

        Ok(())
    }

    #[test]
    fn test_special_characters_in_credentials() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Test with special characters in service, user, and password
        let service = "service-with-dashes_and_underscores.and.dots";
        let user = "user@domain.com";
        let password = "password with spaces & special chars: !@#$%^&*()";

        backend.set_password(service, user, password)?;
        assert_eq!(backend.get_password(service, user)?, password);

        Ok(())
    }

    #[test]
    fn test_json_content_in_password() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Test storing JSON content as password
        let json_password =
            r#"{"access_token":"abc123","refresh_token":"def456","expires_in":3600}"#;
        backend.set_password("oauth_service", "user", json_password)?;

        let retrieved = backend.get_password("oauth_service", "user")?;

        // Parse both as JSON to compare content regardless of key order
        let original: serde_json::Value = serde_json::from_str(json_password).unwrap();
        let retrieved_parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
        assert_eq!(original, retrieved_parsed);

        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_password() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Deleting non-existent password should not error (idempotent)
        backend.delete_password("nonexistent_service", "nonexistent_user")?;

        Ok(())
    }

    #[test]
    fn test_legacy_format_compatibility() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();

        // Write a legacy format secrets.yaml file
        let legacy_content = r#"openai_api_key: sk-abc123
anthropic_api_key: ant-def456
complex_config:
  nested: value
  number: 42
"#;
        std::fs::write(temp_file.path(), legacy_content)?;

        // Load with FileKeyringBackend - should read legacy format
        let backend = FileKeyringBackend::new(temp_file.path().to_path_buf());

        // Load secrets should work with legacy format (read-only)
        let secrets = backend.load_all_secrets()?;
        assert_eq!(secrets.get("openai_api_key").unwrap(), "sk-abc123");
        assert_eq!(secrets.get("anthropic_api_key").unwrap(), "ant-def456");
        assert!(secrets.get("complex_config").unwrap().contains("nested"));

        // Verify the original file format is preserved (no auto-migration)
        let file_content = std::fs::read_to_string(temp_file.path())?;
        assert!(file_content.contains("openai_api_key: sk-abc123"));
        assert!(!file_content.contains("goose:secrets"));

        Ok(())
    }
}
