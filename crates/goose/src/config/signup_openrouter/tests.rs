use crate::config::signup_openrouter::PkceAuthFlow;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

#[test]
fn test_pkce_flow_creation() {
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    // Verify code_verifier is 128 characters
    assert_eq!(flow.code_verifier.len(), 128);

    // Verify code_challenge is base64url encoded (no padding)
    assert!(!flow.code_challenge.contains('='));
    assert!(!flow.code_challenge.contains('+'));
    assert!(!flow.code_challenge.contains('/'));

    // Verify auth URL is properly formatted
    let auth_url = flow.get_auth_url();
    assert!(auth_url.starts_with("https://openrouter.ai/auth"));
    assert!(auth_url.contains("callback_url=http%3A%2F%2Flocalhost%3A3000"));
    assert!(auth_url.contains(&format!("code_challenge={}", flow.code_challenge)));
    assert!(auth_url.contains("code_challenge_method=S256"));
}

#[test]
fn test_different_flows_have_different_verifiers() {
    let flow1 = PkceAuthFlow::new().expect("Failed to create PKCE flow 1");
    let flow2 = PkceAuthFlow::new().expect("Failed to create PKCE flow 2");

    // Verify that different flows have different verifiers and challenges
    assert_ne!(flow1.code_verifier, flow2.code_verifier);
    assert_ne!(flow1.code_challenge, flow2.code_challenge);
}

#[test]
fn test_code_verifier_is_alphanumeric() {
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    // Verify all characters in code_verifier are alphanumeric
    assert!(flow.code_verifier.chars().all(|c| c.is_alphanumeric()));
}

#[test]
fn test_code_challenge_matches_verifier() {
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    // Manually compute the expected challenge
    let mut hasher = Sha256::new();
    hasher.update(&flow.code_verifier);
    let hash = hasher.finalize();
    let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

    // Verify the challenge matches
    assert_eq!(flow.code_challenge, expected_challenge);
}

#[test]
fn test_pkce_verifier_length_bounds() {
    // PKCE spec requires verifier to be 43-128 characters
    // Our implementation uses 128 characters
    let flow = PkceAuthFlow::new().expect("Failed to create PKCE flow");

    assert!(flow.code_verifier.len() >= 43);
    assert!(flow.code_verifier.len() <= 128);
}
