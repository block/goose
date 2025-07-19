#[cfg(test)]
mod tests {
    use crate::config::signup_openrouter::PkceAuthFlow;

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
}
