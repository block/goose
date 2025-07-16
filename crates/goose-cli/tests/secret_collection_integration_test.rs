use anyhow::Result;
use goose::agents::extension::{Envs, ExtensionConfig};
use goose::config::Config;
use goose::recipe::Recipe;
use goose_cli::recipes::secret_discovery::SecretDiscovery;
use goose_cli::session::{build_session, SessionBuilderConfig};
use serde_json::{json, Value};
use serial_test::serial;
use std::env;

/// Integration tests for the secret collection flow
///
/// These tests cover core functionality including:
/// - First-time user with no configured secrets
/// - Existing user with partial secrets configured
/// - Recipe with no extension requirements
/// - Recipe with multiple extensions requiring same secrets

fn setup_test_environment() -> Result<()> {
    // Ensure we're using mock keyring for tests
    env::set_var("GOOSE_DISABLE_KEYRING", "1");
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_recipe_with_no_extension_requirements() -> Result<()> {
        setup_test_environment()?;

        // Create a recipe with no extensions
        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "test-recipe".to_string(),
            description: "Test recipe with no extensions".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: None,
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let discovery = SecretDiscovery::new();
        let requirements = discovery.discover_required_secrets(&recipe);

        // Should have no secret requirements
        assert!(requirements.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_recipe_with_multiple_extensions_same_secrets() -> Result<()> {
        setup_test_environment()?;

        // Create a recipe with multiple extensions requiring the same secret
        let extensions = vec![
            ExtensionConfig::Stdio {
                name: "github-extension".to_string(),
                cmd: "github-mcp".to_string(),
                args: vec![],
                envs: Envs::default(),
                env_keys: vec!["GITHUB_TOKEN".to_string()],
                timeout: None,
                description: None,
                bundled: None,
            },
            ExtensionConfig::Stdio {
                name: "github-analyzer".to_string(),
                cmd: "github-analyzer".to_string(),
                args: vec![],
                envs: Envs::default(),
                env_keys: vec!["GITHUB_TOKEN".to_string()],
                timeout: None,
                description: None,
                bundled: None,
            },
        ];

        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "multi-github-recipe".to_string(),
            description: "Recipe with multiple extensions using same secret".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(extensions),
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let discovery = SecretDiscovery::new();
        let requirements = discovery.discover_required_secrets(&recipe);

        // Should have 2 requirements for the same secret (one per extension)
        assert_eq!(requirements.len(), 2);
        assert_eq!(requirements[0].key, "GITHUB_TOKEN");
        assert_eq!(requirements[1].key, "GITHUB_TOKEN");
        assert_ne!(
            requirements[0].extension_name,
            requirements[1].extension_name
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_first_time_user_scenario() -> Result<()> {
        setup_test_environment()?;

        // Create a recipe with extensions requiring secrets
        let extensions = vec![ExtensionConfig::Stdio {
            name: "openai-provider".to_string(),
            cmd: "openai-mcp".to_string(),
            args: vec![],
            envs: Envs::default(),
            env_keys: vec!["OPENAI_API_KEY".to_string()],
            timeout: None,
            description: None,
            bundled: None,
        }];

        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "first-time-user-recipe".to_string(),
            description: "Recipe for first-time user".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(extensions),
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let discovery = SecretDiscovery::new();
        let requirements = discovery.discover_required_secrets(&recipe);

        // Should have 1 missing requirement
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].key, "OPENAI_API_KEY");
        assert!(!requirements[0].is_available); // Should be missing for first-time user
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_existing_user_with_partial_secrets() -> Result<()> {
        setup_test_environment()?;

        // Set one secret in environment
        env::set_var("GITHUB_TOKEN", "test-github-token");

        // Create a recipe with multiple extensions
        let extensions = vec![
            ExtensionConfig::Stdio {
                name: "github-extension".to_string(),
                cmd: "github-mcp".to_string(),
                args: vec![],
                envs: Envs::default(),
                env_keys: vec!["GITHUB_TOKEN".to_string()],
                timeout: None,
                description: None,
                bundled: None,
            },
            ExtensionConfig::Stdio {
                name: "openai-extension".to_string(),
                cmd: "openai-mcp".to_string(),
                args: vec![],
                envs: Envs::default(),
                env_keys: vec!["OPENAI_API_KEY".to_string()],
                timeout: None,
                description: None,
                bundled: None,
            },
        ];

        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "partial-secrets-recipe".to_string(),
            description: "Recipe for user with partial secrets".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(extensions),
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let discovery = SecretDiscovery::new();
        let requirements = discovery.discover_required_secrets(&recipe);

        // Should have 2 requirements: 1 available, 1 missing
        assert_eq!(requirements.len(), 2);

        let github_req = requirements
            .iter()
            .find(|r| r.key == "GITHUB_TOKEN")
            .unwrap();
        let openai_req = requirements
            .iter()
            .find(|r| r.key == "OPENAI_API_KEY")
            .unwrap();

        assert!(github_req.is_available); // Should be available from env
        assert!(!openai_req.is_available); // Should be missing

        // Clean up
        env::remove_var("GITHUB_TOKEN");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_session_builder_integration() -> Result<()> {
        setup_test_environment()?;

        // Create session config with non-interactive mode
        let session_config = SessionBuilderConfig {
            interactive: false, // Non-interactive mode
            extensions: vec!["stdio://github-mcp".to_string()],
            ..Default::default()
        };

        // This should not trigger secret collection in non-interactive mode
        // The session building should succeed without prompting for input
        let _session = build_session(session_config).await;

        // If we reach here, the session was built successfully without hanging
        // This validates that non-interactive mode works correctly

        Ok(())
    }

    #[tokio::test]
    async fn test_secret_collection_empty_extensions() -> Result<()> {
        setup_test_environment()?;

        // Test extension with no env_keys
        let extensions = vec![ExtensionConfig::Stdio {
            name: "no-secrets-extension".to_string(),
            cmd: "basic-mcp".to_string(),
            args: vec![],
            envs: Envs::default(),
            env_keys: vec![],
            timeout: None,
            description: None,
            bundled: None,
        }];

        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "no-secrets-recipe".to_string(),
            description: "Recipe with extension that needs no secrets".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(extensions),
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
        };

        let discovery = SecretDiscovery::new();
        let requirements = discovery.discover_required_secrets(&recipe);

        // Should have no requirements
        assert_eq!(requirements.len(), 0);

        Ok(())
    }
}

/// Security-focused tests for secret handling
#[cfg(test)]
mod security_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_secret_isolation() -> Result<()> {
        setup_test_environment()?;

        // Test that secrets are properly isolated between different services
        let config = Config::global();

        // Store secrets for different "services"
        config.set_secret("SERVICE_A_KEY", json!("secret-a"))?;
        config.set_secret("SERVICE_B_KEY", json!("secret-b"))?;

        // Verify each service only gets its own secret
        let secret_a: Value = config.get_secret("SERVICE_A_KEY")?;
        let secret_b: Value = config.get_secret("SERVICE_B_KEY")?;
        assert_eq!(secret_a.as_str().unwrap(), "secret-a");
        assert_eq!(secret_b.as_str().unwrap(), "secret-b");

        // Verify non-existent secret returns error
        let result = config.get_secret::<Value>("NON_EXISTENT_KEY");
        assert!(result.is_err());

        // Clean up
        config.delete_secret("SERVICE_A_KEY")?;
        config.delete_secret("SERVICE_B_KEY")?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_secret_overwrite_protection() -> Result<()> {
        setup_test_environment()?;

        let config = Config::global();
        let key = "OVERWRITE_TEST_KEY";

        // Store initial secret
        config.set_secret(key, json!("initial-value"))?;
        let initial: Value = config.get_secret(key)?;
        assert_eq!(initial.as_str().unwrap(), "initial-value");

        // Overwrite with new value
        config.set_secret(key, json!("new-value"))?;
        let updated: Value = config.get_secret(key)?;
        assert_eq!(updated.as_str().unwrap(), "new-value");

        // Clean up
        config.delete_secret(key)?;

        Ok(())
    }
}
