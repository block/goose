use super::*;

pub(super) const OPEN_AI_PROVIDER_NAME: &str = "openai";
pub(super) const OPEN_AI_DEFAULT_BASE_PATH: &str = "v1/chat/completions";
pub(super) const OPEN_AI_VERSIONLESS_BASE_PATH: &str = "chat/completions";
pub(super) const OPEN_AI_DEFAULT_RESPONSES_PATH: &str = "v1/responses";
pub(super) const OPEN_AI_DEFAULT_MODELS_PATH: &str = "v1/models";
pub(super) const OPEN_AI_DEFAULT_EMBEDDINGS_PATH: &str = "v1/embeddings";
pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const OPEN_AI_DEFAULT_FAST_MODEL: &str = "gpt-4o-mini";
pub const OPEN_AI_KNOWN_MODELS: &[(&str, usize)] = &[
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4.1", 128_000),
    ("gpt-4.1-mini", 128_000),
    ("o1", 200_000),
    ("o3", 200_000),
    ("gpt-3.5-turbo", 16_385),
    ("gpt-4-turbo", 128_000),
    ("o4-mini", 128_000),
    ("gpt-5", 400_000),
    ("gpt-5-mini", 400_000),
    ("gpt-5-nano", 400_000),
    ("gpt-5-pro", 400_000),
    ("gpt-5-codex", 400_000),
    ("gpt-5.1", 400_000),
    ("gpt-5.1-codex", 400_000),
    ("gpt-5.2", 400_000),
    ("gpt-5.2-codex", 400_000),
    ("gpt-5.2-pro", 400_000),
    ("gpt-5.3-codex", 400_000),
    ("gpt-5.4", 1_050_000),
    ("gpt-5.4-mini", 400_000),
    ("gpt-5.4-nano", 400_000),
    ("gpt-5.4-pro", 1_050_000),
];

pub const OPEN_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";

pub(super) type OpenAiBaseUrlParts = (String, Vec<(String, String)>, bool);

/// Components extracted from an `OPENAI_BASE_URL` value.
pub(super) struct ParsedBaseUrl {
    /// The host (scheme + authority + any path prefix before `/v1`).
    pub host: String,
    /// Query parameters to forward on every request.
    pub query_params: Vec<(String, String)>,
    /// Whether the URL path ended with `/v1`.
    pub has_v1: bool,
    /// `true` when the host was derived from `OPENAI_BASE_URL`.
    /// Controls whether `OPENAI_BASE_PATH` is read from env only
    /// (to avoid persisted desktop defaults shadowing URL-derived paths)
    /// or from config too (to honour Docker Model Runner setups).
    pub from_base_url: bool,
}

pub(crate) fn parse_openai_base_url(raw_url: &str) -> Result<OpenAiBaseUrlParts> {
    let parsed = url::Url::parse(raw_url)
        .map_err(|e| anyhow::anyhow!("Invalid OPENAI_BASE_URL '{}': {}", raw_url, e))?;

    let authority = parsed[..url::Position::BeforePath].to_string();
    let query_params: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() || path == "/" {
        return Ok((authority, query_params, true));
    }

    if path == "/v1" {
        return Ok((authority, query_params, true));
    }
    if let Some(prefix) = path.strip_suffix("/v1") {
        return Ok((format!("{}{}", authority, prefix), query_params, true));
    }

    Ok((format!("{}{}", authority, path), query_params, false))
}

pub(super) fn parse_custom_headers(s: String) -> HashMap<String, String> {
    s.split(',')
        .filter_map(|header| {
            let mut parts = header.splitn(2, '=');
            let key = parts.next().map(|s| s.trim().to_string())?;
            let value = parts.next().map(|s| s.trim().to_string())?;
            Some((key, value))
        })
        .collect()
}
