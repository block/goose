use anyhow::Result;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::{oneshot, Mutex as TokioMutex};
use url::Url;

#[derive(Debug, Clone)]
struct OidcEndpoints {
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ClientRegistrationRequest {
    redirect_uris: Vec<String>,
    token_endpoint_auth_method: String,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    client_name: String,
    client_uri: String,
}

#[derive(Serialize, Deserialize)]
struct ClientRegistrationResponse {
    client_id: String,
    client_id_issued_at: Option<u64>,
    #[serde(default)]
    client_secret: Option<String>,
}

/// OAuth configuration for any service
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub oauth_host: String,
    pub redirect_uri: String,
    pub client_name: String,
    pub client_uri: String,
    pub discovery_path: Option<String>,
}

impl ServiceConfig {
    /// Create a generic OAuth configuration from an MCP endpoint URL
    /// Extracts the base URL for OAuth discovery
    pub fn from_mcp_endpoint(mcp_url: &str) -> Result<Self> {
        let parsed_url = Url::parse(mcp_url.trim())?;
        let oauth_host = format!(
            "{}://{}{}",
            parsed_url.scheme(),
            parsed_url.host_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid MCP URL: no host found in {}", mcp_url)
            })?,
            if let Some(port) = parsed_url.port() {
                format!(":{}", port)
            } else {
                String::new()
            }
        );

        Ok(Self {
            oauth_host,
            redirect_uri: "http://localhost:8020".to_string(),
            client_name: "Goose MCP Client".to_string(),
            client_uri: "https://github.com/block/goose".to_string(),
            discovery_path: None, // Use standard discovery
        })
    }

    /// Create configuration with custom discovery path for non-standard services
    pub fn with_custom_discovery(mut self, discovery_path: String) -> Self {
        self.discovery_path = Some(discovery_path);
        self
    }

    /// Get the canonical resource URI for the MCP server
    /// This is used as the resource parameter in OAuth requests (RFC 8707)
    pub fn get_canonical_resource_uri(&self, mcp_url: &str) -> Result<String> {
        let parsed_url = Url::parse(mcp_url.trim())?;

        // Build canonical URI: scheme://host[:port][/path]
        let mut canonical = format!(
            "{}://{}",
            parsed_url.scheme().to_lowercase(),
            parsed_url
                .host_str()
                .ok_or_else(|| {
                    anyhow::anyhow!("Invalid MCP URL: no host found in {}", mcp_url)
                })?
                .to_lowercase()
        );

        // Add port if not default
        if let Some(port) = parsed_url.port() {
            canonical.push_str(&format!(":{}", port));
        }

        // Add path if present and not just "/"
        let path = parsed_url.path();
        if !path.is_empty() && path != "/" {
            canonical.push_str(path);
        }

        Ok(canonical)
    }
}

struct OAuthFlow {
    endpoints: OidcEndpoints,
    client_id: String,
    redirect_url: String,
    state: String,
    verifier: String,
}

impl OAuthFlow {
    fn new(endpoints: OidcEndpoints, client_id: String, redirect_url: String) -> Self {
        Self {
            endpoints,
            client_id,
            redirect_url,
            state: nanoid::nanoid!(16),
            verifier: nanoid::nanoid!(64),
        }
    }

    /// Register a dynamic client and return the client_id
    async fn register_client(endpoints: &OidcEndpoints, config: &ServiceConfig) -> Result<String> {
        let Some(registration_endpoint) = &endpoints.registration_endpoint else {
            return Err(anyhow::anyhow!("No registration endpoint available"));
        };

        let registration_request = ClientRegistrationRequest {
            redirect_uris: vec![config.redirect_uri.clone()],
            token_endpoint_auth_method: "none".to_string(),
            grant_types: vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            response_types: vec!["code".to_string()],
            client_name: config.client_name.clone(),
            client_uri: config.client_uri.clone(),
        };

        tracing::info!("Registering dynamic client with OAuth server...");

        let client = reqwest::Client::new();
        let resp = client
            .post(registration_endpoint)
            .header("Content-Type", "application/json")
            .json(&registration_request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to register client: {} - {}",
                status,
                err_text
            ));
        }

        let registration_response: ClientRegistrationResponse = resp.json().await?;

        tracing::info!(
            "Client registered successfully with ID: {}",
            registration_response.client_id
        );
        Ok(registration_response.client_id)
    }

    fn get_authorization_url(&self, resource: &str) -> String {
        let challenge = {
            let digest = sha2::Sha256::digest(self.verifier.as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
        };

        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", &self.redirect_url),
            ("state", &self.state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("resource", resource), // RFC 8707 Resource Parameter
        ];

        format!(
            "{}?{}",
            self.endpoints.authorization_endpoint,
            serde_urlencoded::to_string(params).unwrap()
        )
    }

    async fn exchange_code_for_token(&self, code: &str, resource: &str) -> Result<TokenData> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_url),
            ("code_verifier", &self.verifier),
            ("client_id", &self.client_id),
            ("resource", resource), // RFC 8707 Resource Parameter
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

        let refresh_token = token_response
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(TokenData {
            access_token,
            refresh_token,
        })
    }

    async fn execute(&self, resource: &str) -> Result<TokenData> {
        // Create a channel that will send the auth code from the callback
        let (tx, rx) = oneshot::channel();
        let state = self.state.clone();
        let tx = Arc::new(TokioMutex::new(Some(tx)));

        // Setup a server that will receive the redirect and capture the code
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
                                    return Html(
                                        "<h2>Authentication Successful!</h2><p>You can close this window and return to the application.</p>",
                                    );
                                }
                            }
                            Html("<h2>Error</h2><p>Authentication already completed.</p>")
                        } else {
                            Html("<h2>Error</h2><p>State mismatch - possible security issue.</p>")
                        }
                    } else {
                        Html("<h2>Error</h2><p>Authentication failed - missing parameters.</p>")
                    }
                }
            }),
        );

        // Start the callback server
        let redirect_url = Url::parse(&self.redirect_url)?;
        let port = redirect_url.port().unwrap_or(8020);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let server_handle = tokio::spawn(async move {
            let server = axum::serve(listener, app);
            server.await.unwrap();
        });

        // Open the browser for OAuth
        let authorization_url = self.get_authorization_url(resource);
        tracing::info!("Opening browser for OAuth authentication...");

        if webbrowser::open(&authorization_url).is_err() {
            tracing::warn!("Could not open browser automatically. Please open this URL manually:");
            tracing::warn!("{}", authorization_url);
        }

        // Wait for the authorization code with a timeout
        let code = tokio::time::timeout(
            std::time::Duration::from_secs(120), // 2 minute timeout
            rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Authentication timed out after 2 minutes"))??;

        // Stop the callback server
        server_handle.abort();

        // Exchange the code for a token
        self.exchange_code_for_token(&code, resource).await
    }
}

async fn get_oauth_endpoints(
    host: &str,
    custom_discovery_path: Option<&str>,
) -> Result<OidcEndpoints> {
    let base_url = Url::parse(host)?;
    let client = reqwest::Client::new();

    // Define discovery paths to try, with custom path first if provided
    let mut discovery_paths = Vec::new();
    if let Some(custom_path) = custom_discovery_path {
        discovery_paths.push(custom_path);
    }
    discovery_paths.extend([
        "/.well-known/oauth-authorization-server",
        "/.well-known/openid_configuration",
        "/oauth/.well-known/oauth-authorization-server",
        "/.well-known/oauth_authorization_server", // Some services use underscore
    ]);

    let discovery_paths_for_error = discovery_paths.clone(); // Clone for error message
    let mut last_error = None;

    // Try each discovery path until one works
    for path in discovery_paths {
        match base_url.join(path) {
            Ok(discovery_url) => {
                tracing::debug!("Trying OAuth discovery at: {}", discovery_url);

                match client.get(discovery_url.clone()).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<Value>().await {
                            Ok(oidc_config) => {
                                // Try to parse the OAuth configuration
                                match parse_oauth_config(oidc_config) {
                                    Ok(endpoints) => {
                                        tracing::info!(
                                            "Successfully discovered OAuth endpoints at: {}",
                                            discovery_url
                                        );
                                        return Ok(endpoints);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Invalid OAuth config at {}: {}",
                                            discovery_url,
                                            e
                                        );
                                        last_error = Some(e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Failed to parse JSON from {}: {}",
                                    discovery_url,
                                    e
                                );
                                last_error = Some(e.into());
                            }
                        }
                    }
                    Ok(resp) => {
                        tracing::debug!("HTTP {} from {}", resp.status(), discovery_url);
                    }
                    Err(e) => {
                        tracing::debug!("Request failed to {}: {}", discovery_url, e);
                        last_error = Some(e.into());
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Invalid discovery URL {}{}: {}", host, path, e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!(
            "No OAuth discovery endpoint found at {}. Tried paths: {:?}",
            host,
            discovery_paths_for_error
        )
    }))
}

fn parse_oauth_config(oidc_config: Value) -> Result<OidcEndpoints> {
    let authorization_endpoint = oidc_config
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("authorization_endpoint not found in OAuth configuration"))?
        .to_string();

    let token_endpoint = oidc_config
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("token_endpoint not found in OAuth configuration"))?
        .to_string();

    let registration_endpoint = oidc_config
        .get("registration_endpoint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(OidcEndpoints {
        authorization_endpoint,
        token_endpoint,
        registration_endpoint,
    })
}

/// Perform OAuth flow for a service
pub async fn authenticate_service(config: ServiceConfig, mcp_url: &str) -> Result<String> {
    tracing::info!("Starting OAuth authentication for service...");

    // Get the canonical resource URI for the MCP server
    let resource_uri = config.get_canonical_resource_uri(mcp_url)?;
    tracing::info!("Using resource URI: {}", resource_uri);

    // Get OAuth endpoints using flexible discovery
    let endpoints =
        get_oauth_endpoints(&config.oauth_host, config.discovery_path.as_deref()).await?;

    // Register dynamic client to get client_id
    let client_id = OAuthFlow::register_client(&endpoints, &config).await?;

    // Create and execute OAuth flow with the dynamic client_id
    let flow = OAuthFlow::new(endpoints, client_id, config.redirect_uri);

    let token_data = flow.execute(&resource_uri).await?;

    tracing::info!("OAuth authentication successful!");
    Ok(token_data.access_token)
}
