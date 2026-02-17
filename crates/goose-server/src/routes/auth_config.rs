use crate::state::AppState;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use goose::oidc::OidcProviderConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OidcProvidersResponse {
    pub providers: Vec<OidcProviderInfo>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OidcProviderInfo {
    pub issuer: String,
    pub audience: String,
    pub tenant_claim: Option<String>,
    pub group_claim: Option<String>,
    pub required_groups: Vec<String>,
}

impl From<&OidcProviderConfig> for OidcProviderInfo {
    fn from(p: &OidcProviderConfig) -> Self {
        Self {
            issuer: p.issuer.clone(),
            audience: p.audience.clone(),
            tenant_claim: p.tenant_claim.clone(),
            group_claim: p.group_claim.clone(),
            required_groups: p.required_groups.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AddOidcProviderRequest {
    pub issuer: String,
    pub audience: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub tenant_claim: Option<String>,
    #[serde(default)]
    pub group_claim: Option<String>,
    #[serde(default)]
    pub required_groups: Vec<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct RemoveOidcProviderRequest {
    pub issuer: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthStatusResponse {
    pub oidc_enabled: bool,
    pub provider_count: usize,
    pub issuers: Vec<String>,
}

/// List configured OIDC providers.
#[utoipa::path(
    get,
    path = "/auth/oidc/providers",
    responses(
        (status = 200, description = "List of OIDC providers", body = OidcProvidersResponse)
    )
)]
pub async fn list_oidc_providers(
    State(state): State<Arc<AppState>>,
) -> Json<OidcProvidersResponse> {
    let providers = state.oidc_validator.list_providers().await;
    let infos = providers.iter().map(OidcProviderInfo::from).collect();
    Json(OidcProvidersResponse { providers: infos })
}

/// Add an OIDC provider for JWT validation.
#[utoipa::path(
    post,
    path = "/auth/oidc/providers",
    request_body = AddOidcProviderRequest,
    responses(
        (status = 200, description = "Provider added", body = OidcProvidersResponse)
    )
)]
pub async fn add_oidc_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddOidcProviderRequest>,
) -> Json<OidcProvidersResponse> {
    let config = OidcProviderConfig {
        issuer: req.issuer.clone(),
        audience: req.audience,
        client_secret: req.client_secret,
        tenant_claim: req.tenant_claim,
        group_claim: req.group_claim,
        required_groups: req.required_groups,
    };

    state.oidc_validator.add_provider(config).await;

    tracing::info!(issuer = %req.issuer, "OIDC provider added");

    let providers = state.oidc_validator.list_providers().await;
    let infos = providers.iter().map(OidcProviderInfo::from).collect();
    Json(OidcProvidersResponse { providers: infos })
}

/// Remove an OIDC provider by issuer URL.
#[utoipa::path(
    delete,
    path = "/auth/oidc/providers",
    request_body = RemoveOidcProviderRequest,
    responses(
        (status = 200, description = "Provider removed", body = OidcProvidersResponse)
    )
)]
pub async fn remove_oidc_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RemoveOidcProviderRequest>,
) -> Json<OidcProvidersResponse> {
    state.oidc_validator.remove_provider(&req.issuer).await;

    tracing::info!(issuer = %req.issuer, "OIDC provider removed");

    let providers = state.oidc_validator.list_providers().await;
    let infos = providers.iter().map(OidcProviderInfo::from).collect();
    Json(OidcProvidersResponse { providers: infos })
}

/// Get auth system status.
#[utoipa::path(
    get,
    path = "/auth/status",
    responses(
        (status = 200, description = "Auth status", body = AuthStatusResponse)
    )
)]
pub async fn auth_status(State(state): State<Arc<AppState>>) -> Json<AuthStatusResponse> {
    let providers = state.oidc_validator.list_providers().await;
    Json(AuthStatusResponse {
        oidc_enabled: !providers.is_empty(),
        provider_count: providers.len(),
        issuers: providers.iter().map(|p| p.issuer.clone()).collect(),
    })
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/oidc/providers", get(list_oidc_providers))
        .route("/auth/oidc/providers", post(add_oidc_provider))
        .route("/auth/oidc/providers", delete(remove_oidc_provider))
        .route("/auth/status", get(auth_status))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> Arc<AppState> {
        AppState::new().await.unwrap()
    }

    #[tokio::test]
    async fn test_auth_status_empty() {
        let state = test_state().await;
        let app = routes(state);

        let req = Request::builder()
            .uri("/auth/status")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: AuthStatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(!status.oidc_enabled);
        assert_eq!(status.provider_count, 0);
    }

    #[tokio::test]
    async fn test_list_providers_empty() {
        let state = test_state().await;
        let app = routes(state);

        let req = Request::builder()
            .uri("/auth/oidc/providers")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: OidcProvidersResponse = serde_json::from_slice(&body).unwrap();
        assert!(list.providers.is_empty());
    }

    #[tokio::test]
    async fn test_add_and_list_provider() {
        let state = test_state().await;
        let app = routes(state.clone());

        let add_req = AddOidcProviderRequest {
            issuer: "https://accounts.google.com".to_string(),
            audience: "my-app".to_string(),
            client_secret: None,
            tenant_claim: Some("tid".to_string()),
            group_claim: None,
            required_groups: vec![],
        };

        let req = Request::builder()
            .uri("/auth/oidc/providers")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&add_req).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: OidcProvidersResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.providers.len(), 1);
        assert_eq!(list.providers[0].issuer, "https://accounts.google.com");
        assert_eq!(list.providers[0].audience, "my-app");
        assert_eq!(list.providers[0].tenant_claim, Some("tid".to_string()));
    }

    #[tokio::test]
    async fn test_add_and_remove_provider() {
        let state = test_state().await;

        state
            .oidc_validator
            .add_provider(OidcProviderConfig {
                issuer: "https://accounts.google.com".into(),
                audience: "test-app".into(),
                client_secret: None,
                tenant_claim: None,
                group_claim: None,
                required_groups: vec![],
            })
            .await;
        assert_eq!(state.oidc_validator.list_providers().await.len(), 1);

        let app = routes(state.clone());

        let remove_req = RemoveOidcProviderRequest {
            issuer: "https://accounts.google.com".to_string(),
        };

        let req = Request::builder()
            .uri("/auth/oidc/providers")
            .method("DELETE")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&remove_req).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: OidcProvidersResponse = serde_json::from_slice(&body).unwrap();
        assert!(list.providers.is_empty());
    }
}
