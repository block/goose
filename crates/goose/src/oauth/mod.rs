mod persist;

pub use persist::GooseCredentialStore;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use minijinja::render;
use oauth2::TokenResponse;
use rmcp::transport::auth::{
    AuthorizationRequest, CredentialStore, OAuthState, StoredCredentials, WWWAuthenticateParams,
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlowConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub client_metadata_url: Option<String>,
    /// `WWW-Authenticate` header value from a 401/403 response. When present,
    /// discovery is seeded from the challenge (its `resource_metadata` URL and
    /// `scope` hint) instead of probing well-known locations.
    #[serde(skip)]
    pub challenge: Option<String>,
}

#[derive(Clone)]
struct AppState {
    callback_receiver: Arc<Mutex<Option<oneshot::Sender<String>>>>,
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

async fn complete_automatic_authorization(
    authorization_url: &str,
    redirect_uri: &str,
) -> Result<Option<String>, anyhow::Error> {
    if std::env::var_os("GOOSE_OAUTH_AUTOMATIC_CALLBACK").is_none() {
        return Ok(None);
    }

    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(authorization_url)
        .send()
        .await?;
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or_else(|| anyhow::anyhow!("authorization response did not include Location"))?
        .to_str()?;
    let callback_url = url::Url::parse(location)?;
    let expected_redirect = url::Url::parse(redirect_uri)?;
    if callback_url.scheme() != expected_redirect.scheme()
        || callback_url.host_str() != expected_redirect.host_str()
        || callback_url.port_or_known_default() != expected_redirect.port_or_known_default()
        || callback_url.path() != expected_redirect.path()
    {
        anyhow::bail!("authorization response redirected to an unexpected callback URI");
    }
    Ok(Some(callback_url.to_string()))
}

async fn wait_for_callback(
    callback_receiver: oneshot::Receiver<String>,
    timeout_duration: Duration,
    name: &str,
    authorization_url: &str,
) -> Result<String, anyhow::Error> {
    match tokio::time::timeout(timeout_duration, callback_receiver).await {
        Ok(Ok(callback_url)) => Ok(callback_url),
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

pub async fn oauth_flow(
    mcp_server_url: &String,
    name: &String,
) -> Result<AuthorizationManager, anyhow::Error> {
    oauth_flow_with_challenge(mcp_server_url, name, None).await
}

pub async fn oauth_flow_with_challenge(
    mcp_server_url: &String,
    name: &String,
    challenge: Option<String>,
) -> Result<AuthorizationManager, anyhow::Error> {
    let config = OAuthFlowConfig {
        client_id: std::env::var("GOOSE_MCP_OAUTH_CLIENT_ID").ok(),
        client_secret: std::env::var("GOOSE_MCP_OAUTH_CLIENT_SECRET").ok(),
        client_metadata_url: std::env::var("GOOSE_MCP_OAUTH_CLIENT_METADATA_URL").ok(),
        challenge,
    };
    oauth_flow_with_config(mcp_server_url, name, config).await
}

pub async fn oauth_flow_with_config(
    mcp_server_url: &String,
    name: &String,
    flow_config: OAuthFlowConfig,
) -> Result<AuthorizationManager, anyhow::Error> {
    let credential_store = GooseCredentialStore::new(name.clone());
    let mut auth_manager = AuthorizationManager::new(mcp_server_url).await?;
    auth_manager.set_credential_store(credential_store.clone());

    // With a challenge in hand (e.g. a 403 insufficient_scope after a
    // previously successful authorization), a refresh cannot satisfy the new
    // scope requirement: skip straight to a full re-authorization that
    // requests the union of scopes.
    if auth_manager.initialize_from_store().await? && flow_config.challenge.is_none() {
        match auth_manager.refresh_token().await {
            Ok(_) => return Ok(auth_manager),
            Err(e) => warn!(
                "[OAuth:{}] Token refresh failed: {} - clearing stored credentials and falling back to browser auth",
                name, e
            ),
        }
        if let Err(e) = credential_store.clear().await {
            warn!("[OAuth:{}] error clearing bad credentials: {}", name, e);
        }
    }

    let (callback_sender, callback_receiver) = oneshot::channel::<String>();
    let app_state = AppState {
        callback_receiver: Arc::new(Mutex::new(Some(callback_sender))),
    };
    let rendered = render!(CALLBACK_TEMPLATE, name => name);
    let handler = move |Query(params): Query<CallbackParams>, State(state): State<AppState>| {
        let rendered = rendered.clone();
        async move {
            if let Some(sender) = state.callback_receiver.lock().await.take() {
                let query = serde_urlencoded::to_string([
                    ("code", params.code.as_str()),
                    ("state", params.state.as_str()),
                ])
                .unwrap_or_default();
                let issuer = params
                    .iss
                    .as_deref()
                    .map(|iss| format!("&iss={}", urlencoding::encode(iss)))
                    .unwrap_or_default();
                let _ = sender.send(format!("http://callback/oauth_callback?{query}{issuer}"));
            }
            Html(rendered)
        }
    };
    let app = Router::new()
        .route("/oauth_callback", get(handler))
        .with_state(app_state);

    let port = std::env::var("GOOSE_OAUTH_CALLBACK_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
    let used_addr = listener.local_addr()?;
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("Callback server error: {}", e);
        }
    });

    let mut oauth_state = OAuthState::new(mcp_server_url, None).await?;
    let redirect_uri = format!("http://127.0.0.1:{}/oauth_callback", used_addr.port());
    let mut request = AuthorizationRequest::new(redirect_uri.clone()).with_client_name("goose");
    if let Some(challenge) = flow_config.challenge {
        // SEP-2350: a re-authorization triggered by a scope challenge requests
        // the union of previously-granted scopes and the newly challenged
        // scopes. The fresh AuthorizationManager has no scope memory, so seed
        // the union from the credential store.
        let mut scopes: Vec<String> = credential_store
            .load()
            .await
            .ok()
            .flatten()
            .map(|stored| stored.granted_scopes)
            .unwrap_or_default();
        if let Ok(base_url) = url::Url::parse(mcp_server_url) {
            if let Some(challenged) = WWWAuthenticateParams::parse(&challenge, &base_url).scope {
                scopes.extend(challenged.split_whitespace().map(str::to_string));
            }
        }
        scopes.dedup();
        if !scopes.is_empty() {
            request = request.with_scopes(scopes);
        }
        request = request.with_challenge(challenge);
    }
    if let Some(client_id) = flow_config.client_id {
        request = request.with_preregistered_client(client_id);
        if let Some(client_secret) = flow_config.client_secret {
            request = request.with_client_secret(client_secret);
        }
    } else {
        request = request.with_client_metadata_url(
            flow_config
                .client_metadata_url
                .unwrap_or_else(|| CLIENT_METADATA_URL.to_string()),
        );
    }
    oauth_state.start_authorization(request).await?;

    let authorization_url = oauth_state.get_authorization_url().await?;
    let callback_url = if let Some(callback_url) =
        complete_automatic_authorization(authorization_url.as_str(), &redirect_uri).await?
    {
        callback_url
    } else {
        announce_authorization_url(name, authorization_url.as_str());
        if let Err(e) = webbrowser::open(authorization_url.as_str()) {
            warn!(
                "[OAuth:{}] Failed to open browser automatically: {}",
                name, e
            );
        }
        wait_for_callback(
            callback_receiver,
            oauth_callback_timeout(),
            name,
            authorization_url.as_str(),
        )
        .await?
    };
    server_handle.abort();
    oauth_state.handle_callback_url(&callback_url).await?;

    let (client_id, token_response) = oauth_state.get_credentials().await?;
    let mut auth_manager = oauth_state
        .into_authorization_manager()
        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;
    let granted_scopes = token_response
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
    async fn wait_for_callback_returns_received_callback_url() {
        let (sender, receiver) = oneshot::channel();
        let expected = "http://callback/oauth_callback?code=auth-code&state=csrf-state";
        sender.send(expected.to_string()).unwrap();

        let callback_url = wait_for_callback(
            receiver,
            Duration::from_secs(1),
            "test-server",
            "https://auth.example/authorize",
        )
        .await
        .unwrap();

        assert_eq!(callback_url, expected);
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
