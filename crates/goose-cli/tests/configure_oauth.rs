use goose::providers::{providers, base::ConfigKey};

// Test that verifies GitHub Copilot provider is properly registered
#[test]
fn test_github_copilot_provider_registered() {
    let all_providers = providers();
    
    // Check if GitHub Copilot is in the provider list
    let github_copilot = all_providers
        .iter()
        .find(|p| p.name == "github_copilot");
    
    assert!(github_copilot.is_some(), "GitHub Copilot provider should be registered");
    
    let provider = github_copilot.unwrap();
    assert_eq!(provider.display_name, "Github Copilot");
    
    // Verify it has OAuth configuration
    let has_oauth_key = provider.config_keys
        .iter()
        .any(|key| key.oauth_flow);
    
    assert!(has_oauth_key, "GitHub Copilot should have at least one OAuth configuration key");
}

// Test that verifies OAuth ConfigKey creation
#[test]
fn test_oauth_config_key_creation() {
    let oauth_key = ConfigKey::new_oauth("GITHUB_COPILOT_TOKEN", true, true, None);
    
    assert_eq!(oauth_key.name, "GITHUB_COPILOT_TOKEN");
    assert!(oauth_key.required);
    assert!(oauth_key.secret);
    assert!(oauth_key.oauth_flow);
    assert_eq!(oauth_key.default, None);
}

// Test that verifies regular ConfigKey does not have OAuth flow enabled
#[test]
fn test_regular_config_key_no_oauth() {
    let regular_key = ConfigKey::new("API_KEY", true, true, None);
    
    assert_eq!(regular_key.name, "API_KEY");
    assert!(regular_key.required);
    assert!(regular_key.secret);
    assert!(!regular_key.oauth_flow); // Should be false by default
    assert_eq!(regular_key.default, None);
}

// Test GitHub Copilot provider metadata structure
#[test]
fn test_github_copilot_metadata_structure() {
    let all_providers = providers();
    
    let github_copilot = all_providers
        .iter()
        .find(|p| p.name == "github_copilot")
        .expect("GitHub Copilot provider should be registered");
    
    // Verify basic metadata
    assert_eq!(github_copilot.name, "github_copilot");
    assert_eq!(github_copilot.display_name, "Github Copilot");
    assert!(!github_copilot.description.is_empty());
    assert!(!github_copilot.default_model.is_empty());
    
    // Verify it has known models
    assert!(!github_copilot.known_models.is_empty(), "GitHub Copilot should have known models");
    
    // Verify configuration keys
    assert!(!github_copilot.config_keys.is_empty(), "GitHub Copilot should have configuration keys");
    
    // Find the OAuth token key
    let oauth_token_key = github_copilot.config_keys
        .iter()
        .find(|key| key.name == "GITHUB_COPILOT_TOKEN" && key.oauth_flow);
    
    assert!(oauth_token_key.is_some(), "GitHub Copilot should have GITHUB_COPILOT_TOKEN OAuth key");
    
    let token_key = oauth_token_key.unwrap();
    assert!(token_key.required, "GitHub Copilot token should be required");
    assert!(token_key.secret, "GitHub Copilot token should be secret");
    assert!(token_key.oauth_flow, "GitHub Copilot token should use OAuth flow");
}

// Test for provider list completeness
#[test]
fn test_provider_list_includes_oauth_providers() {
    let all_providers = providers();
    
    // Count OAuth-enabled providers
    let oauth_providers: Vec<_> = all_providers
        .iter()
        .filter(|p| p.config_keys.iter().any(|key| key.oauth_flow))
        .collect();
    
    assert!(!oauth_providers.is_empty(), "Should have at least one OAuth-enabled provider");
    
    // Verify GitHub Copilot is among them
    let github_copilot_in_oauth = oauth_providers
        .iter()
        .any(|p| p.name == "github_copilot");
    
    assert!(github_copilot_in_oauth, "GitHub Copilot should be in OAuth providers list");
    
    // Print all OAuth providers for debugging
    println!("OAuth-enabled providers:");
    for provider in oauth_providers {
        println!("  - {} ({})", provider.name, provider.display_name);
        for key in &provider.config_keys {
            if key.oauth_flow {
                println!("    OAuth key: {}", key.name);
            }
        }
    }
}
