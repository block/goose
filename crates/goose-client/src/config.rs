use std::time::Duration;

const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;

pub struct GooseClientConfig {
    pub(crate) base_url: String,
    pub(crate) secret_key: String,
    pub(crate) accept_invalid_certs: bool,
    pub(crate) connect_timeout: Duration,
}

impl std::fmt::Debug for GooseClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GooseClientConfig")
            .field("base_url", &self.base_url)
            .field("secret_key", &"[REDACTED]")
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .field("connect_timeout", &self.connect_timeout)
            .finish()
    }
}

impl GooseClientConfig {
    /// Defaults to accepting self-signed TLS certificates since goose-server
    /// generates a fresh self-signed cert on every startup. Call
    /// `.accept_invalid_certs(false)` when connecting to a remote server
    /// with a CA-signed certificate.
    pub fn new(base_url: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            secret_key: secret_key.into(),
            accept_invalid_certs: true,
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
        }
    }

    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}
