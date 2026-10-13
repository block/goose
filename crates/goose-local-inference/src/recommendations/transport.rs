#[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
use anyhow::Context;
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
    pub catalogue: bool,
    cache_dir: Option<std::path::PathBuf>,
    retry_at: std::sync::Arc<Mutex<Instant>>,
}

impl Hub {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("goose-local-recommendations")
                .timeout(Duration::from_secs(25))
                .build()?,
            catalogue: false,
            cache_dir: None,
            retry_at: std::sync::Arc::new(Mutex::new(Instant::now())),
            token: crate::huggingface_auth::resolve_token_async()
                .await
                .ok()
                .flatten(),
        })
    }

    #[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
    pub fn public(cache_dir: Option<std::path::PathBuf>) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("goose-local-catalogue")
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(120))
                .build()?,
            token: None,
            catalogue: true,
            cache_dir,
            retry_at: std::sync::Arc::new(Mutex::new(Instant::now())),
        })
    }

    #[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
    pub async fn pages(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<Value>> {
        let key = format!("pages:{}", Self::url(path, query)?);
        if let Some(bytes) = self.cached_bytes(&key) {
            return Ok(serde_json::from_slice(&bytes)?);
        }
        let mut next = Some(Self::url(path, query)?);
        let mut seen = std::collections::HashSet::new();
        let mut rows = Vec::new();
        while let Some(url) = next {
            ensure!(
                url.scheme() == "https" && url.host_str() == Some("huggingface.co"),
                "Invalid HF pagination origin"
            );
            ensure!(
                seen.insert(url.to_string()),
                "Repeated HF pagination cursor"
            );
            let response = self.send(self.request(url)).await?;
            next = response
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|header| header.to_str().ok())
                .and_then(|links| links.split(',').find(|link| link.contains("rel=\"next\"")))
                .and_then(|link| link.trim().strip_prefix('<'))
                .and_then(|link| link.split_once('>').map(|(url, _)| url))
                .map(Url::parse)
                .transpose()?;
            let bytes = Self::bounded_body(response, JSON_LIMIT).await?;
            let page: Value = serde_json::from_slice(&bytes)?;
            rows.extend(
                page.as_array()
                    .ok_or_else(|| anyhow::anyhow!("Expected HF result array"))?
                    .iter()
                    .cloned(),
            );
        }
        self.cache_bytes(&key, &serde_json::to_vec(&rows)?)?;
        Ok(rows)
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

    fn cache_path(&self, key: &str) -> Option<std::path::PathBuf> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hash);
        self.cache_dir
            .as_ref()
            .map(|dir| dir.join(format!("{:016x}", hash.finish())))
    }

    fn cached_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(key)?;
        let age = std::fs::metadata(&path)
            .ok()?
            .modified()
            .ok()?
            .elapsed()
            .ok()?;
        (age < Duration::from_secs(24 * 60 * 60))
            .then(|| std::fs::read(path).ok())
            .flatten()
    }

    fn cache_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        // Large tensor headers are cheap to discard after inspection; retaining all
        // quantizations would exhaust the disk on a standard release runner.
        if key.starts_with("range:") && bytes.len() > 512 * 1024 {
            return Ok(());
        }
        if let Some(path) = self.cache_path(key) {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let temporary = path.with_extension(uuid::Uuid::new_v4().to_string());
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(temporary, path)?;
        }
        Ok(())
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        for attempt in 0..6 {
            if self.catalogue {
                loop {
                    let delay = self
                        .retry_at
                        .lock()
                        .await
                        .saturating_duration_since(Instant::now());
                    if delay.is_zero() {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                }
            }
            let response = request
                .try_clone()
                .expect("GET requests are cloneable")
                .send()
                .await;
            if self.catalogue && attempt < 5 {
                if let Ok(response) = &response {
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let delay = rate_limit_delay(response.headers());
                        eprintln!("HF rate limit: resuming metadata requests in {delay} seconds");
                        let mut retry_at = self.retry_at.lock().await;
                        *retry_at = (*retry_at).max(Instant::now() + Duration::from_secs(delay));
                        continue;
                    }
                }
            }
            match response.and_then(reqwest::Response::error_for_status) {
                Err(error)
                    if self.catalogue
                        && attempt < 5
                        && error.status().is_none_or(|status| status.is_server_error()) =>
                {
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt + 1))).await;
                }
                result => return Ok(result?),
            }
        }
        unreachable!()
    }

    pub(super) async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
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
        let disk_key = format!("json:{url}");
        if let Some(bytes) = self.cached_bytes(&disk_key) {
            return Ok(serde_json::from_slice(&bytes)?);
        }
        let key = (self.token.clone(), url.to_string());
        let cache = JSON_CACHE.get_or_init(Default::default);
        if let Some((time, _, value)) = cache.lock().await.get(&key) {
            if time.elapsed() < Duration::from_secs(600) {
                return Ok(value.clone());
            }
        }
        let response = self.send(self.request(url)).await?;
        let bytes = Self::bounded_body(response, JSON_LIMIT).await?;
        let value: Value = serde_json::from_slice(&bytes)?;
        self.cache_bytes(&disk_key, &bytes)?;
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
        let key = format!("file:{url}");
        if let Some(bytes) = self.cached_bytes(&key) {
            return Ok(bytes);
        }
        let response = self.send(self.request(url)).await?;
        let bytes = Self::bounded_body(response, JSON_LIMIT).await?;
        self.cache_bytes(&key, &bytes)?;
        Ok(bytes)
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
        let key = format!("range:{url}:{start}:{end}");
        if let Some(bytes) = self.cached_bytes(&key) {
            ensure!(bytes.len() >= 8, "Truncated metadata cache");
            let total = u64::from_le_bytes(bytes[..8].try_into()?);
            return Ok((bytes[8..].to_vec(), total));
        }
        let response = self
            .send(
                self.request(url)
                    .header(header::RANGE, format!("bytes={start}-{end}"))
                    .header(header::ACCEPT_ENCODING, "identity"),
            )
            .await?;
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
        let body = Self::bounded_body(response, length)
            .await
            .with_context(|| {
                format!("Reading metadata range {repo}@{revision}/{member} bytes {start}-{end}")
            })?;
        ensure!(body.len() == length, "Truncated metadata range");
        let mut cached = total.to_le_bytes().to_vec();
        cached.extend_from_slice(&body);
        self.cache_bytes(&key, &cached)?;
        Ok((body, total))
    }
}

fn rate_limit_delay(headers: &reqwest::header::HeaderMap) -> u64 {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            headers
                .get("ratelimit")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    value
                        .split(';')
                        .find_map(|part| part.trim().strip_prefix("t=")?.parse::<u64>().ok())
                })
        })
        .unwrap_or(60)
        .saturating_add(1)
        .clamp(1, 600)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_limit_backoff_honors_retry_after_and_hf_window() {
        let mut headers = reqwest::header::HeaderMap::new();
        assert_eq!(rate_limit_delay(&headers), 61);
        headers.insert("ratelimit", "\"api\";r=0;t=278".parse().unwrap());
        assert_eq!(rate_limit_delay(&headers), 279);
        headers.insert(reqwest::header::RETRY_AFTER, "12".parse().unwrap());
        assert_eq!(rate_limit_delay(&headers), 13);
    }
}
