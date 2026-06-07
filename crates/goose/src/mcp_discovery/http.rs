use std::time::Duration;

use async_trait::async_trait;

/// Result of fetching a manifest URL.
#[derive(Debug, Clone)]
pub enum FetchOutcome {
    Found { body: String },
    NotFound,
}

/// Abstraction over the HTTP calls discovery makes, so resolution can be unit
/// tested without a live server.
#[async_trait]
pub trait ManifestFetcher: Send + Sync {
    /// GET a URL. `Found` on a 2xx response, `NotFound` on 404; any other status
    /// or transport failure is an error.
    async fn get(&self, url: &str) -> anyhow::Result<FetchOutcome>;

    /// Reachability probe for the direct-handshake fallback: returns true if the
    /// endpoint answers at all (any HTTP status), false if the connection fails.
    async fn probe(&self, url: &str) -> anyhow::Result<bool>;
}

/// `ManifestFetcher` backed by reqwest.
pub struct ReqwestFetcher {
    client: reqwest::Client,
}

impl ReqwestFetcher {
    pub fn new(timeout: Duration) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(concat!("goose-mcp-discovery/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ManifestFetcher for ReqwestFetcher {
    async fn get(&self, url: &str) -> anyhow::Result<FetchOutcome> {
        let resp = self.client.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(FetchOutcome::NotFound);
        }
        if !resp.status().is_success() {
            anyhow::bail!("unexpected status {} from {url}", resp.status());
        }
        Ok(FetchOutcome::Found {
            body: resp.text().await?,
        })
    }

    async fn probe(&self, url: &str) -> anyhow::Result<bool> {
        match self.client.get(url).send().await {
            Ok(_) => Ok(true),
            Err(e) if e.is_connect() || e.is_timeout() => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}
