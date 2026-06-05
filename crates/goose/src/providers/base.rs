use super::canonical::{map_to_canonical_model, CanonicalModelRegistry, Modality};
use super::errors::ProviderError;
use super::retry::RetryConfig;
use crate::config::base::ConfigValue;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::permission::PermissionConfirmation;
use crate::utils::safe_truncate;
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use once_cell::sync::Lazy;
use regex::Regex;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::Mutex;
use utoipa::ToSchema;

pub use super::provider_def::ProviderDef;

pub const DEFAULT_PROVIDER_TIMEOUT_SECS: u64 = 600;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FilterOut {
    pub content: String,
    pub thinking: String,
}

pub struct ThinkFilter {
    buffer: String,
    inside_think: bool,
    think_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThinkTag {
    Open,
    Close,
    SelfClosing,
}

enum BufferEvent {
    Tag {
        pos: usize,
        end: usize,
        kind: ThinkTag,
    },
    Partial(usize),
}

impl ThinkFilter {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            inside_think: false,
            think_depth: 0,
        }
    }

    pub fn push(&mut self, chunk: &str) -> FilterOut {
        self.buffer.push_str(chunk);
        self.process_buffer()
    }

    pub fn finish(mut self) -> FilterOut {
        let mut out = self.process_buffer();
        if !self.buffer.is_empty() {
            if self.inside_think {
                out.thinking.push_str(&self.buffer);
            } else {
                out.content.push_str(&self.buffer);
            }
            self.buffer.clear();
        }
        out
    }

    fn process_buffer(&mut self) -> FilterOut {
        let mut out = FilterOut::default();

        loop {
            match next_buffer_event(&self.buffer, self.inside_think) {
                Some(BufferEvent::Tag { pos, end, kind }) => {
                    if pos > 0 {
                        let prefix = self.buffer.get(..pos).unwrap_or_default().to_string();
                        if self.inside_think {
                            out.thinking.push_str(&prefix);
                        } else {
                            out.content.push_str(&prefix);
                        }
                    }

                    self.buffer.drain(..end);

                    match kind {
                        ThinkTag::Open => {
                            self.think_depth += 1;
                            self.inside_think = true;
                        }
                        ThinkTag::Close => {
                            self.think_depth = self.think_depth.saturating_sub(1);
                            self.inside_think = self.think_depth > 0;
                        }
                        ThinkTag::SelfClosing => {}
                    }
                }
                Some(BufferEvent::Partial(pos)) => {
                    if pos > 0 {
                        let prefix = self.buffer.get(..pos).unwrap_or_default().to_string();
                        if self.inside_think {
                            out.thinking.push_str(&prefix);
                        } else {
                            out.content.push_str(&prefix);
                        }
                        self.buffer.drain(..pos);
                    }
                    break;
                }
                None => {
                    if !self.buffer.is_empty() {
                        if self.inside_think {
                            out.thinking.push_str(&self.buffer);
                        } else {
                            out.content.push_str(&self.buffer);
                        }
                        self.buffer.clear();
                    }
                    break;
                }
            }
        }

        out
    }
}

impl Default for ThinkFilter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn split_think_blocks(text: &str) -> (String, String) {
    let mut filter = ThinkFilter::new();
    let mut out = filter.push(text);
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);
    (out.content, out.thinking)
}

fn next_buffer_event(buffer: &str, inside_think: bool) -> Option<BufferEvent> {
    let mut search_from = 0;

    while let Some(rel_pos) = buffer.get(search_from..).and_then(|rest| rest.find('<')) {
        let pos = search_from + rel_pos;
        let suffix = buffer.get(pos..).unwrap_or_default();

        if let Some((kind, end)) = parse_think_tag(buffer, pos) {
            if inside_think || matches!(kind, ThinkTag::Open | ThinkTag::SelfClosing) {
                return Some(BufferEvent::Tag { pos, end, kind });
            }
        } else if !contains_unquoted_gt(suffix) && is_possible_partial_think_tag(suffix) {
            return Some(BufferEvent::Partial(pos));
        }

        search_from = pos + 1;
    }

    None
}

fn parse_think_tag(buffer: &str, start: usize) -> Option<(ThinkTag, usize)> {
    let bytes = buffer.as_bytes();
    if bytes.get(start) != Some(&b'<') {
        return None;
    }

    let mut idx = start + 1;
    let is_close = if bytes.get(idx) == Some(&b'/') {
        idx += 1;
        true
    } else {
        false
    };

    let name_start = idx;
    while bytes.get(idx).is_some_and(u8::is_ascii_alphabetic) {
        idx += 1;
    }

    if idx == name_start {
        return None;
    }

    let name = buffer.get(name_start..idx).unwrap_or_default();
    let is_think = name.eq_ignore_ascii_case("think") || name.eq_ignore_ascii_case("thinking");
    if !is_think {
        return None;
    }

    if is_close {
        while bytes.get(idx).is_some_and(u8::is_ascii_whitespace) {
            idx += 1;
        }
        if bytes.get(idx) == Some(&b'>') {
            return Some((ThinkTag::Close, idx + 1));
        }
        return None;
    }

    let valid_open_boundary = match bytes.get(idx) {
        Some(&b) => b == b'>' || b == b'/' || b.is_ascii_whitespace(),
        None => false,
    };
    if !valid_open_boundary {
        return None;
    }

    let mut quote: Option<u8> = None;
    let mut last_non_ws: Option<u8> = None;
    while let Some(&byte) = bytes.get(idx) {
        match quote {
            Some(quote_byte) => {
                if byte == quote_byte {
                    quote = None;
                }
            }
            None if matches!(byte, b'"' | b'\'') => {
                quote = Some(byte);
                last_non_ws = Some(byte);
            }
            None if byte == b'>' => {
                let kind = if last_non_ws == Some(b'/') {
                    ThinkTag::SelfClosing
                } else {
                    ThinkTag::Open
                };
                return Some((kind, idx + 1));
            }
            None if !byte.is_ascii_whitespace() => {
                last_non_ws = Some(byte);
            }
            None => {}
        }
        idx += 1;
    }

    None
}

fn is_possible_partial_think_tag(suffix: &str) -> bool {
    if contains_unquoted_gt(suffix) {
        return false;
    }

    static OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)^<(?:t(?:h(?:i(?:n(?:k(?:i(?:n(?:g)?)?)?)?)?)?)?)(?:\s.*|/)?$").unwrap()
    });
    static CLOSE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)^</(?:t(?:h(?:i(?:n(?:k(?:i(?:n(?:g)?)?)?)?)?)?)?)(?:\s*)?$").unwrap()
    });

    OPEN_RE.is_match(suffix) || CLOSE_RE.is_match(suffix)
}

fn contains_unquoted_gt(text: &str) -> bool {
    let mut quote: Option<u8> = None;
    for &byte in text.as_bytes() {
        match quote {
            Some(quote_byte) => {
                if byte == quote_byte {
                    quote = None;
                }
            }
            None if matches!(byte, b'"' | b'\'') => quote = Some(byte),
            None if byte == b'>' => return true,
            None => {}
        }
    }
    false
}

fn strip_xml_tags(text: &str) -> String {
    static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<([a-zA-Z][a-zA-Z0-9_]*)[^>]*>.*?</[a-zA-Z][a-zA-Z0-9_]*>").unwrap()
    });
    static TAG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"</?[a-zA-Z][a-zA-Z0-9_]*[^>]*>").unwrap());
    let pass1 = BLOCK_RE.replace_all(text, "");
    TAG_RE.replace_all(&pass1, "").into_owned()
}

fn extract_short_title(text: &str) -> String {
    let word_count = text.split_whitespace().count();
    if word_count <= 8 {
        return text.to_string();
    }

    {
        let mut results = Vec::new();
        let mut quote_char: Option<char> = None;
        let mut current = String::new();
        let mut prev_char: Option<char> = None;

        for ch in text.chars() {
            match quote_char {
                None => {
                    if matches!(ch, '"' | '\'' | '`') {
                        let after_alnum = prev_char.map(|p| p.is_alphanumeric()).unwrap_or(false);
                        if !after_alnum {
                            quote_char = Some(ch);
                            current.clear();
                        }
                    }
                }
                Some(q) => {
                    if ch == q {
                        let trimmed = current.trim().to_string();
                        let wc = trimmed.split_whitespace().count();
                        if (2..=8).contains(&wc) {
                            results.push(trimmed);
                        }
                        quote_char = None;
                        current.clear();
                    } else {
                        current.push(ch);
                    }
                }
            }
            prev_char = Some(ch);
        }

        if let Some(title) = results.last() {
            return title.clone();
        }
    }

    if let Some(last) = text.lines().rev().find(|line| !line.trim().is_empty()) {
        return last.trim().to_string();
    }

    text.to_string()
}

pub static CURRENT_MODEL: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn set_current_model(model: &str) {
    if let Ok(mut current_model) = CURRENT_MODEL.lock() {
        *current_model = Some(model.to_string());
    }
}

pub fn get_current_model() -> Option<String> {
    CURRENT_MODEL.lock().ok().and_then(|model| model.clone())
}

pub static MSG_COUNT_FOR_SESSION_NAME_GENERATION: usize = 3;

/// Information about a model's capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct ModelInfo {
    /// The name of the model
    pub name: String,
    /// The underlying model resolved from provider metadata, when the configured model is an alias or endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    /// The maximum context length this model supports
    pub context_limit: usize,
    /// Cost per token for input in USD (optional)
    pub input_token_cost: Option<f64>,
    /// Cost per token for output in USD (optional)
    pub output_token_cost: Option<f64>,
    /// Currency for the costs (default: "$")
    pub currency: Option<String>,
    /// Whether this model supports cache control
    pub supports_cache_control: Option<bool>,
    /// Whether this model supports reasoning/thinking controls
    #[serde(default)]
    pub reasoning: bool,
}

impl ModelInfo {
    pub fn new(name: impl Into<String>, context_limit: usize) -> Self {
        Self {
            name: name.into(),
            resolved_model: None,
            context_limit,
            input_token_cost: None,
            output_token_cost: None,
            currency: None,
            supports_cache_control: None,
            reasoning: false,
        }
    }

    pub fn with_cost(
        name: impl Into<String>,
        context_limit: usize,
        input_cost: f64,
        output_cost: f64,
    ) -> Self {
        Self {
            name: name.into(),
            resolved_model: None,
            context_limit,
            input_token_cost: Some(input_cost),
            output_token_cost: Some(output_cost),
            currency: Some("$".to_string()),
            supports_cache_control: None,
            reasoning: false,
        }
    }
}

fn model_info_for_provider_model(provider_name: &str, model_name: &str) -> ModelInfo {
    let registry = CanonicalModelRegistry::bundled().ok();
    let canonical = registry.as_ref().and_then(|registry| {
        let canonical_id = map_to_canonical_model(provider_name, model_name, registry)?;
        let (provider, model) = canonical_id.split_once('/')?;
        registry.get(provider, model)
    });

    let reasoning = canonical
        .as_ref()
        .and_then(|model| model.reasoning)
        .unwrap_or_else(|| ModelConfig::new_or_fail(model_name).is_reasoning_model());

    ModelInfo {
        name: model_name.to_string(),
        resolved_model: None,
        context_limit: ModelConfig::new_or_fail(model_name)
            .with_canonical_limits(provider_name)
            .context_limit(),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProviderType {
    Preferred,
    Builtin,
    Declarative,
    Custom,
}

/// Metadata about a provider's configuration requirements and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderMetadata {
    /// The unique identifier for this provider
    pub name: String,
    /// Display name for the provider in UIs
    pub display_name: String,
    /// Description of the provider's capabilities
    pub description: String,
    /// The default/recommended model for this provider
    pub default_model: String,
    /// A list of currently known models with their capabilities
    pub known_models: Vec<ModelInfo>,
    /// Link to the docs where models can be found
    pub model_doc_link: String,
    /// Required configuration keys
    pub config_keys: Vec<ConfigKey>,
    /// step-by-step instructions for set up providers eg: api key
    #[serde(default)]
    pub setup_steps: Vec<String>,
    /// Hint shown in the model picker when this provider manages its own model selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_selection_hint: Option<String>,
}

impl ProviderMetadata {
    pub fn new(
        name: &str,
        display_name: &str,
        description: &str,
        default_model: &str,
        model_names: Vec<&str>,
        model_doc_link: &str,
        config_keys: Vec<ConfigKey>,
    ) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            default_model: default_model.to_string(),
            known_models: model_names
                .iter()
                .map(|&model_name| model_info_for_provider_model(name, model_name))
                .collect(),
            model_doc_link: model_doc_link.to_string(),
            config_keys,
            setup_steps: vec![],
            model_selection_hint: None,
        }
    }

    pub fn with_models(
        name: &str,
        display_name: &str,
        description: &str,
        default_model: &str,
        models: Vec<ModelInfo>,
        model_doc_link: &str,
        config_keys: Vec<ConfigKey>,
    ) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            default_model: default_model.to_string(),
            known_models: models,
            model_doc_link: model_doc_link.to_string(),
            config_keys,
            setup_steps: vec![],
            model_selection_hint: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            name: "".to_string(),
            display_name: "".to_string(),
            description: "".to_string(),
            default_model: "".to_string(),
            known_models: vec![],
            model_doc_link: "".to_string(),
            config_keys: vec![],
            setup_steps: vec![],
            model_selection_hint: None,
        }
    }

    pub fn with_setup_steps(mut self, steps: Vec<&str>) -> Self {
        self.setup_steps = steps.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_model_selection_hint(mut self, hint: &str) -> Self {
        self.model_selection_hint = Some(hint.to_string());
        self
    }
}

/// Configuration key metadata for provider setup
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigKey {
    /// The name of the configuration key (e.g., "API_KEY")
    pub name: String,
    /// Whether this key is required for the provider to function
    pub required: bool,
    /// Whether this key should be stored securely (e.g., in keychain)
    pub secret: bool,
    /// Optional default value for the key
    pub default: Option<String>,
    /// Whether this key should be configured using an OAuth flow
    /// When true, the provider's configure_oauth() method will be called instead of prompting for manual input
    pub oauth_flow: bool,
    /// Whether this OAuth flow uses the device code grant (RFC 8628)
    /// When true, the user must enter a verification code in the browser
    #[serde(default)]
    pub device_code_flow: bool,
    /// Whether this key should be shown prominently during provider setup
    /// (onboarding, settings modal, CLI configure)
    #[serde(default)]
    pub primary: bool,
}

impl ConfigKey {
    pub fn new(
        name: &str,
        required: bool,
        secret: bool,
        default: Option<&str>,
        primary: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            required,
            secret,
            default: default.map(|s| s.to_string()),
            oauth_flow: false,
            device_code_flow: false,
            primary,
        }
    }

    pub fn from_value_type<T: ConfigValue>(required: bool, secret: bool, primary: bool) -> Self {
        Self {
            name: T::KEY.to_string(),
            required,
            secret,
            default: Some(T::DEFAULT.to_string()),
            oauth_flow: false,
            device_code_flow: false,
            primary,
        }
    }

    pub fn new_oauth(
        name: &str,
        required: bool,
        secret: bool,
        default: Option<&str>,
        primary: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            required,
            secret,
            default: default.map(|s| s.to_string()),
            oauth_flow: true,
            device_code_flow: false,
            primary,
        }
    }

    pub fn new_oauth_device_code(
        name: &str,
        required: bool,
        secret: bool,
        default: Option<&str>,
        primary: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            required,
            secret,
            default: default.map(|s| s.to_string()),
            oauth_flow: true,
            device_code_flow: true,
            primary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub model: String,
    pub usage: Usage,
}

impl ProviderUsage {
    pub fn new(model: String, usage: Usage) -> Self {
        Self { model, usage }
    }

    pub fn combine_with(&self, other: &ProviderUsage) -> ProviderUsage {
        ProviderUsage {
            model: self.model.clone(),
            usage: self.usage + other.usage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub cache_read_input_tokens: Option<i32>,
    pub cache_write_input_tokens: Option<i32>,
}

fn sum_optionals<T>(a: Option<T>, b: Option<T>) -> Option<T>
where
    T: Add<Output = T> + Default,
{
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) => Some(x + T::default()),
        (None, Some(y)) => Some(T::default() + y),
        (None, None) => None,
    }
}

impl Add for Usage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            sum_optionals(self.input_tokens, other.input_tokens),
            sum_optionals(self.output_tokens, other.output_tokens),
            sum_optionals(self.total_tokens, other.total_tokens),
        )
        .with_cache_tokens(
            sum_optionals(self.cache_read_input_tokens, other.cache_read_input_tokens),
            sum_optionals(
                self.cache_write_input_tokens,
                other.cache_write_input_tokens,
            ),
        )
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Usage {
    pub fn new(
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        total_tokens: Option<i32>,
    ) -> Self {
        let calculated_total = if total_tokens.is_none() {
            match (input_tokens, output_tokens) {
                (Some(input), Some(output)) => Some(input + output),
                (Some(input), None) => Some(input),
                (None, Some(output)) => Some(output),
                (None, None) => None,
            }
        } else {
            total_tokens
        };

        Self {
            input_tokens,
            output_tokens,
            total_tokens: calculated_total,
            cache_read_input_tokens: None,
            cache_write_input_tokens: None,
        }
    }

    pub fn with_cache_tokens(
        mut self,
        cache_read_input_tokens: Option<i32>,
        cache_write_input_tokens: Option<i32>,
    ) -> Self {
        self.cache_read_input_tokens = cache_read_input_tokens;
        self.cache_write_input_tokens = cache_write_input_tokens;
        self
    }
}

pub fn current_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionRouting {
    ActionRequired,
    Noop,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn get_name(&self) -> &str;

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError>;

    #[tracing::instrument(
        skip(self, model_config, session_id, system, messages, tools),
        fields(session.id = %session_id, gen_ai.request.model = %model_config.model_name)
    )]
    async fn complete(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let stream = self
            .stream(model_config, session_id, system, messages, tools)
            .await?;
        collect_stream(stream).await
    }

    async fn complete_fast(
        &self,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let model_config = self.get_model_config();
        let fast_config = model_config.use_fast_model();

        let result = self
            .complete(&fast_config, session_id, system, messages, tools)
            .await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                if fast_config.model_name != model_config.model_name {
                    tracing::warn!(
                        "Fast model {} failed with error: {}. Falling back to regular model {}",
                        fast_config.model_name,
                        e,
                        model_config.model_name
                    );
                    self.complete(&model_config, session_id, system, messages, tools)
                        .await
                } else {
                    Err(e)
                }
            }
        }
    }

    fn get_model_config(&self) -> ModelConfig;

    fn retry_config(&self) -> RetryConfig {
        RetryConfig::default()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }

    async fn fetch_supported_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(self
            .fetch_supported_models()
            .await?
            .iter()
            .map(|model_name| model_info_for_provider_model(self.get_name(), model_name))
            .collect())
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        Ok(model_info_for_provider_model(self.get_name(), model_name))
    }

    fn skip_canonical_filtering(&self) -> bool {
        false
    }

    async fn fetch_recommended_models(&self) -> Result<Vec<String>, ProviderError> {
        let all_models = self.fetch_supported_models().await?;

        if self.skip_canonical_filtering() {
            return Ok(all_models);
        }

        let registry = CanonicalModelRegistry::bundled().map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load canonical registry: {}", e))
        })?;

        let provider_name = self.get_name();
        let mut models_with_dates: Vec<(String, Option<String>)> = all_models
            .iter()
            .filter_map(|model| {
                let canonical_id = map_to_canonical_model(provider_name, model, registry)?;
                let (provider, model_name) = canonical_id.split_once('/')?;
                let canonical_model = registry.get(provider, model_name)?;

                if !canonical_model.modalities.input.contains(&Modality::Text) {
                    return None;
                }

                if !canonical_model.tool_call && !self.get_model_config().toolshim {
                    return None;
                }

                Some((model.clone(), canonical_model.release_date.clone()))
            })
            .collect();

        models_with_dates.sort_by(|a, b| match (&a.1, &b.1) {
            (Some(date_a), Some(date_b)) => date_b.cmp(date_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(&b.0),
        });

        let inventory_models: Vec<String> = models_with_dates
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        if inventory_models.is_empty() {
            Ok(all_models)
        } else {
            Ok(inventory_models)
        }
    }

    async fn fetch_recommended_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(self
            .fetch_recommended_models()
            .await?
            .iter()
            .map(|model_name| model_info_for_provider_model(self.get_name(), model_name))
            .collect())
    }

    async fn map_to_canonical_model(
        &self,
        provider_model: &str,
    ) -> Result<Option<String>, ProviderError> {
        let registry = CanonicalModelRegistry::bundled().map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load canonical registry: {}", e))
        })?;

        Ok(map_to_canonical_model(
            self.get_name(),
            provider_model,
            registry,
        ))
    }

    fn supports_embeddings(&self) -> bool {
        false
    }

    fn manages_own_context(&self) -> bool {
        false
    }

    async fn supports_cache_control(&self) -> bool {
        false
    }

    async fn create_embeddings(
        &self,
        _session_id: &str,
        _texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, ProviderError> {
        Err(ProviderError::ExecutionError(
            "This provider does not support embeddings".to_string(),
        ))
    }

    fn get_initial_user_messages(&self, messages: &Conversation) -> Vec<String> {
        messages
            .iter()
            .filter(|m| m.role == rmcp::model::Role::User)
            .take(MSG_COUNT_FOR_SESSION_NAME_GENERATION)
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|c| c.filter_for_audience(rmcp::model::Role::User))
                    .filter_map(|c| c.as_text().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect()
    }

    fn get_preprompt_context(&self, messages: &Conversation) -> String {
        messages
            .iter()
            .filter(|m| m.role == rmcp::model::Role::User)
            .take(1)
            .flat_map(|m| m.content.iter())
            .filter_map(|c| {
                if c.filter_for_audience(rmcp::model::Role::User).is_none() {
                    c.as_text().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    async fn generate_session_name(
        &self,
        session_id: &str,
        messages: &Conversation,
    ) -> Result<String, ProviderError> {
        const SESSION_NAME_PROMPT: &str = "Generate a short title (four words or less) that describes the topic of the user's messages.\nReply with only the title, nothing else. Do not show your reasoning.";
        const SESSION_NAME_BEGIN_MARKER: &str = "---BEGIN USER MESSAGES---";
        const SESSION_NAME_END_MARKER: &str = "---END USER MESSAGES---";
        const SESSION_NAME_SUFFIX: &str = "Generate a short title for the above messages.";

        let context = self.get_initial_user_messages(messages);
        let preprompt_context = self.get_preprompt_context(messages);

        let preprompt_section = if preprompt_context.is_empty() {
            String::new()
        } else {
            format!(
                "---BEGIN BACKGROUND CONTEXT (for understanding only, do NOT base the title on this)---\n{}\n---END BACKGROUND CONTEXT---\n\n",
                preprompt_context
            )
        };

        let user_text = format!(
            "{}{}\n{}\n{}\n\n{}",
            preprompt_section,
            SESSION_NAME_BEGIN_MARKER,
            context.join("\n"),
            SESSION_NAME_END_MARKER,
            SESSION_NAME_SUFFIX,
        );
        let message = Message::user().with_text(&user_text);
        let result = self
            .complete_fast(session_id, SESSION_NAME_PROMPT, &[message], &[])
            .await?;

        let raw: String = result
            .0
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect();
        let description = strip_xml_tags(&raw)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        Ok(safe_truncate(&extract_short_title(&description), 100))
    }

    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        Err(ProviderError::ExecutionError(
            "OAuth configuration not supported by this provider".to_string(),
        ))
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        Err(ProviderError::NotImplemented(
            "credential refresh not supported by this provider".to_string(),
        ))
    }

    fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::Noop
    }

    async fn handle_permission_confirmation(
        &self,
        _request_id: &str,
        _confirmation: &PermissionConfirmation,
    ) -> bool {
        false
    }
}

pub type MessageStream = Pin<
    Box<dyn Stream<Item = Result<(Option<Message>, Option<ProviderUsage>), ProviderError>> + Send>,
>;

pub fn stream_from_single_message(message: Message, usage: ProviderUsage) -> MessageStream {
    let stream = futures::stream::once(async move { Ok((Some(message), Some(usage))) });
    Box::pin(stream)
}

pub async fn collect_stream(
    mut stream: MessageStream,
) -> Result<(Message, ProviderUsage), ProviderError> {
    use futures::StreamExt;

    let mut final_message: Option<Message> = None;
    let mut final_usage: Option<ProviderUsage> = None;

    while let Some(result) = stream.next().await {
        let (msg_opt, usage_opt) = result?;

        if let Some(msg) = msg_opt {
            final_message = Some(match final_message {
                Some(mut prev) => {
                    for new_content in msg.content {
                        match (&mut prev.content.last_mut(), &new_content) {
                            (
                                Some(MessageContent::Text(last_text)),
                                MessageContent::Text(new_text),
                            ) => {
                                last_text.text.push_str(&new_text.text);
                            }
                            _ => {
                                prev.content.push(new_content);
                            }
                        }
                    }
                    prev
                }
                None => msg,
            });
        }

        if let Some(usage) = usage_opt {
            final_usage = Some(usage);
        }
    }

    match final_message {
        Some(msg) => {
            let usage = final_usage
                .unwrap_or_else(|| ProviderUsage::new("unknown".to_string(), Usage::default()));
            Ok((msg, usage))
        }
        None => Err(ProviderError::ExecutionError(
            "Stream yielded no message".to_string(),
        )),
    }
}
