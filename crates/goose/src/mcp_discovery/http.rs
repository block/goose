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

    /// Direct-handshake probe for the fallback step: returns true only if the
    /// endpoint responds like an MCP Streamable HTTP server, false otherwise.
    async fn probe(&self, url: &str) -> anyhow::Result<bool>;
}

/// Minimal MCP `initialize` request used to probe an endpoint in the fallback
/// step. A non-MCP host answering on `/mcp` (a 404 page, an SPA, a login form)
/// will not produce a JSON-RPC / SSE response, so it is not mistaken for a
/// server.
const MCP_INITIALIZE_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"goose-mcp-discovery","version":"0"}}}"#;

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
        let resp = match self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .body(MCP_INITIALIZE_BODY)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) if e.is_connect() || e.is_timeout() => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        if !resp.status().is_success() {
            return Ok(false);
        }
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        Ok(content_type.contains("application/json") || content_type.contains("text/event-stream"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fetcher() -> ReqwestFetcher {
        ReqwestFetcher::new(Duration::from_secs(5)).unwrap()
    }

    #[tokio::test]
    async fn get_returns_found_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/mcp-server"))
            .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
            .mount(&server)
            .await;
        let url = format!("{}/.well-known/mcp-server", server.uri());
        match fetcher().get(&url).await.unwrap() {
            FetchOutcome::Found { body } => assert_eq!(body, "hello"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_returns_notfound_on_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let url = format!("{}/.well-known/mcp-server", server.uri());
        assert!(matches!(
            fetcher().get(&url).await.unwrap(),
            FetchOutcome::NotFound
        ));
    }

    #[tokio::test]
    async fn get_errors_on_500() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let url = format!("{}/.well-known/mcp-server", server.uri());
        assert!(fetcher().get(&url).await.is_err());
    }

    #[tokio::test]
    async fn probe_accepts_mcp_json_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"jsonrpc":"2.0","id":1,"result":{}})),
            )
            .mount(&server)
            .await;
        let url = format!("{}/mcp", server.uri());
        assert!(fetcher().probe(&url).await.unwrap());
    }

    #[tokio::test]
    async fn probe_rejects_non_mcp_html_page() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_string("<html>not mcp</html>"),
            )
            .mount(&server)
            .await;
        let url = format!("{}/mcp", server.uri());
        assert!(!fetcher().probe(&url).await.unwrap());
    }

    #[tokio::test]
    async fn probe_rejects_404() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let url = format!("{}/mcp", server.uri());
        assert!(!fetcher().probe(&url).await.unwrap());
    }
}
