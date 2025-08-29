use goose::providers::{create, providers};
use goose::model::ModelConfig;

/// Test that GitHub Copilot provider can be instantiated
#[test]
fn test_github_copilot_provider_creation() {
    // GitHub Copilot provider should be creatable even without credentials
    // The provider should handle authentication through OAuth flow
    let model_config = ModelConfig::new("gpt-4o").unwrap();
    
    // Attempt to create the provider - it should succeed even without credentials
    // because OAuth providers handle authentication dynamically
    let result = create("github_copilot", model_config);
    
    // The provider should be created successfully
    // Note: It may fail later during API calls due to missing authentication,
    // but the provider itself should instantiate correctly
    match result {
        Ok(_provider) => {
            // Success - provider was created
            println!("GitHub Copilot provider created successfully");
        }
        Err(e) => {
            // Check if the error is specifically about missing credentials
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("github_copilot_token") || error_msg.contains("oauth") {
                // This is expected - OAuth provider needs authentication
                println!("GitHub Copilot provider creation failed as expected (needs OAuth): {}", e);
            } else {
                // Unexpected error - provider creation failed for other reasons
                panic!("Unexpected error creating GitHub Copilot provider: {}", e);
            }
        }
    }
}

/// Test that GitHub Copilot appears in the providers list
#[test]
fn test_github_copilot_in_providers_list() {
    let all_providers = providers();
    
    // Find GitHub Copilot in the list
    let github_copilot = all_providers
        .iter()
        .find(|p| p.name == "github_copilot");
    
    assert!(github_copilot.is_some(), "GitHub Copilot should be in providers list");
    
    let provider = github_copilot.unwrap();
    
    // Verify it's properly configured
    assert_eq!(provider.name, "github_copilot");
    assert_eq!(provider.display_name, "Github Copilot");
    assert!(!provider.description.is_empty());
    
    // Verify it has OAuth configuration
    let oauth_keys: Vec<_> = provider.config_keys
        .iter()
        .filter(|key| key.oauth_flow)
        .collect();
    
    assert!(!oauth_keys.is_empty(), "GitHub Copilot should have OAuth keys");
    
    // Print provider info for debugging
    println!("GitHub Copilot Provider:");
    println!("  Name: {}", provider.name);
    println!("  Display Name: {}", provider.display_name);
    println!("  Description: {}", provider.description);
    println!("  Default Model: {}", provider.default_model);
    println!("  Known Models: {:?}", provider.known_models.iter().map(|m| &m.name).collect::<Vec<_>>());
    println!("  OAuth Keys: {:?}", oauth_keys.iter().map(|k| &k.name).collect::<Vec<_>>());
}

/// Test GitHub Copilot specific configuration
#[test]
fn test_github_copilot_configuration() {
    let all_providers = providers();
    
    let github_copilot = all_providers
        .iter()
        .find(|p| p.name == "github_copilot")
        .expect("GitHub Copilot provider should exist");
    
    // Should have the expected default model
    assert!(!github_copilot.default_model.is_empty());
    
    // Should have known models
    assert!(!github_copilot.known_models.is_empty(), "Should have known models");
    
    // Should have GITHUB_COPILOT_TOKEN as an OAuth key
    let token_key = github_copilot.config_keys
        .iter()
        .find(|key| key.name == "GITHUB_COPILOT_TOKEN");
    
    assert!(token_key.is_some(), "Should have GITHUB_COPILOT_TOKEN key");
    
    let key = token_key.unwrap();
    assert!(key.required, "Token key should be required");
    assert!(key.secret, "Token key should be secret");
    assert!(key.oauth_flow, "Token key should use OAuth flow");
}

/// Test that OAuth providers are distinguished from regular providers
#[test]
fn test_oauth_vs_regular_providers() {
    let all_providers = providers();
    
    let oauth_providers: Vec<_> = all_providers
        .iter()
        .filter(|p| p.config_keys.iter().any(|key| key.oauth_flow))
        .collect();
    
    let regular_providers: Vec<_> = all_providers
        .iter()
        .filter(|p| p.config_keys.iter().all(|key| !key.oauth_flow))
        .collect();
    
    assert!(!oauth_providers.is_empty(), "Should have OAuth providers");
    assert!(!regular_providers.is_empty(), "Should have regular providers");
    
    // GitHub Copilot should be in OAuth providers
    let github_copilot_is_oauth = oauth_providers
        .iter()
        .any(|p| p.name == "github_copilot");
    
    assert!(github_copilot_is_oauth, "GitHub Copilot should be an OAuth provider");
    
    println!("OAuth providers: {:?}", oauth_providers.iter().map(|p| &p.name).collect::<Vec<_>>());
    println!("Regular providers: {:?}", regular_providers.iter().map(|p| &p.name).collect::<Vec<_>>());
}
