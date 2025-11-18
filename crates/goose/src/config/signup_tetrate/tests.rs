use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

#[test]
fn test_pkce_flow_creation() {
    let flow = PkceAuthFlow::new().unwrap();

    // Verify code_verifier is 128 characters
    assert_eq!(flow.code_verifier.len(), 128);

    // Verify code_verifier is alphanumeric
    assert!(flow.code_verifier.chars().all(|c| c.is_alphanumeric()));

    // Verify code_challenge is base64url encoded
    assert!(!flow.code_challenge.contains('+'));
    assert!(!flow.code_challenge.contains('/'));
    assert!(!flow.code_challenge.contains('='));
}

#[test]
fn test_code_challenge_generation() {
    let flow = PkceAuthFlow::new().unwrap();

    // Manually compute the expected challenge
    let mut hasher = Sha256::new();
    hasher.update(&flow.code_verifier);
    let hash = hasher.finalize();
    let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

    assert_eq!(flow.code_challenge, expected_challenge);
}

#[test]
fn test_auth_url_generation() {
    let flow = PkceAuthFlow::new().unwrap();
    let auth_url = flow.get_auth_url();

    // Verify URL contains required parameters
    assert!(auth_url.contains("callback="));
    assert!(auth_url.contains("code_challenge="));
    assert!(auth_url.starts_with(TETRATE_AUTH_URL));

    // Verify callback URL is properly encoded
    assert!(auth_url.contains(&*urlencoding::encode(CALLBACK_URL)));
}

#[test]
fn test_different_verifiers_produce_different_challenges() {
    let flow1 = PkceAuthFlow::new().unwrap();
    let flow2 = PkceAuthFlow::new().unwrap();

    // Verifiers should be different (extremely high probability)
    assert_ne!(flow1.code_verifier, flow2.code_verifier);

    // Challenges should also be different
    assert_ne!(flow1.code_challenge, flow2.code_challenge);
}

#[test]
fn test_configure_tetrate() {
    use crate::config::Config;
    use tempfile::TempDir;

    // Create a test config with temporary paths
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    let config = Config::new(&config_path, "test_service").unwrap();

    // Configure with a test API key
    let test_key = "test-api-key-123".to_string();
    configure_tetrate(&config, test_key.clone()).unwrap();

    // Verify the configuration was set correctly
    assert_eq!(
        config.get_secret::<String>("TETRATE_API_KEY").unwrap(),
        test_key
    );
    assert_eq!(config.get_goose_provider().unwrap(), "tetrate");
    assert_eq!(
        config.get_goose_model().unwrap(),
        TETRATE_DEFAULT_MODEL.to_string()
    );
}
