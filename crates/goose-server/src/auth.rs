use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use goose::identity::{ExecutionIdentity, UserIdentity};
use goose::oidc::OidcValidator;
use goose::session_token::SessionTokenStore;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// IP-based sliding window rate limiter for auth endpoints.
///
/// Tracks request timestamps per IP within a configurable window.
/// Returns 429 Too Many Requests when the limit is exceeded.
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<IpAddr, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check if a request from this IP is allowed. Returns true if under limit.
    pub async fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        let entries = requests.entry(ip).or_default();

        // Remove expired entries outside the window
        entries.retain(|&t| now.duration_since(t) < self.window);

        if entries.len() >= self.max_requests {
            return false;
        }

        entries.push(now);
        true
    }

    /// Periodic cleanup of stale entries (call from a background task).
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        requests.retain(|_, entries| {
            entries.retain(|&t| now.duration_since(t) < self.window);
            !entries.is_empty()
        });
    }

    #[cfg(test)]
    async fn tracked_ips(&self) -> usize {
        self.requests.read().await.len()
    }
}

/// Axum middleware that applies rate limiting based on client IP.
///
/// Extracts the client IP from `X-Forwarded-For`, `X-Real-IP`, or the
/// connection info, then checks the rate limiter.
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&request);

    if !limiter.check(ip).await {
        tracing::warn!(ip = %ip, "rate limit exceeded on auth endpoint");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Extract client IP from headers or connection info.
fn extract_client_ip(request: &Request) -> IpAddr {
    // Try X-Forwarded-For first (proxied requests)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first_ip) = val.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            if let Ok(ip) = val.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // Fallback to loopback
    IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
}

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
/// 3. `X-Goose-Api-Key: <key_id>` or `X-Api-Key: <key_id>` (legacy) → API key identity
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
    /// Also validates session tokens issued by `/auth/login`.
    /// Falls back to unvalidated extraction if validation fails or no providers configured.
    pub async fn from_headers_validated(
        headers: &HeaderMap,
        oidc: &OidcValidator,
        session_store: &SessionTokenStore,
    ) -> Self {
        if let Some(token) = extract_bearer_token(headers) {
            // Try session token first (issued by /auth/login)
            if let Ok(claims) = session_store.validate_token(&token).await {
                tracing::debug!(sub = %claims.sub, "Session token validated");
                return RequestIdentity {
                    user: claims.into_user_identity(),
                };
            }

            // Try OIDC validation
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
    // API key — prefer x-goose-api-key, fall back to x-api-key (legacy)
    if let Some(key) = headers
        .get("x-goose-api-key")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok())
    {
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

    // 2. API key — prefer x-goose-api-key, fall back to x-api-key (legacy)
    if let Some(key_id) = headers
        .get("x-goose-api-key")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok())
    {
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
    fn test_api_key_from_legacy_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "sk-live-abc123".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert!(!user.is_guest());
        assert_eq!(user.id, "apikey-sk-live-abc123");
    }

    #[test]
    fn test_goose_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-goose-api-key", "gk-my-key-456".parse().unwrap());
        let user = extract_user_from_headers(&headers);
        assert!(!user.is_guest());
        assert_eq!(user.id, "apikey-gk-my-key-456");
    }

    #[test]
    fn test_goose_api_key_takes_priority_over_legacy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-goose-api-key", "preferred-key".parse().unwrap());
        headers.insert("x-api-key", "legacy-key".parse().unwrap());
        let user = extract_non_bearer_identity(&headers);
        assert_eq!(user.id, "apikey-preferred-key");
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

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        for _ in 0..5 {
            assert!(limiter.check(ip).await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        // 4th request should be blocked
        assert!(!limiter.check(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_ips_independent() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip1: IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();

        assert!(limiter.check(ip1).await);
        assert!(limiter.check(ip1).await);
        assert!(!limiter.check(ip1).await); // ip1 blocked

        // ip2 should still be allowed
        assert!(limiter.check(ip2).await);
        assert!(limiter.check(ip2).await);
        assert!(!limiter.check(ip2).await); // ip2 now blocked
    }

    #[tokio::test]
    async fn test_rate_limiter_window_expiry() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await); // blocked

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be allowed again
        assert!(limiter.check(ip).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(10, Duration::from_millis(50));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert_eq!(limiter.tracked_ips().await, 1);

        // Wait for entries to expire, then cleanup
        tokio::time::sleep(Duration::from_millis(60)).await;
        limiter.cleanup().await;
        assert_eq!(limiter.tracked_ips().await, 0);
    }

    #[test]
    fn test_extract_client_ip_forwarded_for() {
        let request = Request::builder()
            .header("x-forwarded-for", "203.0.113.50, 70.41.3.18")
            .body(axum::body::Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "203.0.113.50".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_client_ip_real_ip() {
        let request = Request::builder()
            .header("x-real-ip", "198.51.100.23")
            .body(axum::body::Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "198.51.100.23".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_client_ip_fallback() {
        let request = Request::builder().body(axum::body::Body::empty()).unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn test_extract_client_ip_forwarded_for_priority() {
        let request = Request::builder()
            .header("x-forwarded-for", "203.0.113.50")
            .header("x-real-ip", "198.51.100.23")
            .body(axum::body::Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        // x-forwarded-for takes priority
        assert_eq!(ip, "203.0.113.50".parse::<IpAddr>().unwrap());
    }
}
