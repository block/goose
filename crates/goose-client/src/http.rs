use crate::config::GooseClientConfig;
use crate::error::{GooseClientError, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub(crate) struct HttpClient {
    pub(crate) inner: reqwest::Client,
    pub(crate) base_url: String,
}

impl HttpClient {
    pub(crate) fn new(config: &GooseClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(&config.secret_key).map_err(|e| {
            GooseClientError::Config(format!("invalid secret key header value: {e}"))
        })?;
        auth_value.set_sensitive(true);
        headers.insert("X-Secret-Key", auth_value);

        let inner = reqwest::Client::builder()
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .connect_timeout(config.connect_timeout)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            inner,
            base_url: config.base_url.trim_end_matches('/').to_string(),
        })
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub(crate) async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let resp = self.inner.get(self.url(path)).send().await?;
        self.parse(resp).await
    }

    pub(crate) async fn get_with_query<R: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<R> {
        let resp = self.inner.get(self.url(path)).query(query).send().await?;
        self.parse(resp).await
    }

    pub(crate) async fn get_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let resp = self.inner.get(self.url(path)).send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GooseClientError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = self.extract_error_message(resp).await;
            return Err(GooseClientError::Server { status, message });
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub(crate) async fn get_text(&self, path: &str) -> Result<String> {
        let resp = self.inner.get(self.url(path)).send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GooseClientError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = self.extract_error_message(resp).await;
            return Err(GooseClientError::Server { status, message });
        }
        Ok(resp.text().await?)
    }

    pub(crate) async fn post<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R> {
        let resp = self.inner.post(self.url(path)).json(body).send().await?;
        self.parse(resp).await
    }

    pub(crate) async fn post_empty<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let resp = self.inner.post(self.url(path)).json(body).send().await?;
        self.check_status(resp).await
    }

    pub(crate) async fn put_empty<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let resp = self.inner.put(self.url(path)).json(body).send().await?;
        self.check_status(resp).await
    }

    pub(crate) async fn put<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R> {
        let resp = self.inner.put(self.url(path)).json(body).send().await?;
        self.parse(resp).await
    }

    pub(crate) async fn delete(&self, path: &str) -> Result<()> {
        let resp = self.inner.delete(self.url(path)).send().await?;
        self.check_status(resp).await
    }

    pub(crate) async fn post_streaming<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<reqwest::Response> {
        let resp = self.inner.post(self.url(path)).json(body).send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GooseClientError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(GooseClientError::Server { status, message });
        }
        Ok(resp)
    }

    async fn parse<R: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<R> {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GooseClientError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = self.extract_error_message(resp).await;
            return Err(GooseClientError::Server { status, message });
        }
        let bytes = resp.bytes().await?;
        serde_json::from_slice(&bytes).map_err(GooseClientError::Deserialization)
    }

    async fn check_status(&self, resp: reqwest::Response) -> Result<()> {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GooseClientError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = self.extract_error_message(resp).await;
            return Err(GooseClientError::Server { status, message });
        }
        Ok(())
    }

    async fn extract_error_message(&self, resp: reqwest::Response) -> String {
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("message")?.as_str().map(String::from))
            .unwrap_or(text)
    }
}
