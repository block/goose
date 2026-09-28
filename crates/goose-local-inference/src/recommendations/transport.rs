use anyhow::{ensure, Result};
use futures::StreamExt;
use reqwest::{Client, Url};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const JSON_LIMIT: usize = 16 * 1024 * 1024;
const CACHE_LIMIT: usize = 32 * 1024 * 1024;
type JsonCache = HashMap<(Option<String>, String), (Instant, usize, Value)>;
static JSON_CACHE: OnceLock<Mutex<JsonCache>> = OnceLock::new();

#[derive(Clone)]
pub(super) struct Hub {
    client: Client,
    token: Option<String>,
}

impl Hub {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("goose-local-recommendations")
                .timeout(Duration::from_secs(25))
                .build()?,
            token: crate::huggingface_auth::resolve_token_async()
                .await
                .ok()
                .flatten(),
        })
    }

    pub fn url(path: &str, query: &[(&str, &str)]) -> Result<Url> {
        let mut url = Url::parse("https://huggingface.co/")?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("Invalid Hub URL"))?;
            segments.clear();
            for segment in path.split('/') {
                ensure!(
                    !segment.is_empty() && segment != "." && segment != "..",
                    "Invalid Hub path"
                );
                segments.push(segment);
            }
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        Ok(url)
    }

    fn request(&self, url: Url) -> reqwest::RequestBuilder {
        let request = self.client.get(url);
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
        ensure!(
            response
                .content_length()
                .is_none_or(|size| size <= limit as u64),
            "Metadata exceeds download limit"
        );
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            ensure!(
                body.len().saturating_add(chunk.len()) <= limit,
                "Metadata exceeds download limit"
            );
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    pub async fn json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        let url = Self::url(path, query)?;
        let key = (self.token.clone(), url.to_string());
        let cache = JSON_CACHE.get_or_init(Default::default);
        if let Some((time, _, value)) = cache.lock().await.get(&key) {
            if time.elapsed() < Duration::from_secs(600) {
                return Ok(value.clone());
            }
        }
        let response = self.request(url).send().await?.error_for_status()?;
        let bytes = Self::bounded_body(response, JSON_LIMIT).await?;
        let value: Value = serde_json::from_slice(&bytes)?;
        let mut cache = cache.lock().await;
        cache.retain(|_, (time, _, _)| time.elapsed() < Duration::from_secs(600));
        if cache.values().map(|(_, size, _)| size).sum::<usize>() + bytes.len() > CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), bytes.len(), value.clone()));
        Ok(value)
    }

    pub async fn model(&self, repo: &str) -> Result<Value> {
        ensure!(
            repo.split('/').count() == 2,
            "Expected a Hugging Face owner/model ID"
        );
        self.json(&format!("api/models/{repo}"), &[("blobs", "true")])
            .await
    }

    #[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
    pub async fn file(&self, repo: &str, revision: &str, member: &str) -> Result<Vec<u8>> {
        let url = Self::url(&format!("{repo}/resolve/{revision}/{member}"), &[])?;
        let response = self.request(url).send().await?.error_for_status()?;
        Self::bounded_body(response, JSON_LIMIT).await
    }

    #[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
    pub async fn range(
        &self,
        repo: &str,
        revision: &str,
        member: &str,
        start: u64,
        end: u64,
    ) -> Result<(Vec<u8>, u64)> {
        use reqwest::header;
        let url = Self::url(&format!("{repo}/resolve/{revision}/{member}"), &[])?;
        let response = self
            .request(url)
            .header(header::RANGE, format!("bytes={start}-{end}"))
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await?
            .error_for_status()?;
        // Reject an ignored Range before consuming a potentially multi-GB body.
        ensure!(
            response.status() == reqwest::StatusCode::PARTIAL_CONTENT,
            "Server did not honor metadata range request"
        );
        let range = response
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let Some((positions, total)) = range.strip_prefix("bytes ").and_then(|s| s.split_once('/'))
        else {
            anyhow::bail!("Missing content range for metadata");
        };
        let (actual_start, actual_end) = positions
            .split_once('-')
            .ok_or_else(|| anyhow::anyhow!("Invalid content range"))?;
        let (actual_start, actual_end, total) = (
            actual_start.parse::<u64>()?,
            actual_end.parse::<u64>()?,
            total.parse::<u64>()?,
        );
        ensure!(
            actual_start == start
                && actual_end == end.min(total.saturating_sub(1))
                && actual_end >= start
                && actual_end < total,
            "Unexpected metadata range"
        );
        let length = usize::try_from(actual_end - start + 1)?;
        ensure!(length <= JSON_LIMIT, "Metadata exceeds download limit");
        let body = Self::bounded_body(response, length).await?;
        ensure!(body.len() == length, "Truncated metadata range");
        Ok((body, total))
    }
}
