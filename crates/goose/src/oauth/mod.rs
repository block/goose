mod persist;

pub use persist::GooseCredentialStore;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use minijinja::render;
use oauth2::TokenResponse;
use rmcp::transport::auth::{
    AuthorizationRequest, CredentialStore, OAuthClientConfig, OAuthState, StoredCredentials,
};
use rmcp::transport::AuthorizationManager;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing::warn;

const CALLBACK_TEMPLATE: &str = include_str!("oauth_callback.html");
const CLIENT_METADATA_URL: &str = "https://goose-docs.ai/oauth/client-metadata.json";
const DEFAULT_OAUTH_CALLBACK_TIMEOUT_SECS: u64 = 300;
const OAUTH_CALLBACK_TIMEOUT_ENV: &str = "GOOSE_OAUTH_CALLBACK_TIMEOUT_SECONDS";

#[derive(Clone)]
struct AppState {
    code_receiver: Arc<Mutex<Option<oneshot::Sender<CallbackParams>>>>,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
    iss: Option<String>,
}

fn resolve_oauth_callback_timeout(value: Option<&str>) -> Duration {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_OAUTH_CALLBACK_TIMEOUT_SECS))
}

fn oauth_callback_timeout() -> Duration {
    let timeout = std::env::var(OAUTH_CALLBACK_TIMEOUT_ENV).ok();
    resolve_oauth_callback_timeout(timeout.as_deref())
}

fn announce_authorization_url(name: &str, authorization_url: &str) {
    warn!(
        "[OAuth:{}] If the browser did not open, authorize manually at: {}",
        name, authorization_url
    );
    eprintln!(
        "If the browser did not open, authorize {} at:\n  {}",
        name, authorization_url
    );
}

async fn wait_for_callback(
    code_receiver: oneshot::Receiver<CallbackParams>,
    timeout_duration: Duration,
    name: &str,
    authorization_url: &str,
) -> Result<CallbackParams, anyhow::Error> {
    match tokio::time::timeout(timeout_duration, code_receiver).await {
        Ok(Ok(params)) => Ok(params),
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "OAuth authorization for {} ended before the callback was received: {}",
            name,
            e
        )),
        Err(_) => {
            let message = format!(
                "OAuth authorization for {} timed out waiting for the local callback. \
                 Start the OAuth flow again and open this URL manually if the browser does not open: {}",
                name, authorization_url
            );
            warn!("[OAuth:{}] {}", name, message);
            Err(anyhow::anyhow!(message))
        }
    }
}

/// OAuth client credentials registered with the authorization server out of
/// band, for servers whose authorization server supports neither Dynamic
/// Client Registration nor Client ID Metadata Documents.
#[derive(Clone, Debug, PartialEq)]
pub struct StaticOAuthClientConfig {
    pub client_id: String,
    /// Secret paired with the client ID. Optional: public clients using PKCE
    /// have no secret.
    pub client_secret: Option<String>,
    /// Scopes to request. When empty, scopes are selected from server
    /// metadata, which may be broader than the extension needs.
    pub scopes: Vec<String>,
}

/// A stored grant satisfies the configured scopes when every requested scope
/// was granted. A broader stored grant is accepted: forcing re-authorization
/// until granted == requested would loop on servers that grant supersets or
/// let users decline individual scopes.
fn granted_scopes_cover_requested(granted: &[String], requested: &[String]) -> bool {
    requested
        .iter()
        .all(|scope| granted.iter().any(|granted_scope| granted_scope == scope))
}

/// RFC 6749 section 5.1 makes the token response's `scope` optional when the
/// granted scopes are identical to those requested, so an omitted scope list
/// means the requested scopes were granted.
fn resolve_granted_scopes(
    token_scopes: Option<Vec<String>>,
    static_client: Option<&StaticOAuthClientConfig>,
) -> Vec<String> {
    match token_scopes {
        Some(scopes) => scopes,
        None => static_client
            .map(|client| client.scopes.clone())
            .unwrap_or_default(),
    }
}

fn build_authorization_request(
    redirect_uri: String,
    static_client: Option<&StaticOAuthClientConfig>,
) -> AuthorizationRequest {
    let mut request = AuthorizationRequest::new(redirect_uri).with_client_name("goose");
    match static_client {
        Some(client) => {
            request = request.with_preregistered_client(client.client_id.clone());
            if let Some(secret) = &client.client_secret {
                request = request.with_client_secret(secret.clone());
            }
            if !client.scopes.is_empty() {
                request = request.with_scopes(client.scopes.clone());
            }
        }
        None => {
            request = request.with_client_metadata_url(CLIENT_METADATA_URL);
        }
    }
    request
}

pub async fn oauth_flow(
    mcp_server_url: &String,
    name: &String,
    static_client: Option<&StaticOAuthClientConfig>,
) -> Result<AuthorizationManager, anyhow::Error> {
    let credential_store = GooseCredentialStore::new(name.clone());
    let mut auth_manager = AuthorizationManager::new(mcp_server_url).await?;
    auth_manager.set_credential_store(credential_store.clone());

    if auth_manager.initialize_from_store().await? {
        // Refreshing keeps the stored grant's scopes, so a configured scope
        // the stored grant lacks can only be obtained by authorizing again.
        let scopes_satisfied = match static_client {
            Some(client) if !client.scopes.is_empty() => {
                let granted = credential_store
                    .load()
                    .await?
                    .map(|credentials| credentials.granted_scopes)
                    .unwrap_or_default();
                granted_scopes_cover_requested(&granted, &client.scopes)
            }
            _ => true,
        };

        if scopes_satisfied {
            // initialize_from_store configures the client from the stored
            // client_id alone; a confidential client must present its secret
            // at the token endpoint for the refresh to succeed.
            if let Some(client) = static_client {
                let mut config =
                    OAuthClientConfig::new(client.client_id.clone(), mcp_server_url.clone());
                if let Some(secret) = &client.client_secret {
                    config = config.with_client_secret(secret.clone());
                }
                auth_manager.configure_client(config)?;
            }
            match auth_manager.refresh_token().await {
                Ok(_) => {
                    return Ok(auth_manager);
                }
                Err(e) => {
                    warn!(
                        "[OAuth:{}] Token refresh failed: {} - clearing stored credentials and falling back to browser auth",
                        name, e
                    );
                }
            }
        } else {
            warn!(
                "[OAuth:{}] Stored grant is missing configured scopes - starting browser authorization to request them",
                name
            );
        }

        if let Err(e) = credential_store.clear().await {
            warn!("[OAuth:{}] error clearing bad credentials: {}", name, e);
        }
    }

    // No existing credentials or they were invalid - need to do the full oauth flow
    let (code_sender, code_receiver) = oneshot::channel::<CallbackParams>();
    let app_state = AppState {
        code_receiver: Arc::new(Mutex::new(Some(code_sender))),
    };

    let rendered = render!(CALLBACK_TEMPLATE, name => name);
    let handler = move |Query(params): Query<CallbackParams>, State(state): State<AppState>| {
        let rendered = rendered.clone();
        async move {
            if let Some(sender) = state.code_receiver.lock().await.take() {
                let _ = sender.send(params);
            }
            Html(rendered)
        }
    };
    let app = Router::new()
        .route("/oauth_callback", get(handler))
        .with_state(app_state);

    let port: u16 = std::env::var("GOOSE_OAUTH_CALLBACK_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let used_addr = listener.local_addr()?;
    let server_handle = tokio::spawn(async move {
        let result = axum::serve(listener, app).await;
        if let Err(e) = result {
            eprintln!("Callback server error: {}", e);
        }
    });

    let mut oauth_state = OAuthState::new(mcp_server_url, None).await?;

    let redirect_uri = format!("http://127.0.0.1:{}/oauth_callback", used_addr.port());
    oauth_state
        .start_authorization(build_authorization_request(redirect_uri, static_client))
        .await?;

    let authorization_url = oauth_state.get_authorization_url().await?;
    announce_authorization_url(name, authorization_url.as_str());
    if let Err(e) = webbrowser::open(authorization_url.as_str()) {
        warn!(
            "[OAuth:{}] Failed to open browser automatically: {}",
            name, e
        );
    }

    let callback_params = wait_for_callback(
        code_receiver,
        oauth_callback_timeout(),
        name,
        authorization_url.as_str(),
    )
    .await;
    server_handle.abort();
    let CallbackParams {
        code: auth_code,
        state: csrf_token,
        iss,
    } = callback_params?;
    oauth_state
        .handle_callback_with_issuer(&auth_code, &csrf_token, iss.as_deref())
        .await?;

    let (client_id, token_response) = oauth_state.get_credentials().await?;

    let mut auth_manager = oauth_state
        .into_authorization_manager()
        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;

    let granted_scopes = resolve_granted_scopes(
        token_response
            .as_ref()
            .and_then(|tr| tr.scopes())
            .map(|scopes| scopes.iter().map(|s| s.to_string()).collect()),
        static_client,
    );

    credential_store
        .save(StoredCredentials::new(
            client_id,
            token_response,
            granted_scopes,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0),
            ),
        ))
        .await?;

    auth_manager.set_credential_store(credential_store);

    Ok(auth_manager)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_oauth_callback_timeout_uses_default_for_missing_or_invalid_values() {
        assert_eq!(
            resolve_oauth_callback_timeout(None),
            Duration::from_secs(DEFAULT_OAUTH_CALLBACK_TIMEOUT_SECS)
        );
        assert_eq!(
            resolve_oauth_callback_timeout(Some("not-a-number")),
            Duration::from_secs(DEFAULT_OAUTH_CALLBACK_TIMEOUT_SECS)
        );
        assert_eq!(
            resolve_oauth_callback_timeout(Some("0")),
            Duration::from_secs(DEFAULT_OAUTH_CALLBACK_TIMEOUT_SECS)
        );
    }

    #[test]
    fn resolve_oauth_callback_timeout_uses_positive_values() {
        assert_eq!(
            resolve_oauth_callback_timeout(Some("42")),
            Duration::from_secs(42)
        );
    }

    #[tokio::test]
    async fn wait_for_callback_returns_received_callback_params() {
        let (sender, receiver) = oneshot::channel();
        sender
            .send(CallbackParams {
                code: "auth-code".to_string(),
                state: "csrf-state".to_string(),
                iss: Some("https://auth.example".to_string()),
            })
            .unwrap();

        let params = wait_for_callback(
            receiver,
            Duration::from_secs(1),
            "test-server",
            "https://auth.example/authorize",
        )
        .await
        .unwrap();

        assert_eq!(params.code, "auth-code");
        assert_eq!(params.state, "csrf-state");
        assert_eq!(params.iss.as_deref(), Some("https://auth.example"));
    }

    #[test]
    fn callback_params_capture_rfc_9207_issuer() {
        let uri: axum::http::Uri =
            "http://127.0.0.1/oauth_callback?code=auth-code&state=csrf-state&iss=https%3A%2F%2Fauth.example%2Fidp"
                .parse()
                .unwrap();

        let Query(params) = Query::<CallbackParams>::try_from_uri(&uri).unwrap();

        assert_eq!(params.iss.as_deref(), Some("https://auth.example/idp"));
    }

    #[test]
    fn callback_params_accept_missing_issuer() {
        let uri: axum::http::Uri =
            "http://127.0.0.1/oauth_callback?code=auth-code&state=csrf-state"
                .parse()
                .unwrap();

        let Query(params) = Query::<CallbackParams>::try_from_uri(&uri).unwrap();

        assert_eq!(params.iss, None);
    }

    #[test]
    fn resolve_granted_scopes_prefers_the_token_response_scopes() {
        let static_client = StaticOAuthClientConfig {
            client_id: "registered-client".to_string(),
            client_secret: None,
            scopes: vec!["scope.read".to_string()],
        };

        assert_eq!(
            resolve_granted_scopes(Some(vec!["scope.other".to_string()]), Some(&static_client)),
            vec!["scope.other"]
        );
    }

    #[test]
    fn resolve_granted_scopes_falls_back_to_requested_scopes_when_omitted() {
        let static_client = StaticOAuthClientConfig {
            client_id: "registered-client".to_string(),
            client_secret: None,
            scopes: vec!["scope.read".to_string(), "scope.write".to_string()],
        };

        assert_eq!(
            resolve_granted_scopes(None, Some(&static_client)),
            vec!["scope.read", "scope.write"]
        );
        assert!(resolve_granted_scopes(None, None).is_empty());
    }

    #[test]
    fn granted_scopes_cover_requested_accepts_exact_and_superset_grants() {
        let requested = vec!["scope.read".to_string()];

        assert!(granted_scopes_cover_requested(&requested, &requested));
        assert!(granted_scopes_cover_requested(
            &["scope.read".to_string(), "scope.write".to_string()],
            &requested
        ));
        assert!(granted_scopes_cover_requested(&[], &[]));
    }

    #[test]
    fn granted_scopes_cover_requested_rejects_missing_scopes() {
        assert!(!granted_scopes_cover_requested(
            &["scope.read".to_string()],
            &["scope.read".to_string(), "scope.write".to_string()]
        ));
        assert!(!granted_scopes_cover_requested(
            &[],
            &["scope.read".to_string()]
        ));
    }

    #[test]
    fn authorization_request_uses_client_metadata_url_without_static_client() {
        let request =
            build_authorization_request("http://127.0.0.1:1234/oauth_callback".to_string(), None);

        assert_eq!(request.client_id, None);
        assert_eq!(request.client_secret, None);
        assert_eq!(
            request.client_metadata_url.as_deref(),
            Some(CLIENT_METADATA_URL)
        );
        assert!(request.scopes.is_empty());
    }

    #[test]
    fn authorization_request_prefers_static_client_over_client_metadata_url() {
        let static_client = StaticOAuthClientConfig {
            client_id: "registered-client".to_string(),
            client_secret: Some("registered-secret".to_string()),
            scopes: vec!["scope.read".to_string(), "scope.write".to_string()],
        };

        let request = build_authorization_request(
            "http://127.0.0.1:1234/oauth_callback".to_string(),
            Some(&static_client),
        );

        assert_eq!(request.client_id.as_deref(), Some("registered-client"));
        assert_eq!(request.client_secret.as_deref(), Some("registered-secret"));
        assert_eq!(request.client_metadata_url, None);
        assert_eq!(request.scopes, vec!["scope.read", "scope.write"]);
    }

    #[test]
    fn authorization_request_omits_secret_and_scopes_for_public_static_client() {
        let static_client = StaticOAuthClientConfig {
            client_id: "registered-client".to_string(),
            client_secret: None,
            scopes: vec![],
        };

        let request = build_authorization_request(
            "http://127.0.0.1:1234/oauth_callback".to_string(),
            Some(&static_client),
        );

        assert_eq!(request.client_id.as_deref(), Some("registered-client"));
        assert_eq!(request.client_secret, None);
        assert_eq!(request.client_metadata_url, None);
        assert!(request.scopes.is_empty());
    }

    #[tokio::test]
    async fn wait_for_callback_times_out_with_authorization_url() {
        let (_sender, receiver) = oneshot::channel();

        let error = wait_for_callback(
            receiver,
            Duration::from_millis(1),
            "test-server",
            "https://auth.example/authorize",
        )
        .await
        .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("test-server"));
        assert!(message.contains("timed out"));
        assert!(message.contains("https://auth.example/authorize"));
    }
}
