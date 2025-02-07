use anyhow::Result;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use etcetera::{choose_app_strategy, AppStrategy};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{oneshot, Mutex as TokioMutex};
use url::Url;

lazy_static! {
    static ref OAUTH_MUTEX: TokioMutex<()> = TokioMutex::new(());
}

#[derive(Debug, Clone)]
struct OidcEndpoints {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    expires_at: Option<DateTime<Utc>>,
}

struct TokenCache {
    cache_path: PathBuf,
}

fn get_base_path() -> PathBuf {
    // choose_app_strategy().config_dir()
    // - macOS/Linux: ~/.config/goose/databricks/oauth
    // - Windows:     ~\AppData\Roaming\Block\goose\config\databricks\oauth\
    choose_app_strategy(crate::config::APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .in_config_dir("databricks/oauth")
}

impl TokenCache {
    fn new(host: &str, client_id: &str, scopes: &[String]) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(host.as_bytes());
        hasher.update(client_id.as_bytes());
        hasher.update(scopes.join(",").as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        fs::create_dir_all(get_base_path()).unwrap();
        let cache_path = get_base_path().join(format!("{}.json", hash));

        Self { cache_path }
    }

    fn load_token(&self) -> Option<TokenData> {
        if let Ok(contents) = fs::read_to_string(&self.cache_path) {
            if let Ok(token_data) = serde_json::from_str::<TokenData>(&contents) {
                if let Some(expires_at) = token_data.expires_at {
                    if expires_at > Utc::now() {
                        return Some(token_data);
                    }
                } else {
                    return Some(token_data);
                }
            }
        }
        None
    }

    fn save_token(&self, token_data: &TokenData) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string(token_data)?;
        fs::write(&self.cache_path, contents)?;
        Ok(())
    }
}

async fn get_workspace_endpoints(host: &str) -> Result<OidcEndpoints> {
    let base_url = Url::parse(host).expect("Invalid host URL");
    let oidc_url = base_url
        .join("oidc/.well-known/oauth-authorization-server")
        .expect("Invalid OIDC URL");

    let client = reqwest::Client::new();
    let resp = client.get(oidc_url.clone()).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to get OIDC configuration from {}",
            oidc_url.to_string()
        ));
    }

    let oidc_config: Value = resp.json().await?;

    let authorization_endpoint = oidc_config
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("authorization_endpoint not found in OIDC configuration"))?
        .to_string();

    let token_endpoint = oidc_config
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("token_endpoint not found in OIDC configuration"))?
        .to_string();

    Ok(OidcEndpoints {
        authorization_endpoint,
        token_endpoint,
    })
}

struct OAuthFlow {
    endpoints: OidcEndpoints,
    client_id: String,
    redirect_url: String,
    scopes: Vec<String>,
    state: String,
    verifier: String,
}

impl OAuthFlow {
    fn new(
        endpoints: OidcEndpoints,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            endpoints,
            client_id,
            redirect_url,
            scopes,
            state: nanoid::nanoid!(16),
            verifier: nanoid::nanoid!(64),
        }
    }

    fn get_authorization_url(&self) -> String {
        let challenge = {
            let digest = sha2::Sha256::digest(self.verifier.as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
        };

        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", &self.redirect_url),
            ("scope", &self.scopes.join(" ")),
            ("state", &self.state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];

        format!(
            "{}?{}",
            self.endpoints.authorization_endpoint,
            serde_urlencoded::to_string(params).unwrap()
        )
    }

    async fn exchange_code_for_token(&self, code: &str) -> Result<TokenData> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_url),
            ("code_verifier", &self.verifier),
            ("client_id", &self.client_id),
        ];

        let client = reqwest::Client::new();
        let resp = client
            .post(&self.endpoints.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to exchange code for token: {}",
                err_text
            ));
        }

        let token_response: Value = resp.json().await?;
        let access_token = token_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("access_token not found in token response"))?
            .to_string();

        let expires_in = token_response
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);

        Ok(TokenData {
            access_token,
            expires_at: Some(expires_at),
        })
    }

    async fn execute(&self) -> Result<TokenData> {
        // Create a channel that will send the auth code from the app process
        let (tx, rx) = oneshot::channel();
        let state = self.state.clone();
        // Axum can theoretically spawn multiple threads, so we need this to be in an Arc even
        // though it will ultimately only get used once
        let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

        // Setup a server that will recieve the redirect, capture the code, and display success/failure
        let app = Router::new().route(
            "/",
            get(move |Query(params): Query<HashMap<String, String>>| {
                let tx = Arc::clone(&tx);
                let state = state.clone();
                async move {
                    let code = params.get("code").cloned();
                    let received_state = params.get("state").cloned();

                    if let (Some(code), Some(received_state)) = (code, received_state) {
                        if received_state == state {
                            if let Some(sender) = tx.lock().await.take() {
                                if sender.send(code).is_ok() {
                                    // Use the improved HTML response
                                    return Html(
                                        "<h2>Login Success</h2><p>You can close this window</p>",
                                    );
                                }
                            }
                            Html("<h2>Error</h2><p>Authentication already completed.</p>")
                        } else {
                            Html("<h2>Error</h2><p>State mismatch.</p>")
                        }
                    } else {
                        Html("<h2>Error</h2><p>Authentication failed.</p>")
                    }
                }
            }),
        );

        // Start the server to accept the oauth code
        let redirect_url = Url::parse(&self.redirect_url)?;
        let port = redirect_url.port().unwrap_or(80);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let server_handle = tokio::spawn(async move {
            let server = axum::serve(listener, app);
            server.await.unwrap();
        });

        // Open the browser which will redirect with the code to the server
        let authorization_url = self.get_authorization_url();
        if webbrowser::open(&authorization_url).is_err() {
            println!(
                "Please open this URL in your browser:\n{}",
                authorization_url
            );
        }

        // Wait for the authorization code with a timeout
        let code = tokio::time::timeout(
            std::time::Duration::from_secs(60), // 1 minute timeout
            rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Authentication timed out"))??;

        // Stop the server
        server_handle.abort();

        // Exchange the code for a token
        self.exchange_code_for_token(&code).await
    }
}

pub(crate) async fn get_oauth_token_async(
    host: &str,
    client_id: &str,
    redirect_url: &str,
    scopes: &[String],
) -> Result<String> {
    // Acquire the global mutex to ensure only one OAuth flow runs at a time
    let _guard = OAUTH_MUTEX.lock().await;

    let token_cache = TokenCache::new(host, client_id, scopes);

    // Try cache first
    if let Some(token) = token_cache.load_token() {
        return Ok(token.access_token);
    }

    // Get endpoints and execute flow
    let endpoints = get_workspace_endpoints(host).await?;
    let flow = OAuthFlow::new(
        endpoints,
        client_id.to_string(),
        redirect_url.to_string(),
        scopes.to_vec(),
    );

    // Execute the OAuth flow and get token
    let token = flow.execute().await?;

    // Cache and return
    token_cache.save_token(&token)?;
    Ok(token.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_get_workspace_endpoints() -> Result<()> {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "authorization_endpoint": "https://example.com/oauth2/authorize",
            "token_endpoint": "https://example.com/oauth2/token"
        });

        Mock::given(method("GET"))
            .and(path("/oidc/.well-known/oauth-authorization-server"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let endpoints = get_workspace_endpoints(&mock_server.uri()).await?;

        assert_eq!(
            endpoints.authorization_endpoint,
            "https://example.com/oauth2/authorize"
        );
        assert_eq!(endpoints.token_endpoint, "https://example.com/oauth2/token");

        Ok(())
    }

    #[test]
    fn test_token_cache() -> Result<()> {
        let cache = TokenCache::new(
            "https://example.com",
            "test-client",
            &["scope1".to_string()],
        );

        let token_data = TokenData {
            access_token: "test-token".to_string(),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        };

        cache.save_token(&token_data)?;

        let loaded_token = cache.load_token().unwrap();
        assert_eq!(loaded_token.access_token, token_data.access_token);

        Ok(())
    }
}
