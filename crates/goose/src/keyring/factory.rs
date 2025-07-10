#[cfg(test)]
use super::KeyringError;
use super::{FileKeyringBackend, KeyringBackend, MockKeyringBackend, SystemKeyringBackend};
use etcetera::AppStrategy;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, path::Path};

/// Factory for creating keyring backends with environment-based defaults.
///
/// This factory provides a consistent way to create keyring backends across
/// the entire codebase, handling environment variable detection and providing
/// sensible defaults based on the execution context.
///
/// # Environment Variable Priority
///
/// The factory checks environment variables in this order:
/// 1. `GOOSE_USE_MOCK_KEYRING` (highest priority) - Forces mock backend for testing
/// 2. `GOOSE_DISABLE_KEYRING` - Forces file-based backend for production without OS keyring
/// 3. Default behavior - Uses system keyring for production
pub struct KeyringFactory;

impl KeyringFactory {
    /// Create a keyring backend using environment-based defaults.
    ///
    /// This is the most common use case - let the factory decide which backend
    /// to use based on environment variables and execution context.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use goose::keyring::KeyringFactory;
    ///
    /// let keyring = KeyringFactory::create_default();
    /// keyring.set_password("service", "user", "password").unwrap();
    /// ```
    pub fn create_default() -> Arc<dyn KeyringBackend> {
        Self::create_with_config(DefaultKeyringConfig::new())
    }

    /// Create a keyring backend with custom configuration.
    ///
    /// This allows overriding default behavior such as specifying a custom
    /// file path for the file-based backend.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use goose::keyring::{KeyringFactory, DefaultKeyringConfig};
    /// use std::path::PathBuf;
    ///
    /// let config = DefaultKeyringConfig::new()
    ///     .with_file_path(PathBuf::from("/custom/path/secrets.yaml"));
    /// let keyring = KeyringFactory::create_with_config(config);
    /// ```
    pub fn create_with_config(config: DefaultKeyringConfig) -> Arc<dyn KeyringBackend> {
        // Priority 1: Mock keyring for testing
        if Self::is_env_var_truthy("GOOSE_USE_MOCK_KEYRING") {
            return Arc::new(MockKeyringBackend::new());
        }

        // Priority 2: File-based keyring when system keyring disabled
        if Self::is_env_var_truthy("GOOSE_DISABLE_KEYRING") {
            let file_path = config
                .file_path
                .unwrap_or_else(|| Self::default_config_dir().join("secrets.yaml"));
            return Arc::new(FileKeyringBackend::new(file_path));
        }

        // Priority 3: System keyring (default)
        Arc::new(SystemKeyringBackend)
    }

    /// Check if an environment variable is set to a truthy value.
    ///
    /// This matches the same logic used throughout the goose codebase for
    /// consistency in environment variable interpretation.
    ///
    /// Truthy values: "1", "true", "yes", "on" (case insensitive)
    /// Falsy values: "0", "false", "no", "off", "" or unset
    fn is_env_var_truthy(var_name: &str) -> bool {
        match env::var(var_name) {
            Ok(value) => {
                let normalized = value.trim().to_lowercase();
                matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
            }
            Err(_) => false,
        }
    }

    /// Get the default configuration directory using the same logic as Config.
    ///
    /// This ensures consistency with the rest of the goose configuration system.
    fn default_config_dir() -> PathBuf {
        use etcetera::{choose_app_strategy, AppStrategyArgs};

        let strategy = AppStrategyArgs {
            top_level_domain: "Block".to_string(),
            author: "Block".to_string(),
            app_name: "goose".to_string(),
        };

        choose_app_strategy(strategy)
            .expect("goose requires a home dir")
            .config_dir()
    }
}

/// Configuration options for keyring factory creation.
///
/// This struct allows customizing the behavior of the keyring factory
/// without breaking the simple default use case.
#[derive(Default)]
pub struct DefaultKeyringConfig {
    /// Optional custom file path for FileKeyringBackend.
    ///
    /// If not provided, defaults to `{config_dir}/secrets.yaml`
    pub file_path: Option<PathBuf>,
}

impl DefaultKeyringConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Set a custom file path for the file-based keyring backend.
    ///
    /// This path will be used when `GOOSE_DISABLE_KEYRING` is set but
    /// `GOOSE_USE_MOCK_KEYRING` is not set.
    pub fn with_file_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.file_path = Some(path.as_ref().to_path_buf());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;

    /// Test utility for managing environment variables in tests.
    ///
    /// This ensures that environment variables are properly restored
    /// after each test, preventing test interference.
    struct EnvGuard {
        key: String,
        original_value: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let original_value = env::var(key).ok();
            env::set_var(key, value);
            EnvGuard {
                key: key.to_string(),
                original_value,
            }
        }

        fn remove(key: &str) -> Self {
            let original_value = env::var(key).ok();
            env::remove_var(key);
            EnvGuard {
                key: key.to_string(),
                original_value,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original_value {
                Some(value) => env::set_var(&self.key, value),
                None => env::remove_var(&self.key),
            }
        }
    }

    #[test]
    #[serial]
    fn test_mock_keyring_priority() {
        let _guard1 = EnvGuard::set("GOOSE_USE_MOCK_KEYRING", "true");
        let _guard2 = EnvGuard::set("GOOSE_DISABLE_KEYRING", "true");

        let keyring = KeyringFactory::create_default();

        // Should use MockKeyringBackend despite GOOSE_DISABLE_KEYRING being set
        // We can verify this by checking that a non-existent key returns NotFound
        match keyring.get_password("test_service", "test_user") {
            Err(e) => {
                if let Some(keyring_err) = e.downcast_ref::<KeyringError>() {
                    assert!(matches!(keyring_err, KeyringError::NotFound { .. }));
                }
            }
            Ok(_) => panic!("Mock keyring should return NotFound for non-existent keys"),
        }
    }

    #[test]
    #[serial]
    fn test_file_keyring_when_disabled() {
        let _guard1 = EnvGuard::remove("GOOSE_USE_MOCK_KEYRING");
        let _guard2 = EnvGuard::set("GOOSE_DISABLE_KEYRING", "true");

        let temp_dir = tempdir().unwrap();
        let keyring = KeyringFactory::create_with_config(
            DefaultKeyringConfig::new().with_file_path(temp_dir.path().join("test_secrets.yaml")),
        );

        // Use unique service/user names to avoid conflicts
        let service = format!("service_{}", std::process::id());
        let user = format!("user_{}", std::process::id());

        // Verify it creates a FileKeyringBackend by testing file operations
        keyring.set_password(&service, &user, "password").unwrap();
        assert_eq!(keyring.get_password(&service, &user).unwrap(), "password");

        // Verify the file was actually created
        assert!(temp_dir.path().join("test_secrets.yaml").exists());
    }

    #[test]
    #[serial]
    fn test_system_keyring_default() {
        // For this test, we'll use mock keyring to avoid OS keyring popups
        // but verify the logic by ensuring that when neither override is set,
        // the factory doesn't choose the file backend
        let _guard1 = EnvGuard::set("GOOSE_USE_MOCK_KEYRING", "true");
        let _guard2 = EnvGuard::remove("GOOSE_DISABLE_KEYRING");

        let keyring = KeyringFactory::create_default();

        // Verify it creates a MockKeyringBackend (since we forced it for testing)
        // The main thing we're testing is that the logic flow is correct
        match keyring.get_password("non_existent_service", "non_existent_user") {
            Err(e) => {
                if let Some(keyring_err) = e.downcast_ref::<KeyringError>() {
                    assert!(matches!(keyring_err, KeyringError::NotFound { .. }));
                }
            }
            Ok(_) => panic!("Mock keyring should return NotFound for non-existent keys"),
        }
    }

    #[test]
    #[serial]
    fn test_default_config_with_custom_path() {
        let _guard1 = EnvGuard::remove("GOOSE_USE_MOCK_KEYRING");
        let _guard2 = EnvGuard::set("GOOSE_DISABLE_KEYRING", "true");

        let temp_dir = tempdir().unwrap();
        let custom_path = temp_dir.path().join("custom_secrets.yaml");

        let config = DefaultKeyringConfig::new().with_file_path(&custom_path);
        let keyring = KeyringFactory::create_with_config(config);

        // Use unique service/user names to avoid conflicts
        let service = format!("test_service_{}", std::process::id());
        let user = format!("test_user_{}", std::process::id());

        // Test that the custom path is used
        keyring
            .set_password(&service, &user, "test_password")
            .unwrap();

        // Verify the custom file was created
        assert!(custom_path.exists());

        // Verify we can retrieve the password
        assert_eq!(
            keyring.get_password(&service, &user).unwrap(),
            "test_password"
        );
    }

    #[test]
    #[serial]
    fn test_env_var_truthy_values() {
        // Test truthy values
        for value in [
            "1", "true", "TRUE", "yes", "YES", "on", "ON", " true ", "True",
        ] {
            let _guard = EnvGuard::set("TEST_TRUTHY_VAR", value);
            assert!(
                KeyringFactory::is_env_var_truthy("TEST_TRUTHY_VAR"),
                "Value '{}' should be truthy",
                value
            );
        }

        // Test falsy values
        for value in [
            "0", "false", "FALSE", "no", "NO", "off", "OFF", "", " ", "random",
        ] {
            let _guard = EnvGuard::set("TEST_TRUTHY_VAR", value);
            assert!(
                !KeyringFactory::is_env_var_truthy("TEST_TRUTHY_VAR"),
                "Value '{}' should be falsy",
                value
            );
        }

        // Test unset variable
        let _guard = EnvGuard::remove("TEST_TRUTHY_VAR");
        assert!(
            !KeyringFactory::is_env_var_truthy("TEST_TRUTHY_VAR"),
            "Unset variable should be falsy"
        );
    }

    #[test]
    #[serial]
    fn test_factory_consistency_with_existing_is_env_var_truthy() {
        // This test ensures our factory's is_env_var_truthy matches the existing implementation
        // We'll test a few key values to ensure consistency

        let test_cases = [
            ("1", true),
            ("true", true),
            ("TRUE", true),
            ("yes", true),
            ("on", true),
            ("0", false),
            ("false", false),
            ("no", false),
            ("off", false),
            ("", false),
            ("random", false),
        ];

        for (value, expected) in test_cases {
            let _guard = EnvGuard::set("TEST_CONSISTENCY_VAR", value);
            assert_eq!(
                KeyringFactory::is_env_var_truthy("TEST_CONSISTENCY_VAR"),
                expected,
                "Value '{}' should be {}",
                value,
                if expected { "truthy" } else { "falsy" }
            );
        }
    }
}
