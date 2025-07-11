use goose::{
    agents::extension::ExtensionConfig,
    config::Config,
    keyring::{KeyringBackend, SystemKeyringBackend},
    recipe::Recipe,
};
use std::sync::Arc;

/// Represents a required secret for an extension
#[derive(Debug, Clone)]
pub struct SecretRequirement {
    pub key: String,
    pub extension_name: String,
    pub description: Option<String>,
    pub is_available: bool,
    pub source: SecretSource,
}

/// Indicates where a secret value is found or missing
#[derive(Debug, Clone)]
pub enum SecretSource {
    Missing,
    Environment,
    Keyring,
}

/// Engine for discovering required secrets from recipes and extensions
pub struct SecretDiscovery {
    #[allow(dead_code)]
    keyring_backend: Arc<dyn KeyringBackend>,
}

impl SecretDiscovery {
    /// Create a new SecretDiscovery instance
    pub fn new() -> Self {
        Self {
            keyring_backend: Arc::new(SystemKeyringBackend),
        }
    }

    /// Create a new SecretDiscovery instance with a custom keyring backend
    pub fn with_keyring_backend(keyring_backend: Arc<dyn KeyringBackend>) -> Self {
        Self { keyring_backend }
    }

    /// Analyze a recipe and collect all required secrets from its extensions
    pub fn discover_required_secrets(&self, recipe: &Recipe) -> Vec<SecretRequirement> {
        let mut requirements = Vec::new();

        if let Some(extensions) = &recipe.extensions {
            for extension in extensions {
                let extension_requirements = self.discover_extension_secrets(extension);
                requirements.extend(extension_requirements);
            }
        }

        requirements
    }

    /// Discover required secrets for a specific extension configuration
    pub fn discover_extension_secrets(&self, config: &ExtensionConfig) -> Vec<SecretRequirement> {
        let extension_name = config.name();
        let env_keys = self.get_env_keys_from_config(config);

        env_keys
            .into_iter()
            .map(|key| {
                let (is_available, source) = self.check_secret_availability(&key);
                SecretRequirement {
                    key: key.clone(),
                    extension_name: extension_name.clone(),
                    description: None,
                    is_available,
                    source,
                }
            })
            .collect()
    }

    /// Check if a specific secret is available in environment or keyring
    pub fn is_secret_available(&self, key: &str) -> bool {
        let (is_available, _) = self.check_secret_availability(key);
        is_available
    }

    /// Get the list of env_keys from an ExtensionConfig
    fn get_env_keys_from_config(&self, config: &ExtensionConfig) -> Vec<String> {
        match config {
            ExtensionConfig::Sse { env_keys, .. } => env_keys.clone(),
            ExtensionConfig::Stdio { env_keys, .. } => env_keys.clone(),
            ExtensionConfig::StreamableHttp { env_keys, .. } => env_keys.clone(),
            ExtensionConfig::Builtin { .. } => Vec::new(),
            ExtensionConfig::Frontend { .. } => Vec::new(),
        }
    }

    /// Check availability of a secret and determine its source
    fn check_secret_availability(&self, key: &str) -> (bool, SecretSource) {
        // First check environment variables
        if std::env::var(key).is_ok() {
            return (true, SecretSource::Environment);
        }

        // Then check keyring/config system
        let config = Config::global();
        match config.get(key, true) {
            Ok(value) => {
                if value.is_null() {
                    (false, SecretSource::Missing)
                } else if value.is_string() {
                    (true, SecretSource::Keyring)
                } else {
                    (false, SecretSource::Missing)
                }
            }
            Err(_) => (false, SecretSource::Missing),
        }
    }
}

impl Default for SecretDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::{keyring::MockKeyringBackend, recipe::Recipe};

    fn create_test_discovery() -> SecretDiscovery {
        let mock_keyring = Arc::new(MockKeyringBackend::new());
        SecretDiscovery::with_keyring_backend(mock_keyring)
    }

    #[test]
    fn test_discover_extension_secrets_stdio() {
        let discovery = create_test_discovery();

        let config = ExtensionConfig::Stdio {
            name: "github-mcp".to_string(),
            cmd: "github-mcp".to_string(),
            args: vec![],
            envs: Default::default(),
            env_keys: vec!["GITHUB_TOKEN".to_string(), "GITHUB_REPO".to_string()],
            timeout: Some(300),
            description: Some("GitHub MCP extension".to_string()),
            bundled: Some(false),
        };

        let requirements = discovery.discover_extension_secrets(&config);

        assert_eq!(requirements.len(), 2);
        assert_eq!(requirements[0].key, "GITHUB_TOKEN");
        assert_eq!(requirements[0].extension_name, "github-mcp");
        assert!(!requirements[0].is_available);
        assert!(matches!(requirements[0].source, SecretSource::Missing));

        assert_eq!(requirements[1].key, "GITHUB_REPO");
        assert_eq!(requirements[1].extension_name, "github-mcp");
        assert!(!requirements[1].is_available);
    }

    #[test]
    fn test_discover_extension_secrets_sse() {
        let discovery = create_test_discovery();

        let config = ExtensionConfig::Sse {
            name: "openai-provider".to_string(),
            uri: "http://example.com/sse".to_string(),
            envs: Default::default(),
            env_keys: vec!["OPENAI_API_KEY".to_string()],
            description: Some("OpenAI provider".to_string()),
            timeout: Some(300),
            bundled: Some(false),
        };

        let requirements = discovery.discover_extension_secrets(&config);

        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].key, "OPENAI_API_KEY");
        assert_eq!(requirements[0].extension_name, "openai-provider");
        assert!(!requirements[0].is_available);
    }

    #[test]
    fn test_discover_extension_secrets_builtin() {
        let discovery = create_test_discovery();

        let config = ExtensionConfig::Builtin {
            name: "developer".to_string(),
            display_name: Some("Developer Tools".to_string()),
            timeout: Some(300),
            bundled: Some(true),
        };

        let requirements = discovery.discover_extension_secrets(&config);

        // Builtin extensions don't have env_keys
        assert_eq!(requirements.len(), 0);
    }

    #[test]
    fn test_discover_required_secrets_from_recipe() {
        let discovery = create_test_discovery();

        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(vec![
                ExtensionConfig::Stdio {
                    name: "github-mcp".to_string(),
                    cmd: "github-mcp".to_string(),
                    args: vec![],
                    envs: Default::default(),
                    env_keys: vec!["GITHUB_TOKEN".to_string()],
                    timeout: Some(300),
                    description: Some("GitHub MCP extension".to_string()),
                    bundled: Some(false),
                },
                ExtensionConfig::Sse {
                    name: "openai-provider".to_string(),
                    uri: "http://example.com/sse".to_string(),
                    envs: Default::default(),
                    env_keys: vec!["OPENAI_API_KEY".to_string()],
                    description: Some("OpenAI provider".to_string()),
                    timeout: Some(300),
                    bundled: Some(false),
                },
            ]),
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let requirements = discovery.discover_required_secrets(&recipe);

        assert_eq!(requirements.len(), 2);
        assert_eq!(requirements[0].key, "GITHUB_TOKEN");
        assert_eq!(requirements[0].extension_name, "github-mcp");
        assert_eq!(requirements[1].key, "OPENAI_API_KEY");
        assert_eq!(requirements[1].extension_name, "openai-provider");
    }

    #[test]
    fn test_check_secret_availability_with_env_var() {
        let discovery = create_test_discovery();

        // Set an environment variable for testing
        std::env::set_var("TEST_SECRET", "test_value");

        let (is_available, source) = discovery.check_secret_availability("TEST_SECRET");
        assert!(is_available);
        assert!(matches!(source, SecretSource::Environment));

        // Clean up
        std::env::remove_var("TEST_SECRET");
    }

    #[test]
    fn test_check_secret_availability_missing() {
        let discovery = create_test_discovery();

        let (is_available, source) = discovery.check_secret_availability("NONEXISTENT_SECRET");
        assert!(!is_available);
        assert!(matches!(source, SecretSource::Missing));
    }
}
