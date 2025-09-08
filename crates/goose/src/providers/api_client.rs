use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Certificate, Client, Identity, Response, StatusCode,
};
use serde_json::Value;
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::time::Duration;

pub struct ApiClient {
    client: Client,
    host: String,
    auth: AuthMethod,
    default_headers: HeaderMap,
    timeout: Duration,
    tls_config: Option<TlsConfig>,
}

pub enum AuthMethod {
    BearerToken(String),
    ApiKey {
        header_name: String,
        key: String,
    },
    #[allow(dead_code)]
    OAuth(OAuthConfig),
    Custom(Box<dyn AuthProvider>),
}

#[derive(Debug, Clone)]
pub struct TlsCertKeyPair {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub client_identity: Option<TlsCertKeyPair>,
    pub ca_cert_path: Option<PathBuf>,
}

impl TlsConfig {
    pub fn new() -> Self {
        Self {
            client_identity: None,
            ca_cert_path: None,
        }
    }

    pub fn from_config() -> Result<Option<Self>> {
        let config = crate::config::Config::global();
        let mut tls_config = TlsConfig::new();
        let mut has_tls_config = false;

        let client_cert_path = config.get_param::<String>("GOOSE_CLIENT_CERT_PATH").ok();
        let client_key_path = config.get_param::<String>("GOOSE_CLIENT_KEY_PATH").ok();

        // Validate that both cert and key are provided if either is provided
        match (client_cert_path, client_key_path) {
            (Some(cert_path), Some(key_path)) => {
                tls_config = tls_config.with_client_cert_and_key(
                    std::path::PathBuf::from(cert_path),
                    std::path::PathBuf::from(key_path),
                );
                has_tls_config = true;
            }
            (Some(_), None) => {
                return Err(anyhow::anyhow!(
                    "Client certificate provided (GOOSE_CLIENT_CERT_PATH) but no private key (GOOSE_CLIENT_KEY_PATH)"
                ));
            }
            (None, Some(_)) => {
                return Err(anyhow::anyhow!(
                    "Client private key provided (GOOSE_CLIENT_KEY_PATH) but no certificate (GOOSE_CLIENT_CERT_PATH)"
                ));
            }
            (None, None) => {}
        }

        if let Ok(ca_cert_path) = config.get_param::<String>("GOOSE_CA_CERT_PATH") {
            tls_config = tls_config.with_ca_cert(std::path::PathBuf::from(ca_cert_path));
            has_tls_config = true;
        }

        if has_tls_config {
            Ok(Some(tls_config))
        } else {
            Ok(None)
        }
    }

    pub fn with_client_cert_and_key(mut self, cert_path: PathBuf, key_path: PathBuf) -> Self {
        self.client_identity = Some(TlsCertKeyPair {
            cert_path,
            key_path,
        });
        self
    }

    pub fn with_ca_cert(mut self, path: PathBuf) -> Self {
        self.ca_cert_path = Some(path);
        self
    }

    pub fn is_configured(&self) -> bool {
        self.client_identity.is_some() || self.ca_cert_path.is_some()
    }

    pub fn load_identity(&self) -> Result<Option<Identity>> {
        if let Some(cert_key_pair) = &self.client_identity {
            let cert_pem = read_to_string(&cert_key_pair.cert_path)
                .map_err(|e| anyhow::anyhow!("Failed to read client certificate: {}", e))?;
            let key_pem = read_to_string(&cert_key_pair.key_path)
                .map_err(|e| anyhow::anyhow!("Failed to read client private key: {}", e))?;

            // Create a combined PEM file with certificate and private key
            let combined_pem = format!("{}\n{}", cert_pem, key_pem);

            let identity = Identity::from_pem(combined_pem.as_bytes()).map_err(|e| {
                anyhow::anyhow!("Failed to create identity from cert and key: {}", e)
            })?;

            Ok(Some(identity))
        } else {
            Ok(None)
        }
    }

    pub fn load_ca_certificates(&self) -> Result<Vec<Certificate>> {
        match &self.ca_cert_path {
            Some(ca_path) => {
                let ca_pem = read_to_string(ca_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read CA certificate: {}", e))?;

                let certs = Certificate::from_pem_bundle(ca_pem.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Failed to parse CA certificate bundle: {}", e))?;

                Ok(certs)
            }
            None => Ok(Vec::new()),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OAuthConfig {
    pub host: String,
    pub client_id: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn get_auth_header(&self) -> Result<(String, String)>;
}

pub struct ApiResponse {
    pub status: StatusCode,
    pub payload: Option<Value>,
}

impl fmt::Debug for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::BearerToken(_) => f.debug_tuple("BearerToken").field(&"[hidden]").finish(),
            AuthMethod::ApiKey { header_name, .. } => f
                .debug_struct("ApiKey")
                .field("header_name", header_name)
                .field("key", &"[hidden]")
                .finish(),
            AuthMethod::OAuth(_) => f.debug_tuple("OAuth").field(&"[config]").finish(),
            AuthMethod::Custom(_) => f.debug_tuple("Custom").field(&"[provider]").finish(),
        }
    }
}

impl ApiResponse {
    /// Parse a `reqwest::Response` into an ApiResponse. Optionally accepts the
    /// original request payload (as JSON string) and a provider hint so we can
    /// log all three (request payload, URL and response body) when an error
    /// occurs. This helps debugging when the desktop auto-starts the server
    /// and you cannot enable debug logging easily.
    pub async fn from_response(
        response: Response,
        request_payload: Option<&str>,
        provider_hint: Option<&str>,
    ) -> Result<Self> {
        let status = response.status();

        // Capture URL and raw body text for debugging when there is an error
        let url = response.url().clone();
        let response = response;
        let text_body = response.text().await.ok();

        if !status.is_success() {
            tracing::error!(
                %url,
                status = %status,
                provider = %provider_hint.unwrap_or("<unknown>"),
                request = %request_payload.unwrap_or("<unknown>"),
                response = %text_body.as_deref().unwrap_or("<empty>"),
                "LLM_RESPONSE_ERROR"
            );
        }

        // If the body is valid JSON, parse it into payload; otherwise use None
        let payload = text_body
            .as_deref()
            .and_then(|t| serde_json::from_str::<Value>(t).ok());

        Ok(Self { status, payload })
    }
}

pub struct ApiRequestBuilder<'a> {
    client: &'a ApiClient,
    path: &'a str,
    headers: HeaderMap,
}

impl ApiClient {
    pub fn new(host: String, auth: AuthMethod) -> Result<Self> {
        Self::with_timeout(host, auth, Duration::from_secs(600))
    }

    pub fn with_timeout(host: String, auth: AuthMethod, timeout: Duration) -> Result<Self> {
        let mut client_builder = Client::builder().timeout(timeout);

        // Configure TLS if needed
        let tls_config = TlsConfig::from_config()?;
        if let Some(ref config) = tls_config {
            client_builder = Self::configure_tls(client_builder, config)?;
        }

        let client = client_builder.build()?;

        Ok(Self {
            client,
            host,
            auth,
            default_headers: HeaderMap::new(),
            timeout,
            tls_config,
        })
    }

    fn rebuild_client(&mut self) -> Result<()> {
        let mut client_builder = Client::builder()
            .timeout(self.timeout)
            .default_headers(self.default_headers.clone());

        // Configure TLS if needed
        if let Some(ref tls_config) = self.tls_config {
            client_builder = Self::configure_tls(client_builder, tls_config)?;
        }

        self.client = client_builder.build()?;
        Ok(())
    }

    /// Configure TLS settings on a reqwest ClientBuilder
    fn configure_tls(
        mut client_builder: reqwest::ClientBuilder,
        tls_config: &TlsConfig,
    ) -> Result<reqwest::ClientBuilder> {
        if tls_config.is_configured() {
            // Load client identity (certificate + private key)
            if let Some(identity) = tls_config.load_identity()? {
                client_builder = client_builder.identity(identity);
            }

            // Load CA certificates
            let ca_certs = tls_config.load_ca_certificates()?;
            for ca_cert in ca_certs {
                client_builder = client_builder.add_root_certificate(ca_cert);
            }
        }
        Ok(client_builder)
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Result<Self> {
        self.default_headers = headers;
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self> {
        let header_name = HeaderName::from_bytes(key.as_bytes())?;
        let header_value = HeaderValue::from_str(value)?;
        self.default_headers.insert(header_name, header_value);
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn request<'a>(&'a self, path: &'a str) -> ApiRequestBuilder<'a> {
        ApiRequestBuilder {
            client: self,
            path,
            headers: HeaderMap::new(),
        }
    }

    pub async fn api_post(&self, path: &str, payload: &Value) -> Result<ApiResponse> {
        self.request(path).api_post(payload).await
    }

    pub async fn response_post(&self, path: &str, payload: &Value) -> Result<Response> {
        self.request(path).response_post(payload).await
    }

    pub async fn api_get(&self, path: &str) -> Result<ApiResponse> {
        self.request(path).api_get().await
    }

    pub async fn response_get(&self, path: &str) -> Result<Response> {
        self.request(path).response_get().await
    }

    fn build_url(&self, path: &str) -> Result<url::Url> {
        use url::Url;
        let mut base_url =
            Url::parse(&self.host).map_err(|e| anyhow::anyhow!("Invalid base URL: {}", e))?;

        let base_path = base_url.path();
        if !base_path.is_empty() && base_path != "/" && !base_path.ends_with('/') {
            base_url.set_path(&format!("{}/", base_path));
        }

        // Join the requested path onto the base URL.
        let mut joined = base_url
            .join(path)
            .map_err(|e| anyhow::anyhow!("Failed to construct URL: {}", e))?;

        // If the caller's path does not explicitly end with a slash, ensure we
        // do not introduce a trailing slash on the final URL. Some remote APIs
        // treat a trailing slash as a different endpoint and return 404.
        if !path.ends_with('/') {
            let url_str = joined.as_str().trim_end_matches('/').to_string();
            joined = Url::parse(&url_str).map_err(|e| anyhow::anyhow!("Failed to normalize URL: {}", e))?;
        }

        Ok(joined)
    }

    async fn get_oauth_token(&self, config: &OAuthConfig) -> Result<String> {
        super::oauth::get_oauth_token_async(
            &config.host,
            &config.client_id,
            &config.redirect_url,
            &config.scopes,
        )
        .await
    }
}

impl<'a> ApiRequestBuilder<'a> {
    pub fn header(mut self, key: &str, value: &str) -> Result<Self> {
        let header_name = HeaderName::from_bytes(key.as_bytes())?;
        let header_value = HeaderValue::from_str(value)?;
        self.headers.insert(header_name, header_value);
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    pub async fn api_post(self, payload: &Value) -> Result<ApiResponse> {
        let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let provider_hint = self.client.host.as_str();
        let response = self.response_post(payload).await?;
        ApiResponse::from_response(response, Some(&payload_str), Some(provider_hint)).await
    }

    pub async fn response_post(self, payload: &Value) -> Result<Response> {
        // Log the JSON payload being sent to the LLM and the final URL being called
        let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        tracing::debug!(payload = %payload_str, path = %self.path, "LLM_REQUEST");

        let request_builder = self.send_request(|url, client| client.post(url)).await?;
        // For additional visibility, attempt to extract the final URL from the builder
        // Note: reqwest RequestBuilder does not expose final URL before send, so we build the
        // request to get the URL for logging. This consumes the builder into a Request.
        match request_builder.try_clone() {
            Some(rb) => {
                if let Ok(req) = rb.json(payload).build() {
                    tracing::debug!(url = %req.url(), "LLM_OUTGOING_URL");
                }
            }
            None => {
                tracing::debug!(path = %self.path, "LLM_OUTGOING_PATH_NO_CLONE");
            }
        }

        Ok(request_builder.json(payload).send().await?)
    }

    pub async fn api_get(self) -> Result<ApiResponse> {
        // GET doesn't always have a payload, but keep consistency
        let provider_hint = self.client.host.as_str();
        let response = self.response_get().await?;
        ApiResponse::from_response(response, None, Some(provider_hint)).await
    }

    pub async fn response_get(self) -> Result<Response> {
        let request = self.send_request(|url, client| client.get(url)).await?;
        Ok(request.send().await?)
    }

    async fn send_request<F>(&self, request_builder: F) -> Result<reqwest::RequestBuilder>
    where
        F: FnOnce(url::Url, &Client) -> reqwest::RequestBuilder,
    {
        let url = self.client.build_url(self.path)?;
        let mut request = request_builder(url, &self.client.client);
        request = request.headers(self.headers.clone());

        request = match &self.client.auth {
            AuthMethod::BearerToken(token) => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            AuthMethod::ApiKey { header_name, key } => request.header(header_name.as_str(), key),
            AuthMethod::OAuth(config) => {
                let token = self.client.get_oauth_token(config).await?;
                request.header("Authorization", format!("Bearer {}", token))
            }
            AuthMethod::Custom(provider) => {
                let (header_name, header_value) = provider.get_auth_header().await?;
                request.header(header_name, header_value)
            }
        };

        Ok(request)
    }
}

impl fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiClient")
            .field("host", &self.host)
            .field("auth", &"[auth method]")
            .field("timeout", &self.timeout)
            .field("default_headers", &self.default_headers)
            .finish_non_exhaustive()
    }
}
