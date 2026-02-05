//! Authentication handling for the connector proxy.
//!
//! Supports two modes:
//!
//! 1. **Packed headers (Fabrix mode)**: API key contains 3 parts separated by `<|>`:
//!    - Format: `{client_id}<|>{token}<|>{user_id}`
//!    - Converted to 3 HTTP headers: x-generative-ai-client, x-openapi-token, x-client-user
//!
//! 2. **Simple API key (OpenAI mode)**: Single API key value
//!    - Sent as `Authorization: Bearer {api_key}` header

const DELIMITER: &str = "<|>";

/// Authentication credentials that can be either packed (Fabrix) or simple (OpenAI).
#[derive(Debug, Clone, PartialEq)]
pub enum Auth {
    /// Packed authentication with 3 separate credentials
    Packed(PackedAuth),
    /// Simple API key authentication
    Simple(SimpleAuth),
}

impl Auth {
    /// Parse an API key string, auto-detecting the format.
    ///
    /// If the key contains `<|>` delimiter, it's treated as packed format.
    /// Otherwise, it's treated as a simple API key.
    pub fn from_api_key(key: &str) -> anyhow::Result<Self> {
        if is_packed_key(key) {
            Ok(Auth::Packed(PackedAuth::from_api_key(key)?))
        } else {
            Ok(Auth::Simple(SimpleAuth::from_api_key(key)))
        }
    }

    /// Convert to HTTP headers for the backend LLM.
    pub fn to_headers(&self) -> Vec<(String, String)> {
        match self {
            Auth::Packed(packed) => packed.to_headers(),
            Auth::Simple(simple) => simple.to_headers(),
        }
    }
}

/// Parsed packed authentication credentials (Fabrix mode).
#[derive(Debug, Clone, PartialEq)]
pub struct PackedAuth {
    pub client_id: String,
    pub token: String,
    pub user_id: String,
}

impl PackedAuth {
    /// Parse a packed API key string into its component parts.
    ///
    /// The key format is: `{client_id}<|>{token}<|>{user_id}`
    ///
    /// Returns an error if the key does not contain exactly 3 parts.
    pub fn from_api_key(key: &str) -> anyhow::Result<Self> {
        let raw = key.strip_prefix("Bearer ").unwrap_or(key);
        let parts: Vec<&str> = raw.split(DELIMITER).collect();
        if parts.len() != 3 {
            anyhow::bail!(
                "Invalid packed api_key format. Expected 3 parts separated by '{}', got {}. \
                 Format: client_id{}token{}user_id",
                DELIMITER,
                parts.len(),
                DELIMITER,
                DELIMITER
            );
        }
        Ok(Self {
            client_id: parts[0].to_string(),
            token: parts[1].to_string(),
            user_id: parts[2].to_string(),
        })
    }

    /// Convert to a map of HTTP header name-value pairs for the custom LLM.
    pub fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("x-generative-ai-client".to_string(), self.client_id.clone()),
            (
                "x-openapi-token".to_string(),
                format!("Bearer {}", self.token),
            ),
            ("x-client-user".to_string(), self.user_id.clone()),
        ]
    }
}

/// Simple API key authentication (OpenAI mode).
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleAuth {
    pub api_key: String,
}

impl SimpleAuth {
    /// Create from a simple API key string.
    pub fn from_api_key(key: &str) -> Self {
        let raw = key.strip_prefix("Bearer ").unwrap_or(key);
        Self {
            api_key: raw.to_string(),
        }
    }

    /// Convert to HTTP headers (Authorization: Bearer).
    pub fn to_headers(&self) -> Vec<(String, String)> {
        vec![(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        )]
    }
}

/// Check whether a string contains the packed key delimiter.
pub fn is_packed_key(s: &str) -> bool {
    s.contains(DELIMITER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_auth_valid() {
        let auth = PackedAuth::from_api_key("client123<|>jwt-token-here<|>user456").unwrap();
        assert_eq!(auth.client_id, "client123");
        assert_eq!(auth.token, "jwt-token-here");
        assert_eq!(auth.user_id, "user456");
    }

    #[test]
    fn test_packed_auth_with_bearer_prefix() {
        let auth =
            PackedAuth::from_api_key("Bearer client123<|>jwt-token-here<|>user456").unwrap();
        assert_eq!(auth.client_id, "client123");
        assert_eq!(auth.token, "jwt-token-here");
        assert_eq!(auth.user_id, "user456");
    }

    #[test]
    fn test_packed_auth_two_parts() {
        let result = PackedAuth::from_api_key("only-two<|>parts");
        assert!(result.is_err());
    }

    #[test]
    fn test_packed_auth_to_headers() {
        let auth = PackedAuth {
            client_id: "connector".to_string(),
            token: "my-secret-token".to_string(),
            user_id: "user@example.com".to_string(),
        };
        let headers = auth.to_headers();
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0], ("x-generative-ai-client".to_string(), "connector".to_string()));
        assert_eq!(
            headers[1],
            ("x-openapi-token".to_string(), "Bearer my-secret-token".to_string())
        );
        assert_eq!(headers[2], ("x-client-user".to_string(), "user@example.com".to_string()));
    }

    #[test]
    fn test_simple_auth() {
        let auth = SimpleAuth::from_api_key("sk-12345");
        assert_eq!(auth.api_key, "sk-12345");
    }

    #[test]
    fn test_simple_auth_with_bearer() {
        let auth = SimpleAuth::from_api_key("Bearer sk-12345");
        assert_eq!(auth.api_key, "sk-12345");
    }

    #[test]
    fn test_simple_auth_to_headers() {
        let auth = SimpleAuth {
            api_key: "sk-12345".to_string(),
        };
        let headers = auth.to_headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0], ("Authorization".to_string(), "Bearer sk-12345".to_string()));
    }

    #[test]
    fn test_auth_auto_detect_packed() {
        let auth = Auth::from_api_key("client<|>token<|>user").unwrap();
        assert!(matches!(auth, Auth::Packed(_)));
    }

    #[test]
    fn test_auth_auto_detect_simple() {
        let auth = Auth::from_api_key("sk-12345").unwrap();
        assert!(matches!(auth, Auth::Simple(_)));
    }

    #[test]
    fn test_is_packed_key() {
        assert!(is_packed_key("a<|>b<|>c"));
        assert!(is_packed_key("Bearer a<|>b<|>c"));
        assert!(!is_packed_key("Bearer sk-12345"));
        assert!(!is_packed_key("plain-key"));
    }
}
