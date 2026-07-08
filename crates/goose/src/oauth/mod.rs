mod persist;

pub use persist::GooseCredentialStore;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use minijinja::render;
use oauth2::TokenResponse;
use rmcp::transport::auth::{
    AuthorizationManager, CredentialStore, OAuthClientConfig, OAuthState, StoredCredentials,
};
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

#[derive(Clone, Debug)]
pub struct StaticOAuthClientConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

fn static_client_config(
    client: &StaticOAuthClientConfig,
    redirect_uri: &str,
    scopes: Vec<String>,
) -> OAuthClientConfig {
    let mut config = OAuthClientConfig::new(client.client_id.clone(), redirect_uri.to_string())
        .with_scopes(scopes);
    if let Some(secret) = &client.client_secret {
        config = config.with_client_secret(secret.clone());
    }
    config
}

pub async fn oauth_flow(
    mcp_server_url: &String,
    name: &String,
    static_client: Option<StaticOAuthClientConfig>,
) -> Result<AuthorizationManager, anyhow::Error> {
    let credential_store = GooseCredentialStore::new(name.clone());
    let mut auth_manager = AuthorizationManager::new(mcp_server_url).await?;
    auth_manager.set_credential_store(credential_store.clone());

    if let Some(client) = &static_client {
        let metadata = auth_manager.discover_metadata().await?;
        auth_manager.set_metadata(metadata);
        auth_manager.configure_client(static_client_config(client, mcp_server_url, Vec::new()))?;
    }

    let initialized_from_store = if static_client.is_some() {
        credential_store.load().await?.is_some()
    } else {
        auth_manager.initialize_from_store().await?
    };

    if initialized_from_store {
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

    let redirect_uri = format!("http://127.0.0.1:{}/oauth_callback", used_addr.port());

    let mut oauth_state = None;
    let authorization_url = if let Some(client) = &static_client {
        let metadata = auth_manager.discover_metadata().await?;
        auth_manager.set_metadata(metadata);
        let scopes = auth_manager.select_scopes(None, &[]);
        auth_manager.configure_client(static_client_config(
            client,
            redirect_uri.as_str(),
            scopes.clone(),
        ))?;
        let scope_refs: Vec<&str> = scopes.iter().map(|scope| scope.as_str()).collect();
        auth_manager.get_authorization_url(&scope_refs).await?
    } else {
        let mut state = OAuthState::new(mcp_server_url, None).await?;
        state
            .start_authorization_with_metadata_url(
                &[],
                redirect_uri.as_str(),
                Some("goose"),
                Some(CLIENT_METADATA_URL),
            )
            .await?;
        let authorization_url = state.get_authorization_url().await?;
        oauth_state = Some(state);
        authorization_url
    };
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
    } = callback_params?;
    if let Some(mut state) = oauth_state {
        state.handle_callback(&auth_code, &csrf_token).await?;

        let (client_id, token_response) = state.get_credentials().await?;

        let mut auth_manager = state
            .into_authorization_manager()
            .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;

        let granted_scopes: Vec<String> = token_response
            .as_ref()
            .and_then(|tr| tr.scopes())
            .map(|scopes| scopes.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

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
    } else {
        auth_manager
            .exchange_code_for_token(&auth_code, &csrf_token)
            .await?;
        Ok(auth_manager)
    }
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
