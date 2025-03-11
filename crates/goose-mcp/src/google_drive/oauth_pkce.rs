use std::error::Error;
use std::fs;
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

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

use crate::google_drive::token_storage::CredentialsManager;

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
}

use std::sync::Mutex;

/// PkceOAuth2Client implements the GetToken trait required by DriveHub
/// It uses the oauth2 crate to implement a PKCE-enabled OAuth2 flow
#[derive(Clone)]
pub struct PkceOAuth2Client {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    credentials_manager: Arc<CredentialsManager>,
    refresh_token: Arc<Mutex<Option<String>>>,
    http_client: reqwest::Client,
}

impl PkceOAuth2Client {
    pub fn new(
        config_path: impl AsRef<Path>,
        credentials_manager: Arc<CredentialsManager>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Load and parse the config file
        let config_content = fs::read_to_string(config_path)?;
        let config: OAuth2Config = serde_json::from_str(&config_content)?;

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
                RedirectUrl::new("http://localhost:8080".to_string())
                    .expect("Invalid redirect URL"),
            );

        // Try to load a refresh token from storage
        let refresh_token = credentials_manager
            .read_credentials::<TokenData>()
            .inspect_err(|e| debug!("No stored credentials found or error reading them: {}", e))
            .ok()
            .map(|token_data| token_data.refresh_token);

        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Oauth2 HTTP Client should build");

        Ok(Self {
            client,
            credentials_manager,
            refresh_token: Arc::new(Mutex::new(refresh_token)),
            http_client,
        })
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

        // Update the stored refresh token if a new one was provided
        // not all authorization servers return a new refresh token
        if let Some(refresh_token) = token_result.refresh_token() {
            let token_data = TokenData {
                access_token: access_token.clone(),
                refresh_token: refresh_token.secret().clone(),
                expires_at: token_result.expires_in().map(|d| d.as_secs()),
            };

            self.refresh_token
                .lock()
                .map(|mut token_guard| {
                    *token_guard = Some(refresh_token.secret().clone());
                    debug!("Successfully updated in-memory refresh token");
                })
                .unwrap_or_else(|_| error!("Failed to acquire lock on refresh token"));

            self.credentials_manager
                .write_credentials(&token_data)
                .map(|_| debug!("Successfully stored refresh token"))
                .unwrap_or_else(|e| error!("Failed to store refresh token: {}", e));
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

        // Update the stored refresh token if a new one was provided
        // not all authorization servers return a new refresh token
        if let Some(refresh_token) = token_result.refresh_token() {
            let token_data = TokenData {
                access_token: access_token.clone(),
                refresh_token: refresh_token.secret().clone(),
                expires_at: token_result.expires_in().map(|d| d.as_secs()),
            };

            self.refresh_token
                .lock()
                .map(|mut token_guard| {
                    *token_guard = Some(refresh_token.secret().clone());
                    debug!("Successfully updated in-memory refresh token");
                })
                .unwrap_or_else(|_| error!("Failed to acquire lock on refresh token"));

            self.credentials_manager
                .write_credentials(&token_data)
                .map(|_| debug!("Successfully stored refresh token"))
                .unwrap_or_else(|e| error!("Failed to store refresh token: {}", e));
        }

        Ok(access_token)
    }

    fn start_redirect_server(
    ) -> Result<(AuthorizationCode, CsrfToken), Box<dyn Error + Send + Sync>> {
        let listener = TcpListener::bind("127.0.0.1:8080")?;
        println!("Listening for the authorization code on http://localhost:8080");

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

impl GetToken for PkceOAuth2Client {
    fn get_token<'a>(
        &'a self,
        scopes: &'a [&str],
    ) -> Pin<
        Box<dyn Future<Output = Result<Option<String>, Box<dyn Error + Send + Sync>>> + Send + 'a>,
    > {
        Box::pin(async move {
            // Attempt to get token from memory
            let token_from_memory = self
                .refresh_token
                .lock()
                .ok()
                .and_then(|guard| guard.clone());

            // In error cases we just fall through to checking storage
            if let Some(ref token) = token_from_memory {
                if let Ok(access_token) = self.refresh_token(token).await {
                    debug!("Successfully refreshed access token from memory");
                    return Ok(Some(access_token));
                }
            }

            // Attempt to read token from storage and update in-memory cache
            let token_from_storage = self
                .credentials_manager
                .read_credentials::<TokenData>()
                .ok()
                .map(|token_data| {
                    if let Ok(mut token_guard) = self.refresh_token.lock() {
                        *token_guard = Some(token_data.refresh_token.clone());
                        debug!("Updated in-memory refresh token from storage");
                    }
                    token_data.refresh_token
                });

            // If we fail to use the refresh token here, fall through to full OAuth flow
            if let Some(ref token) = token_from_storage {
                if let Ok(access_token) = self.refresh_token(token).await {
                    debug!("Successfully refreshed access token from storage");
                    return Ok(Some(access_token));
                }
            }

            // Fallback: perform interactive OAuth flow
            match self.perform_oauth_flow(scopes).await {
                Ok(token) => {
                    debug!("Successfully obtained new access token through OAuth flow");
                    Ok(Some(token))
                }
                Err(e) => {
                    error!("OAuth flow failed: {}", e);
                    Err(e)
                }
            }
        })
    }
}

