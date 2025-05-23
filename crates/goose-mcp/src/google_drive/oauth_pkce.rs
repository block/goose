use std::error::Error;
use std::fs;
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use google_drive3::common::GetToken;
use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use url::Url;

use super::storage::CredentialsManager;

/// Structure representing the OAuth2 configuration file format
#[derive(Debug, Deserialize, Serialize)]
struct OAuth2Config {
    installed: InstalledConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstalledConfig {
    client_id: String,
    project_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_secret: String,
    redirect_uris: Vec<String>,
}

/// Structure for token storage
#[derive(Debug, Deserialize, Serialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<u64>,
    project_id: String,
    scopes: Vec<String>,
}

/// PkceOAuth2Client implements the GetToken trait required by DriveHub
/// It uses the oauth2 crate to implement a PKCE-enabled OAuth2 flow
#[derive(Clone)]
pub struct PkceOAuth2Client {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    credentials_manager: Arc<CredentialsManager>,
    http_client: reqwest::Client,
    project_id: String,
    scopes: Vec<String>,
}

impl PkceOAuth2Client {
    pub fn new(
        config_path: impl AsRef<Path>,
        credentials_manager: Arc<CredentialsManager>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Load and parse the config file
        let config_content = fs::read_to_string(config_path)?;
        let config: OAuth2Config = serde_json::from_str(&config_content)?;

        // Extract the project_id from the config
        let project_id = config.installed.project_id.clone();
        let scopes = vec![];

        // Create OAuth URLs
        let auth_url =
            AuthUrl::new(config.installed.auth_uri).expect("Invalid authorization endpoint URL");
        let token_url =
            TokenUrl::new(config.installed.token_uri).expect("Invalid token endpoint URL");

        // Set up the OAuth2 client
        let client = BasicClient::new(ClientId::new(config.installed.client_id))
            .set_client_secret(ClientSecret::new(config.installed.client_secret))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(
                RedirectUrl::new("http://localhost:18080".to_string())
                    .expect("Invalid redirect URL"),
            );

        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Oauth2 HTTP Client should build");

        Ok(Self {
            client,
            credentials_manager,
            http_client,
            project_id,
            scopes,
        })
    }

    /// Check if a token is expired or about to expire within the buffer period
    fn is_token_expired(&self, expires_at: Option<u64>, buffer_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        // Consider the token expired if it's within buffer_seconds of expiring
        // This gives us a safety margin to avoid using tokens right before expiration
        expires_at
            .map(|expiry_time| now + buffer_seconds >= expiry_time)
            .unwrap_or(true) // If we don't know when it expires, assume it's expired to be safe
    }

    async fn perform_oauth_flow(
        &self,
        scopes: &[&str],
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Create a PKCE code verifier and challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes.iter().map(|&s| Scope::new(s.to_string())))
            .set_pkce_challenge(pkce_challenge)
            .url();

        info!("Opening browser for OAuth2 authentication");
        if let Err(e) = webbrowser::open(auth_url.as_str()) {
            error!("Failed to open browser: {}", e);
            println!("Please open this URL in your browser:\n{}\n", auth_url);
        }

        // Start a local server to receive the authorization code
        // We'll spawn this in a separate thread since it's blocking
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || match Self::start_redirect_server() {
            Ok(result) => {
                let _ = tx.send(Ok(result));
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        });

        // Wait for the code from the redirect server
        let (code, received_state) = rx.await??;

        // Verify the CSRF state
        if received_state.secret() != csrf_token.secret() {
            return Err("CSRF token mismatch".into());
        }

        // Use the built-in exchange_code method with PKCE verifier
        let token_result = self
            .client
            .exchange_code(code)
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let access_token = token_result.access_token().secret().clone();

        // Calculate expires_at as a Unix timestamp by adding expires_in to current time
        let expires_at = token_result.expires_in().map(|duration| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            now + duration.as_secs()
        });

        // Get the refresh token if provided
        if let Some(refresh_token) = token_result.refresh_token() {
            let refresh_token_str = refresh_token.secret().clone();

            // Store token data
            let token_data = TokenData {
                access_token: access_token.clone(),
                refresh_token: refresh_token_str.clone(),
                expires_at,
                project_id: self.project_id.clone(),
                scopes: scopes.iter().map(|s| s.to_string()).collect(),
            };

            // Store updated token data
            self.credentials_manager
                .write_credentials(&token_data)
                .map(|_| debug!("Successfully stored token data"))
                .unwrap_or_else(|e| error!("Failed to store token data: {}", e));
        } else {
            debug!("No refresh token provided in OAuth flow response");
        }

        Ok(access_token)
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        debug!("Attempting to refresh access token");

        // Create a RefreshToken from the string
        let refresh_token = RefreshToken::new(refresh_token.to_string());

        // Use the built-in exchange_refresh_token method
        let token_result = self
            .client
            .exchange_refresh_token(&refresh_token)
            .request_async(&self.http_client)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let access_token = token_result.access_token().secret().clone();

        // Calculate expires_at as a Unix timestamp by adding expires_in to current time
        let expires_at = token_result.expires_in().map(|duration| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            now + duration.as_secs()
        });

        // Get the refresh token - either the new one or reuse the existing one
        let new_refresh_token = token_result
            .refresh_token()
            .map(|token| token.secret().clone())
            .unwrap_or_else(|| refresh_token.secret().to_string());

        // Always update the token data with the new access token and expiration
        let token_data = TokenData {
            access_token: access_token.clone(),
            refresh_token: new_refresh_token.clone(),
            expires_at,
            project_id: self.project_id.clone(),
            scopes: self.scopes.clone(),
        };

        // Store updated token data
        self.credentials_manager
            .write_credentials(&token_data)
            .map(|_| debug!("Successfully stored token data"))
            .unwrap_or_else(|e| error!("Failed to store token data: {}", e));

        Ok(access_token)
    }

    fn start_redirect_server(
    ) -> Result<(AuthorizationCode, CsrfToken), Box<dyn Error + Send + Sync>> {
        let listener = TcpListener::bind("127.0.0.1:18080")?;
        println!("Listening for the authorization code on http://localhost:18080");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut reader = BufReader::new(&stream);
                    let mut request_line = String::new();
                    reader.read_line(&mut request_line)?;

                    let redirect_url = request_line
                        .split_whitespace()
                        .nth(1)
                        .ok_or("Invalid request")?;

                    let url = Url::parse(&format!("http://localhost{}", redirect_url))?;

                    let code = url
                        .query_pairs()
                        .find(|(key, _)| key == "code")
                        .map(|(_, value)| AuthorizationCode::new(value.into_owned()))
                        .ok_or("No code found in the response")?;

                    let state = url
                        .query_pairs()
                        .find(|(key, _)| key == "state")
                        .map(|(_, value)| CsrfToken::new(value.into_owned()))
                        .ok_or("No state found in the response")?;

                    // Send a success response to the browser
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                        <html><body><h1>Authentication successful!</h1>\
                        <p>You can now close this window and return to the application.</p></body></html>";

                    stream.write_all(response.as_bytes())?;
                    stream.flush()?;

                    return Ok((code, state));
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }

        Err("Failed to receive authorization code".into())
    }
}

// impl GetToken for use with DriveHub directly
// see google_drive3::common::GetToken
impl GetToken for PkceOAuth2Client {
    fn get_token<'a>(
        &'a self,
        scopes: &'a [&str],
    ) -> Pin<
        Box<dyn Future<Output = Result<Option<String>, Box<dyn Error + Send + Sync>>> + Send + 'a>,
    > {
        Box::pin(async move {
            // Try to read token data from storage to check if we have a valid token
            if let Ok(token_data) = self.credentials_manager.read_credentials::<TokenData>() {
                // Verify the project_id matches
                if token_data.project_id == self.project_id {
                    // Convert stored scopes to &str slices for comparison
                    let stored_scope_refs: Vec<&str> =
                        token_data.scopes.iter().map(|s| s.as_str()).collect();

                    // Check if we need additional scopes
                    let needs_additional_scopes = scopes.iter().any(|&scope| {
                        !stored_scope_refs
                            .iter()
                            .any(|&stored| stored.contains(scope))
                    });

                    if !needs_additional_scopes {
                        // Check if the token is expired or expiring within a 5-min buffer
                        if !self.is_token_expired(token_data.expires_at, 300) {
                            return Ok(Some(token_data.access_token));
                        }

                        // Token is expired or will expire soon, try to refresh it
                        debug!("Token is expired or will expire soon, refreshing...");

                        // Try to refresh the token
                        if let Ok(access_token) =
                            self.refresh_token(&token_data.refresh_token).await
                        {
                            debug!("Successfully refreshed access token");
                            return Ok(Some(access_token));
                        }
                    } else {
                        // Only allocate new strings when we need to combine scopes
                        let mut combined_scopes: Vec<&str> =
                            Vec::with_capacity(scopes.len() + stored_scope_refs.len());
                        combined_scopes.extend(scopes);
                        combined_scopes.extend(stored_scope_refs.iter().filter(|&&stored| {
                            !scopes.iter().any(|&scope| stored.contains(scope))
                        }));

                        return self
                            .perform_oauth_flow(&combined_scopes)
                            .await
                            .map(Some)
                            .map_err(|e| {
                                error!("OAuth flow failed: {}", e);
                                e
                            });
                    }
                }
            }
            // If we get here, either:
            // 1. The project ID didn't match
            // 2. Token refresh failed
            // 3. There are no valid tokens yet
            // 4. We didn't have to change the scopes of an existing token
            // Fallback: perform interactive OAuth flow
            self.perform_oauth_flow(scopes)
                .await
                .map(Some)
                .map_err(|e| {
                    error!("OAuth flow failed: {}", e);
                    e
                })
        })
    }
}
