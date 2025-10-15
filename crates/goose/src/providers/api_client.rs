use anyhow::Result;
use async_trait::async_trait;
use opentelemetry::{global, propagation::Injector};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Certificate, Client, Identity, Response, StatusCode,
};
use serde_json::Value;
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::time::Duration;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
    pub async fn from_response(response: Response) -> Result<Self> {
        let status = response.status();
        let payload = response.json().await.ok();
        Ok(Self { status, payload })
    }
}

pub struct ApiRequestBuilder<'a> {
    client: &'a ApiClient,
    path: &'a str,
    headers: HeaderMap,
}

/// A header injector for OpenTelemetry trace propagation
struct HeaderInjector<'a> {
    headers: &'a mut HeaderMap,
}

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                self.headers.insert(header_name, header_value);
            }
        }
    }
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

        base_url
            .join(path)
            .map_err(|e| anyhow::anyhow!("Failed to construct URL: {}", e))
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
        let response = self.response_post(payload).await?;
        ApiResponse::from_response(response).await
    }

    pub async fn response_post(self, payload: &Value) -> Result<Response> {
        // Log the JSON payload being sent to the LLM
        tracing::debug!(
            "LLM_REQUEST: {}",
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        );

        let request = self.send_request(|url, client| client.post(url)).await?;
        Ok(request.json(payload).send().await?)
    }

    pub async fn api_get(self) -> Result<ApiResponse> {
        let response = self.response_get().await?;
        ApiResponse::from_response(response).await
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

        // Inject OpenTelemetry trace context headers
        let mut trace_headers = HeaderMap::new();
        let cx = Span::current().context();
        global::get_text_map_propagator(|propagator| {
            let mut injector = HeaderInjector {
                headers: &mut trace_headers,
            };
            propagator.inject_context(&cx, &mut injector);
        });

        // Add trace headers first, then request headers (allowing overrides)
        request = request.headers(trace_headers);
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{trace::TracerProvider, KeyValue};
    use opentelemetry_sdk::{
        trace::{RandomIdGenerator, Sampler, TracerProvider as SdkTracerProvider},
        Resource,
    };
    use tracing::instrument;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    #[test]
    fn test_header_injector_implements_injector() {
        let mut headers = HeaderMap::new();
        let mut injector = HeaderInjector {
            headers: &mut headers,
        };

        // Test injecting trace context headers
        injector.set("traceparent", "00-trace-id-span-id-01".to_string());
        injector.set("tracestate", "test=value".to_string());

        assert_eq!(
            headers.get("traceparent").unwrap().to_str().unwrap(),
            "00-trace-id-span-id-01"
        );
        assert_eq!(
            headers.get("tracestate").unwrap().to_str().unwrap(),
            "test=value"
        );
    }

    #[test]
    fn test_header_injector_handles_invalid_headers() {
        let mut headers = HeaderMap::new();
        let mut injector = HeaderInjector {
            headers: &mut headers,
        };

        // Test injecting headers with invalid characters (should be skipped)
        injector.set("invalid\nheader", "value".to_string());

        // Invalid header names should not be added
        assert!(headers.get("invalid\nheader").is_none());
    }

    #[tokio::test]
    async fn test_send_request_injects_trace_context() {
        // Set up OpenTelemetry tracer
        let resource = Resource::new(vec![KeyValue::new("service.name", "test")]);
        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let tracer = provider.tracer("test");

        // Set up tracing subscriber with OpenTelemetry layer
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry_layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        // Set W3C TraceContext propagator
        crate::tracing::init_otel_propagation();

        // Create API client
        let client = ApiClient::new(
            "http://localhost:8080".to_string(),
            AuthMethod::BearerToken("test-token".to_string()),
        )
        .unwrap();

        // Create a test function with a span
        #[instrument]
        async fn make_request_with_span(client: &ApiClient) -> Result<HeaderMap> {
            // Build a request
            let builder = client.request("/test");
            let request = builder.send_request(|url, client| client.get(url)).await?;

            // Extract headers from the request
            let headers = request.build().unwrap().headers().clone();
            Ok(headers)
        }

        // Call within a span
        let headers = make_request_with_span(&client).await.unwrap();

        // Verify trace context headers were injected
        assert!(
            headers.contains_key("traceparent"),
            "traceparent header should be present"
        );

        // Verify the traceparent format (00-<trace-id>-<span-id>-<flags>)
        let traceparent = headers.get("traceparent").unwrap().to_str().unwrap();
        assert!(
            traceparent.starts_with("00-"),
            "traceparent should start with version 00"
        );
        let parts: Vec<&str> = traceparent.split('-').collect();
        assert_eq!(parts.len(), 4, "traceparent should have 4 parts");
        assert_eq!(parts[0], "00", "version should be 00");
        assert_eq!(parts[1].len(), 32, "trace-id should be 32 hex chars");
        assert_eq!(parts[2].len(), 16, "span-id should be 16 hex chars");
        assert_eq!(parts[3].len(), 2, "flags should be 2 hex chars");
    }

    #[tokio::test]
    async fn test_send_request_without_span() {
        // Create API client without setting up tracing
        let client = ApiClient::new(
            "http://localhost:8080".to_string(),
            AuthMethod::BearerToken("test-token".to_string()),
        )
        .unwrap();

        // Build a request without an active span
        let builder = client.request("/test");
        let request = builder
            .send_request(|url, client| client.get(url))
            .await
            .unwrap();

        // Extract headers from the request
        let built_request = request.build().unwrap();
        let headers = built_request.headers();

        // Verify that authorization header is still present
        assert!(headers.contains_key("authorization"));

        // Note: Without an active span, traceparent may or may not be present
        // depending on if there's a default/empty context
    }
}
