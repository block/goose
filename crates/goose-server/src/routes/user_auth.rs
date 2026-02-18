use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::auth::RequestIdentity;
use crate::state::AppState;
use goose::identity::{AuthMethod, UserIdentity};

// ── Response types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserInfoResponse {
    pub id: String,
    pub name: String,
    pub auth_method: String,
    pub tenant: Option<String>,
    pub is_guest: bool,
    pub is_authenticated: bool,
}

impl From<&UserIdentity> for UserInfoResponse {
    fn from(user: &UserIdentity) -> Self {
        let auth_method = match &user.auth_method {
            AuthMethod::Guest => "guest".to_string(),
            AuthMethod::Oidc { provider, .. } => format!("oidc:{provider}"),
            AuthMethod::ApiKey => "api_key".to_string(),
            AuthMethod::ServiceAccount { .. } => "service_account".to_string(),
            AuthMethod::Password => "password".to_string(),
        };
        Self {
            id: user.id.clone(),
            name: user.name.clone(),
            auth_method,
            tenant: user.tenant.clone(),
            is_guest: user.is_guest(),
            is_authenticated: !user.is_guest(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Display name (optional, for guest upgrade)
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfoResponse,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OidcLoginRequest {
    /// The raw ID token from the OIDC provider (obtained via authorization code flow)
    pub id_token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OidcLoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfoResponse,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OidcAuthUrlRequest {
    /// OIDC provider issuer (e.g. "https://accounts.google.com")
    pub issuer: String,
    /// Redirect URI for the callback (e.g. "http://localhost:PORT/callback")
    pub redirect_uri: String,
    /// Optional scopes (defaults to "openid profile email")
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OidcAuthUrlResponse {
    /// The full authorization URL to redirect the user to
    pub auth_url: String,
    /// The state parameter (for CSRF protection)
    pub state: String,
    /// The nonce parameter (for replay protection)
    pub nonce: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OidcCodeExchangeRequest {
    /// OIDC provider issuer (e.g. "https://accounts.google.com")
    pub issuer: String,
    /// The authorization code from the OIDC callback
    pub code: String,
    /// The redirect URI used in the original authorization request
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OidcCodeExchangeResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfoResponse,
}

// ── Route handlers ──────────────────────────────────────────────────────

/// Get current user info from request headers
#[utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "Current user info", body = UserInfoResponse)
    ),
    tag = "auth"
)]
pub async fn get_user_info(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<UserInfoResponse> {
    let identity = RequestIdentity::from_headers_validated(
        &headers,
        &state.oidc_validator,
        &state.session_token_store,
    )
    .await;
    Json(UserInfoResponse::from(&identity.user))
}

/// Login with API key and receive a session token
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = if let Some(api_key) = &request.api_key {
        UserIdentity::from_api_key(api_key)
    } else {
        // Check existing auth headers first
        let identity = RequestIdentity::from_headers_validated(
            &headers,
            &state.oidc_validator,
            &state.session_token_store,
        )
        .await;
        if !identity.user.is_guest() {
            identity.user
        } else if let Some(name) = &request.display_name {
            // Guest with a display name — create a named guest
            UserIdentity::guest_stable(format!("named-{name}"))
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let token = state.session_token_store.issue_token(&user).map_err(|e| {
        tracing::error!("Failed to issue session token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ttl = state.session_token_store.ttl();
    Ok(Json(LoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: ttl.as_secs(),
        user: UserInfoResponse::from(&user),
    }))
}

/// Logout — revoke the current session token
#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse)
    ),
    tag = "auth"
)]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<LogoutResponse> {
    // Extract bearer token from Authorization header
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                state.session_token_store.revoke_by_token(token).await.ok();
                return Json(LogoutResponse { success: true });
            }
        }
    }
    Json(LogoutResponse { success: false })
}

/// Refresh an existing session token
#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshResponse),
        (status = 401, description = "Invalid or expired token")
    ),
    tag = "auth"
)]
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, StatusCode> {
    // Validate the existing token
    let claims = state
        .session_token_store
        .validate_token(&request.token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Revoke the old token
    state
        .session_token_store
        .revoke_by_token(&request.token)
        .await
        .ok();

    // Issue a new one with the same user
    let user = claims.into_user_identity();
    let new_token = state.session_token_store.issue_token(&user).map_err(|e| {
        tracing::error!("Failed to issue refresh token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ttl = state.session_token_store.ttl();
    Ok(Json(RefreshResponse {
        token: new_token,
        token_type: "Bearer".to_string(),
        expires_in: ttl.as_secs(),
    }))
}

/// Generate an OIDC authorization URL for starting the login flow.
/// The CLI or frontend calls this to get a URL to open in the browser.
#[utoipa::path(
    post,
    path = "/auth/login/oidc/url",
    request_body = OidcAuthUrlRequest,
    responses(
        (status = 200, description = "Authorization URL generated", body = OidcAuthUrlResponse),
        (status = 400, description = "Unknown OIDC provider")
    ),
    tag = "auth"
)]
pub async fn oidc_auth_url(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OidcAuthUrlRequest>,
) -> Result<Json<OidcAuthUrlResponse>, StatusCode> {
    let providers = state.oidc_validator.list_providers().await;
    let provider = providers
        .iter()
        .find(|p| p.issuer == request.issuer)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let client_id = provider.audience.clone();

    let (auth_endpoint, _token_endpoint) = state
        .oidc_validator
        .discover_endpoints(&request.issuer)
        .await
        .map_err(|e| {
            tracing::error!("OIDC discovery failed for {}: {}", request.issuer, e);
            StatusCode::BAD_GATEWAY
        })?;

    let auth_endpoint = auth_endpoint.ok_or_else(|| {
        tracing::error!("No authorization_endpoint for {}", request.issuer);
        StatusCode::BAD_GATEWAY
    })?;

    let state_param = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let scopes = request
        .scopes
        .unwrap_or_else(|| vec!["openid".into(), "profile".into(), "email".into()]);

    let scope_str = scopes.join(" ");
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
        auth_endpoint,
        urlencoding::encode(&client_id),
        urlencoding::encode(&request.redirect_uri),
        urlencoding::encode(&scope_str),
        urlencoding::encode(&state_param),
        urlencoding::encode(&nonce),
    );

    Ok(Json(OidcAuthUrlResponse {
        auth_url,
        state: state_param,
        nonce,
    }))
}

/// Login with an OIDC ID token (obtained after the authorization code exchange).
/// The CLI exchanges the auth code for tokens client-side, then sends the ID token here.
#[utoipa::path(
    post,
    path = "/auth/login/oidc",
    request_body = OidcLoginRequest,
    responses(
        (status = 200, description = "OIDC login successful", body = OidcLoginResponse),
        (status = 401, description = "Invalid or unverifiable ID token")
    ),
    tag = "auth"
)]
pub async fn oidc_login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OidcLoginRequest>,
) -> Result<Json<OidcLoginResponse>, StatusCode> {
    // Validate the ID token using our OIDC validator
    let claims = state
        .oidc_validator
        .validate_token(&request.id_token)
        .await
        .map_err(|e| {
            tracing::warn!("OIDC login failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    let user = claims.into_user_identity();

    let token = state.session_token_store.issue_token(&user).map_err(|e| {
        tracing::error!("Failed to issue session token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ttl = state.session_token_store.ttl();
    Ok(Json(OidcLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: ttl.as_secs(),
        user: UserInfoResponse::from(&user),
    }))
}

// ── Router ──────────────────────────────────────────────────────────────

/// Exchange an OIDC authorization code for a session token.
/// The server handles the code-to-token exchange with the OIDC provider,
/// validates the resulting ID token, and returns a goose session JWT.
#[utoipa::path(
    post,
    path = "/auth/login/oidc/code",
    request_body = OidcCodeExchangeRequest,
    responses(
        (status = 200, description = "Code exchange successful", body = OidcCodeExchangeResponse),
        (status = 400, description = "Unknown OIDC provider"),
        (status = 401, description = "Token exchange or validation failed"),
        (status = 502, description = "Failed to communicate with OIDC provider")
    ),
    tag = "auth"
)]
pub async fn oidc_code_exchange(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OidcCodeExchangeRequest>,
) -> Result<Json<OidcCodeExchangeResponse>, StatusCode> {
    // Exchange the authorization code for tokens via the OIDC provider
    let (id_token, _access_token) = state
        .oidc_validator
        .exchange_code(&request.issuer, &request.code, &request.redirect_uri)
        .await
        .map_err(|e| {
            tracing::warn!("OIDC code exchange failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    // Validate the ID token
    let claims = state
        .oidc_validator
        .validate_token(&id_token)
        .await
        .map_err(|e| {
            tracing::warn!("OIDC token validation after code exchange failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    let user = claims.into_user_identity();

    let token = state.session_token_store.issue_token(&user).map_err(|e| {
        tracing::error!("Failed to issue session token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ttl = state.session_token_store.ttl();
    Ok(Json(OidcCodeExchangeResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: ttl.as_secs(),
        user: UserInfoResponse::from(&user),
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/me", get(get_user_info))
        .route("/auth/login", post(login))
        .route("/auth/login/oidc", post(oidc_login))
        .route("/auth/login/oidc/url", post(oidc_auth_url))
        .route("/auth/login/oidc/code", post(oidc_code_exchange))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh_token))
        .with_state(state)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn test_state() -> Arc<AppState> {
        AppState::new().await.unwrap()
    }

    #[tokio::test]
    async fn test_get_user_info_guest() {
        let state = test_state().await;
        let app = routes(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let info: UserInfoResponse = serde_json::from_slice(&body).unwrap();
        assert!(info.is_guest);
        assert!(!info.is_authenticated);
        assert_eq!(info.auth_method, "guest");
    }

    #[tokio::test]
    async fn test_get_user_info_api_key() {
        let state = test_state().await;
        let app = routes(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("x-api-key", "my-test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let info: UserInfoResponse = serde_json::from_slice(&body).unwrap();
        assert!(!info.is_guest);
        assert!(info.is_authenticated);
        assert_eq!(info.auth_method, "api_key");
    }

    #[tokio::test]
    async fn test_login_with_api_key() {
        let state = test_state().await;
        let app = routes(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&LoginRequest {
                            api_key: Some("test-api-key-123".to_string()),
                            display_name: None,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let login_resp: LoginResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(login_resp.token_type, "Bearer");
        assert!(!login_resp.token.is_empty());
        assert!(!login_resp.user.is_guest);
        assert_eq!(login_resp.user.auth_method, "api_key");
    }

    #[tokio::test]
    async fn test_login_no_credentials_rejected() {
        let state = test_state().await;
        let app = routes(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&LoginRequest {
                            api_key: None,
                            display_name: None,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_and_logout() {
        let state = test_state().await;

        // Login first
        let login_app = routes(state.clone());
        let resp = login_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&LoginRequest {
                            api_key: Some("test-key".to_string()),
                            display_name: None,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let login_resp: LoginResponse = serde_json::from_slice(&body).unwrap();
        let token = login_resp.token;

        // Logout with that token
        let logout_app = routes(state.clone());
        let resp = logout_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let logout_resp: LogoutResponse = serde_json::from_slice(&body).unwrap();
        assert!(logout_resp.success);
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let state = test_state().await;

        // Login first
        let login_app = routes(state.clone());
        let resp = login_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&LoginRequest {
                            api_key: Some("refresh-test-key".to_string()),
                            display_name: None,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let login_resp: LoginResponse = serde_json::from_slice(&body).unwrap();
        let old_token = login_resp.token;

        // Refresh
        let refresh_app = routes(state.clone());
        let resp = refresh_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&RefreshRequest {
                            token: old_token.clone(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let refresh_resp: RefreshResponse = serde_json::from_slice(&body).unwrap();
        assert_ne!(refresh_resp.token, old_token);
        assert_eq!(refresh_resp.token_type, "Bearer");
    }
}
