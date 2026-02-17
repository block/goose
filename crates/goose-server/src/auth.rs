use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use goose::identity::{ExecutionIdentity, UserIdentity};
use goose::oidc::OidcValidator;

/// Existing secret-key middleware — unchanged.
#[allow(dead_code)]
pub async fn check_token(
    State(state): State<String>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.uri().path() == "/status"
        || request.uri().path() == "/mcp-ui-proxy"
        || request.uri().path() == "/mcp-app-proxy"
    {
        return Ok(next.run(request).await);
    }
    let secret_key = request
        .headers()
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok());

    match secret_key {
        Some(key) if key == state => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Identity extracted from HTTP request headers.
///
/// Extraction priority:
/// 1. `Authorization: Bearer <jwt>` → OIDC-validated identity (if providers configured)
/// 2. `Authorization: Bearer <jwt>` → unvalidated JWT decode (fallback when no OIDC)
/// 3. `X-Api-Key: <key_id>` → API key identity
/// 4. `X-Goose-User-Id: <id>` → stable guest identity (for desktop app)
/// 5. Fallback → anonymous guest with random UUID
///
/// This never rejects — it always produces an identity.
/// Auth *enforcement* is handled by `check_token` middleware.
#[derive(Debug, Clone)]
pub struct RequestIdentity {
    pub user: UserIdentity,
}

impl RequestIdentity {
    /// Build an `ExecutionIdentity` by combining the extracted user with an agent.
    pub fn into_execution(self, agent_kind: &str, agent_persona: &str) -> ExecutionIdentity {
        let agent = goose::identity::AgentIdentity::new(agent_kind, agent_persona, &self.user.id);
        ExecutionIdentity::new(self.user, agent)
    }

    /// Extract identity from HTTP headers without OIDC validation.
    /// Uses lightweight JWT claim extraction (no signature check).
    #[allow(dead_code)]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        RequestIdentity {
            user: extract_user_from_headers(headers),
        }
    }

    /// Extract identity from HTTP headers with full OIDC validation when providers are configured.
    /// Falls back to unvalidated extraction if OIDC validation fails or no providers configured.
    pub async fn from_headers_validated(headers: &HeaderMap, oidc: &OidcValidator) -> Self {
        if let Some(token) = extract_bearer_token(headers) {
            if oidc.has_providers().await {
                match oidc.validate_token(&token).await {
                    Ok(claims) => {
                        tracing::debug!(
                            subject = %claims.subject,
                            issuer = %claims.issuer,
                            "OIDC token validated"
                        );
                        return RequestIdentity {
                            user: claims.into_user_identity(),
                        };
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "OIDC validation failed, falling back");
                    }
                }
            }
            // Fallback: lightweight JWT decode without signature verification
            if let Some(user) = UserIdentity::from_bearer_token(&token) {
                return RequestIdentity { user };
            }
        }

        // Non-Bearer fallbacks
        RequestIdentity {
            user: extract_non_bearer_identity(headers),
        }
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(|s| s.to_string())
}

fn extract_non_bearer_identity(headers: &HeaderMap) -> UserIdentity {
    // API key
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return UserIdentity::from_api_key(key);
    }
    // Stable guest ID (desktop app)
    if let Some(id) = headers.get("x-goose-user-id").and_then(|v| v.to_str().ok()) {
        return UserIdentity::guest_stable(id);
    }
    // Anonymous guest
    UserIdentity::guest()
}

/// Extract user identity from HTTP headers with fallback chain.
fn extract_user_from_headers(headers: &HeaderMap) -> UserIdentity {
    // 1. Bearer token → JWT decode → OIDC
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if let Some(user) = UserIdentity::from_bearer_token(token) {
                return user;
            }
        }
    }

    // 2. API key
    if let Some(key_id) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return UserIdentity::from_api_key(key_id);
    }

    // 3. Stable guest ID from desktop app
    if let Some(user_id) = headers.get("x-goose-user-id").and_then(|v| v.to_str().ok()) {
        return UserIdentity::guest_stable(user_id);
    }

    // 4. Anonymous guest
    UserIdentity::guest()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_fallback_guest() {
        let headers = HeaderMap::new();
        let user = extract_user_from_headers(&headers);
        assert!(user.is_guest());
        assert!(user.id.starts_with("guest-"));
    }

    #[test]
    fn test_stable_guest_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-goose-user-id", "desktop-user-42".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert!(user.is_guest());
        assert_eq!(user.id, "desktop-user-42");
    }

    #[test]
    fn test_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "sk-live-abc123".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert!(!user.is_guest());
        assert_eq!(user.id, "apikey-sk-live-abc123");
    }

    #[test]
    fn test_bearer_token_from_header() {
        use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

        let payload = serde_json::json!({
            "sub": "user-789",
            "name": "Test User",
            "iss": "https://accounts.google.com"
        });
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("{}.{}.fake-sig", header, body);

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let user = extract_user_from_headers(&headers);
        assert!(!user.is_guest());
        assert_eq!(user.id, "oidc-google-user-789");
        assert_eq!(user.name, "Test User");
    }

    #[test]
    fn test_bearer_takes_priority_over_api_key() {
        use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

        let payload = serde_json::json!({
            "sub": "oidc-sub",
            "name": "OIDC User",
            "iss": "https://accounts.google.com"
        });
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("{}.{}.fake-sig", header, body);

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        headers.insert("x-api-key", "should-be-ignored".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert_eq!(user.id, "oidc-google-oidc-sub");
    }

    #[test]
    fn test_malformed_bearer_falls_through() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer not-a-jwt".parse().unwrap());
        headers.insert("x-api-key", "fallback-key".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert_eq!(user.id, "apikey-fallback-key");
    }

    #[test]
    fn test_into_execution() {
        let identity = RequestIdentity {
            user: UserIdentity::guest_stable("user-1"),
        };
        let exec = identity.into_execution("developer", "Developer Agent");
        assert_eq!(exec.user.id, "user-1");
        assert_eq!(exec.agent.kind, "developer");
        assert_eq!(exec.agent.persona, "Developer Agent");
        assert_eq!(exec.agent.spawned_by, "user-1");
    }

    #[test]
    fn test_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-goose-user-id", "desktop-42".parse().unwrap());
        let req_id = RequestIdentity::from_headers(&headers);
        assert_eq!(req_id.user.id, "desktop-42");
        assert!(req_id.user.is_guest());
    }
}
