//! In-process uniffi bindings for the Goose SDK.
//!
//! This is the API surface exposed to Python and Kotlin. It focuses on native
//! Goose providers and mirrors the provider message/tool/streaming model closely
//! enough for Kotlin agent frameworks to avoid JSON-only shims for common paths.

use std::{collections::HashMap, future::Future, sync::Arc, sync::OnceLock, time::Duration};

use base64::Engine as _;
use futures::StreamExt;
use goose_providers::{
    anthropic::AnthropicProviderBuilder,
    api_client::{ApiClient, AuthMethod},
    base::{MessageStream, Provider as GooseProvider},
    conversation::{
        message::{Message, MessageContent as GooseMessageContent},
        token_usage::ProviderUsage,
    },
    databricks::DatabricksProvider as GooseDatabricksProvider,
    databricks_auth::DatabricksAuth,
    databricks_v2::DatabricksV2Provider as GooseDatabricksV2Provider,
    declarative::{DeclarativeProviderConfig, EnvKeyResolver},
    model::ModelConfig,
    openai::OpenAiProviderBuilder,
    utils::sanitize_unicode_tags,
};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ErrorCode, ErrorData, Role, Tool,
};
use serde_json::Value;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GooseError {
    #[error("Rate limit exceeded{retry_after_suffix}")]
    RateLimited {
        retry_after_ms: Option<u64>,
        retry_after_suffix: String,
    },
    #[error("Output token limit exceeded: {details}")]
    OutputTokenLimitExceeded { details: String },
    #[error("Context length exceeded: {details}")]
    ContextLengthExceeded { details: String },
    #[error("Authentication error: {details}")]
    Authentication { details: String },
    #[error("Timeout: {details}")]
    Timeout { details: String },
    #[error("Provider unavailable: {details}")]
    ProviderUnavailable { details: String },
    #[error("{details}")]
    Generic { details: String },
}

impl GooseError {
    fn generic(error: impl ToString) -> Self {
        Self::Generic {
            details: error.to_string(),
        }
    }
}

impl From<anyhow::Error> for GooseError {
    fn from(error: anyhow::Error) -> Self {
        Self::generic(error)
    }
}

impl From<goose_providers::errors::ProviderError> for GooseError {
    fn from(error: goose_providers::errors::ProviderError) -> Self {
        match error {
            goose_providers::errors::ProviderError::Authentication(message) => {
                Self::Authentication { details: message }
            }
            goose_providers::errors::ProviderError::ContextLengthExceeded(message) => {
                Self::ContextLengthExceeded { details: message }
            }
            goose_providers::errors::ProviderError::RateLimitExceeded { retry_delay, .. } => {
                let retry_after_ms = retry_delay.map(|delay| delay.as_millis() as u64);
                let retry_after_suffix = retry_after_ms
                    .map(|ms| format!("; retry after {ms}ms"))
                    .unwrap_or_default();
                Self::RateLimited {
                    retry_after_ms,
                    retry_after_suffix,
                }
            }
            goose_providers::errors::ProviderError::ServerError(message)
            | goose_providers::errors::ProviderError::EndpointNotFound(message)
            | goose_providers::errors::ProviderError::CreditsExhausted {
                details: message, ..
            } => Self::ProviderUnavailable { details: message },
            goose_providers::errors::ProviderError::NetworkError(message)
                if is_timeout(&message) =>
            {
                Self::Timeout { details: message }
            }
            goose_providers::errors::ProviderError::RequestFailed(message)
                if is_timeout(&message) =>
            {
                Self::Timeout { details: message }
            }
            goose_providers::errors::ProviderError::ExecutionError(message)
                if is_output_token_limit(&message) =>
            {
                Self::OutputTokenLimitExceeded { details: message }
            }
            other => Self::generic(other),
        }
    }
}

impl From<serde_json::Error> for GooseError {
    fn from(error: serde_json::Error) -> Self {
        Self::generic(error)
    }
}

fn is_timeout(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("timed out") || message.contains("timeout")
}

fn is_output_token_limit(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("output token")
        || message.contains("max_tokens")
        || message.contains("max tokens")
}

/// Receives provider request logs as JSONL records.
///
/// `start` returns an identifier that is passed to `write` for every record in
/// that request, allowing callers to keep concurrent request logs separate.
#[uniffi::export(callback_interface)]
pub trait RequestLogger: Send + Sync {
    fn start(&self) -> Result<u64, GooseError>;
    fn write(&self, request_id: u64, record: String) -> Result<(), GooseError>;
}

struct RequestLoggerAdapter {
    logger: Arc<dyn RequestLogger>,
}

impl goose_providers::request_log::RequestLogger for RequestLoggerAdapter {
    fn start(
        &self,
    ) -> Result<
        Box<dyn goose_providers::request_log::RequestLogHandle>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        Ok(Box::new(RequestLogHandleAdapter {
            request_id: self.logger.start()?,
            logger: Arc::clone(&self.logger),
        }))
    }
}

struct RequestLogHandleAdapter {
    request_id: u64,
    logger: Arc<dyn RequestLogger>,
}

impl goose_providers::request_log::RequestLogHandle for RequestLogHandleAdapter {
    fn write(&mut self, record: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.write(self.request_id, record.to_string())?;
        Ok(())
    }
}

/// Installs the process-wide provider request logger.
///
/// A logger can only be installed once for the lifetime of the process.
#[uniffi::export]
pub fn install_request_logger(logger: Box<dyn RequestLogger>) -> Result<(), GooseError> {
    goose_providers::request_log::install_logger(RequestLoggerAdapter {
        logger: Arc::from(logger),
    })
    .map_err(|error| GooseError::generic(error.to_string()))
}

/// A text message passed to a provider.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderMessage {
    pub role: MessageRole,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum MessageContent {
    Text {
        text: String,
    },
    Image {
        mime_type: String,
        data: Vec<u8>,
    },
    ToolRequest {
        id: String,
        name: String,
        arguments_json: String,
    },
    ToolResult {
        id: String,
        success: bool,
        content_json: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
}

impl ProviderMessage {
    fn to_goose_message(&self) -> Result<Option<Message>, GooseError> {
        let role = match self.role {
            MessageRole::User | MessageRole::Tool => Role::User,
            MessageRole::Assistant => Role::Assistant,
        };
        let mut message = Message::new(role, chrono_now(), Vec::new());
        for content in &self.content {
            message = message.with_content(content.to_goose_content()?);
        }
        Ok(Some(message))
    }
}

impl MessageContent {
    fn to_goose_content(&self) -> Result<GooseMessageContent, GooseError> {
        match self {
            MessageContent::Text { text } => {
                Ok(GooseMessageContent::text(sanitize_unicode_tags(text)))
            }
            MessageContent::Image { mime_type, data } => Ok(GooseMessageContent::image(
                base64::engine::general_purpose::STANDARD.encode(data),
                mime_type.clone(),
            )),
            MessageContent::ToolRequest {
                id,
                name,
                arguments_json,
            } => {
                let arguments = parse_json_object(arguments_json)?;
                Ok(GooseMessageContent::tool_request(
                    id.clone(),
                    Ok(CallToolRequestParams::new(name.clone()).with_arguments(arguments)),
                ))
            }
            MessageContent::ToolResult {
                id,
                success,
                content_json,
            } => {
                let value: Value = serde_json::from_str(content_json)?;
                let tool_result = if *success {
                    Ok(call_tool_result(value, false))
                } else {
                    Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        value.to_string(),
                        None,
                    ))
                };
                Ok(GooseMessageContent::tool_response(id.clone(), tool_result))
            }
            MessageContent::Thinking {
                thinking,
                signature,
            } => Ok(GooseMessageContent::thinking(
                thinking.clone(),
                signature.clone(),
            )),
            MessageContent::RedactedThinking { data } => {
                Ok(GooseMessageContent::redacted_thinking(data.clone()))
            }
        }
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn call_tool_result(value: Value, is_error: bool) -> CallToolResult {
    let content = match value {
        Value::Array(items) => items.into_iter().map(value_to_content).collect(),
        other => vec![value_to_content(other)],
    };

    if is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

fn value_to_content(value: Value) -> Content {
    match value {
        Value::String(text) => Content::text(text),
        Value::Object(object) => match object.get("type").and_then(|value| value.as_str()) {
            Some("text") => object
                .get("text")
                .and_then(|value| value.as_str().map(str::to_owned))
                .map(Content::text)
                .unwrap_or_else(|| Content::text(Value::Object(object).to_string())),
            Some("image") => {
                let mime_type = object
                    .get("mimeType")
                    .or_else(|| object.get("mime_type"))
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "image/png".to_string());
                let data = object
                    .get("data")
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default();
                Content::image(data, mime_type)
            }
            _ => Content::text(Value::Object(object).to_string()),
        },
        other => Content::text(other.to_string()),
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderTool {
    pub name: String,
    pub description: String,
    pub input_schema_json: String,
    #[uniffi(default = None)]
    pub annotations_json: Option<String>,
}

impl ProviderTool {
    fn to_goose_tool(&self) -> Result<Tool, GooseError> {
        let schema = parse_json_object(&self.input_schema_json)?;
        let mut tool = Tool::new(self.name.clone(), self.description.clone(), schema);
        if let Some(annotations_json) = &self.annotations_json {
            tool.annotations = Some(serde_json::from_str(annotations_json)?);
        }
        Ok(tool)
    }
}

fn parse_json_object(json: &str) -> Result<serde_json::Map<String, Value>, GooseError> {
    match serde_json::from_str(json)? {
        Value::Object(object) => Ok(object),
        _ => Err(GooseError::generic("expected a JSON object")),
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderModelConfig {
    pub model_name: String,
    #[uniffi(default = None)]
    pub context_limit: Option<i32>,
    #[uniffi(default = None)]
    pub temperature: Option<f32>,
    #[uniffi(default = None)]
    pub max_tokens: Option<i32>,
    #[uniffi(default = false)]
    pub toolshim: bool,
    #[uniffi(default = None)]
    pub toolshim_model: Option<String>,
    #[uniffi(default = None)]
    pub request_params_json: Option<String>,
    #[uniffi(default = None)]
    pub provider_params_json: Option<String>,
    #[uniffi(default = None)]
    pub reasoning: Option<bool>,
    #[uniffi(default = None)]
    pub timeout_ms: Option<u64>,
    /// Per-request HTTP headers attached to the outgoing provider call.
    /// These override any static headers configured on the provider.
    #[uniffi(default = None)]
    pub request_headers: Option<HashMap<String, String>>,
}

impl ProviderModelConfig {
    fn to_goose_model_config(&self) -> Result<ModelConfig, GooseError> {
        let mut config = ModelConfig::new(&self.model_name)
            .with_context_limit(self.context_limit.map(|limit| limit.max(0) as usize))
            .with_temperature(self.temperature)
            .with_max_tokens(self.max_tokens)
            .with_toolshim(self.toolshim)
            .with_toolshim_model(self.toolshim_model.clone());

        let mut request_params = serde_json::Map::new();
        merge_params(&mut request_params, self.request_params_json.as_ref())?;
        merge_params(&mut request_params, self.provider_params_json.as_ref())?;
        if !request_params.is_empty() {
            config = config.with_merged_request_params(request_params.into_iter().collect());
        }

        config = config.with_request_headers(self.request_headers.clone());
        config.reasoning = self.reasoning;
        Ok(config)
    }
}

fn merge_params(
    target: &mut serde_json::Map<String, Value>,
    params_json: Option<&String>,
) -> Result<(), GooseError> {
    if let Some(params_json) = params_json {
        for (key, value) in parse_json_object(params_json)? {
            target.insert(key, value);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub cache_read_input_tokens: Option<i32>,
    pub cache_creation_input_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub model: String,
    pub provider_metadata_json: Option<String>,
}

impl Usage {
    fn from_provider_usage(usage: &ProviderUsage) -> Result<Self, GooseError> {
        Ok(Self {
            input_tokens: usage.usage.input_tokens,
            output_tokens: usage.usage.output_tokens,
            total_tokens: usage.usage.total_tokens,
            cache_read_input_tokens: usage.usage.cache_read_input_tokens,
            cache_creation_input_tokens: usage.usage.cache_write_input_tokens,
            reasoning_tokens: None,
            model: usage.model.clone(),
            provider_metadata_json: Some(serde_json::to_string(usage)?),
        })
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum StreamChunk {
    TextChunk {
        text: String,
    },
    ToolChunk {
        id: String,
        name: String,
        arguments_json: String,
    },
    ThinkingChunk {
        thinking: String,
        signature: String,
    },
    RedactedThinkingChunk {
        data: String,
    },
    EndChunk {
        usage: Option<Usage>,
    },
    ErrorChunk {
        error: GooseStreamError,
    },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GooseStreamError {
    pub kind: GooseStreamErrorKind,
    pub message: String,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum GooseStreamErrorKind {
    RateLimited,
    OutputTokenLimitExceeded,
    ContextLengthExceeded,
    Authentication,
    Timeout,
    ProviderUnavailable,
    Generic,
}

impl From<GooseError> for GooseStreamError {
    fn from(error: GooseError) -> Self {
        match error {
            GooseError::RateLimited {
                retry_after_ms,
                retry_after_suffix,
            } => Self {
                kind: GooseStreamErrorKind::RateLimited,
                message: format!("Rate limit exceeded{retry_after_suffix}"),
                retry_after_ms,
            },
            GooseError::OutputTokenLimitExceeded { details } => Self {
                kind: GooseStreamErrorKind::OutputTokenLimitExceeded,
                message: details,
                retry_after_ms: None,
            },
            GooseError::ContextLengthExceeded { details } => Self {
                kind: GooseStreamErrorKind::ContextLengthExceeded,
                message: details,
                retry_after_ms: None,
            },
            GooseError::Authentication { details } => Self {
                kind: GooseStreamErrorKind::Authentication,
                message: details,
                retry_after_ms: None,
            },
            GooseError::Timeout { details } => Self {
                kind: GooseStreamErrorKind::Timeout,
                message: details,
                retry_after_ms: None,
            },
            GooseError::ProviderUnavailable { details } => Self {
                kind: GooseStreamErrorKind::ProviderUnavailable,
                message: details,
                retry_after_ms: None,
            },
            GooseError::Generic { details } => Self {
                kind: GooseStreamErrorKind::Generic,
                message: details,
                retry_after_ms: None,
            },
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderCompletion {
    pub message_json: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum Feature {
    Tools,
    Streaming,
    Images,
    JsonSchema,
    Reasoning,
}

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> Result<&'static tokio::runtime::Runtime, GooseError> {
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(GooseError::generic)?;

    if let Err(unused) = RUNTIME.set(runtime) {
        // Lost the init race; dropping a runtime inside an async context
        // panics, so shut it down without blocking.
        unused.shutdown_background();
    }
    Ok(RUNTIME.get().expect("runtime was initialized"))
}

struct AbortOnDrop<T> {
    handle: tokio::task::JoinHandle<T>,
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn run_on_runtime<T>(
    future: impl Future<Output = T> + Send + 'static,
) -> Result<T, GooseError>
where
    T: Send + 'static,
{
    let mut task = AbortOnDrop {
        handle: runtime()?.spawn(future),
    };
    (&mut task.handle).await.map_err(|error| {
        if error.is_cancelled() {
            GooseError::Timeout {
                details: "runtime task was cancelled".to_string(),
            }
        } else {
            GooseError::generic(error)
        }
    })
}

struct ProviderHandle {
    provider: Arc<dyn GooseProvider>,
}

impl ProviderHandle {
    fn new(provider: Box<dyn GooseProvider>) -> Self {
        Self {
            provider: Arc::from(provider),
        }
    }

    fn name(&self) -> String {
        self.provider.get_name().to_string()
    }

    async fn stream(
        &self,
        model: ProviderModelConfig,
        system: String,
        messages: Vec<ProviderMessage>,
        tools: Vec<ProviderTool>,
    ) -> Result<Arc<ProviderStream>, GooseError> {
        let timeout_ms = model.timeout_ms;
        let model = model.to_goose_model_config()?;
        let messages = convert_messages(messages)?;
        let tools = convert_tools(tools)?;
        let provider = Arc::clone(&self.provider);
        let stream = run_provider_future(timeout_ms, async move {
            provider.stream(&model, &system, &messages, &tools).await
        })
        .await??;

        Ok(Arc::new(ProviderStream {
            state: Arc::new(tokio::sync::Mutex::new(ProviderStreamState {
                stream,
                pending: Vec::new(),
                final_usage: None,
                ended: false,
            })),
            timeout_ms,
        }))
    }

    async fn complete(
        &self,
        model: ProviderModelConfig,
        system: String,
        messages: Vec<ProviderMessage>,
        tools: Vec<ProviderTool>,
    ) -> Result<ProviderCompletion, GooseError> {
        let timeout_ms = model.timeout_ms;
        let model = model.to_goose_model_config()?;
        let messages = convert_messages(messages)?;
        let tools = convert_tools(tools)?;
        let provider = Arc::clone(&self.provider);
        let (message, usage) = run_provider_future(timeout_ms, async move {
            provider.complete(&model, &system, &messages, &tools).await
        })
        .await??;

        Ok(ProviderCompletion {
            message_json: serde_json::to_string(&message)?,
            usage: Some(Usage::from_provider_usage(&usage)?),
        })
    }
}

async fn run_provider_future<T>(
    timeout_ms: Option<u64>,
    future: impl Future<Output = T> + Send + 'static,
) -> Result<T, GooseError>
where
    T: Send + 'static,
{
    run_on_runtime(async move {
        if let Some(timeout_ms) = timeout_ms {
            tokio::time::timeout(Duration::from_millis(timeout_ms), future)
                .await
                .map_err(|_| GooseError::Timeout {
                    details: format!("request timed out after {timeout_ms}ms"),
                })
        } else {
            Ok(future.await)
        }
    })
    .await?
}

fn convert_messages(messages: Vec<ProviderMessage>) -> Result<Vec<Message>, GooseError> {
    messages
        .iter()
        .filter_map(|message| message.to_goose_message().transpose())
        .collect()
}

fn convert_tools(tools: Vec<ProviderTool>) -> Result<Vec<Tool>, GooseError> {
    tools.iter().map(ProviderTool::to_goose_tool).collect()
}

#[derive(uniffi::Object)]
pub struct Provider {
    handle: ProviderHandle,
}

impl Provider {
    fn new(provider: Box<dyn GooseProvider>) -> Arc<Self> {
        Arc::new(Self {
            handle: ProviderHandle::new(provider),
        })
    }
}

#[uniffi::export]
impl Provider {
    pub fn name(&self) -> String {
        self.handle.name()
    }

    pub fn supported_features(&self) -> Vec<Feature> {
        let name = self.name();
        let mut features = vec![Feature::Streaming, Feature::Tools, Feature::JsonSchema];
        if matches!(
            name.as_str(),
            "openai" | "anthropic" | "databricks" | "databricks_v2" | "groq"
        ) {
            features.push(Feature::Images);
        }
        if matches!(
            name.as_str(),
            "openai" | "anthropic" | "databricks" | "databricks_v2"
        ) {
            features.push(Feature::Reasoning);
        }
        features
    }

    pub async fn stream(
        &self,
        model: ProviderModelConfig,
        system: String,
        messages: Vec<ProviderMessage>,
        tools: Vec<ProviderTool>,
    ) -> Result<Arc<ProviderStream>, GooseError> {
        self.handle.stream(model, system, messages, tools).await
    }

    pub async fn complete(
        &self,
        model: ProviderModelConfig,
        system: String,
        messages: Vec<ProviderMessage>,
        tools: Vec<ProviderTool>,
    ) -> Result<ProviderCompletion, GooseError> {
        self.handle.complete(model, system, messages, tools).await
    }
}

#[uniffi::export]
pub fn declarative_provider_from_json(json: String) -> Result<Arc<Provider>, GooseError> {
    let provider = goose_providers::declarative::from_json(&json, None, EnvKeyResolver {})?;
    Ok(Provider::new(provider))
}

#[uniffi::export]
pub fn openai_default_model() -> String {
    goose_providers::openai::OPEN_AI_DEFAULT_MODEL.to_string()
}

#[uniffi::export]
pub fn openai_provider(api_key: String) -> Result<Arc<Provider>, GooseError> {
    let api_client = ApiClient::new_with_tls(
        "https://api.openai.com".to_string(),
        AuthMethod::BearerToken(api_key),
        None,
    )?;
    let provider = OpenAiProviderBuilder::new(api_client).build();
    Ok(Provider::new(Box::new(provider)))
}

#[uniffi::export]
pub fn anthropic_default_model() -> String {
    goose_providers::anthropic::ANTHROPIC_DEFAULT_MODEL.to_string()
}

#[uniffi::export]
pub fn anthropic_provider(
    api_key: String,
    base_url: Option<String>,
    beta_headers: Vec<String>,
) -> Result<Arc<Provider>, GooseError> {
    let mut api_client = ApiClient::new_with_tls(
        base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
        AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key: api_key,
        },
        None,
    )?
    .with_header(
        "anthropic-version",
        goose_providers::anthropic::ANTHROPIC_API_VERSION,
    )?;

    if !beta_headers.is_empty() {
        api_client = api_client.with_header("anthropic-beta", &beta_headers.join(","))?;
    }

    let provider = AnthropicProviderBuilder::new(api_client).build();
    Ok(Provider::new(Box::new(provider)))
}

#[uniffi::export]
pub fn groq_default_model() -> String {
    groq_config()
        .models
        .first()
        .map(|model| model.name.clone())
        .unwrap_or_else(|| "moonshotai/kimi-k2-instruct-0905".to_string())
}

#[uniffi::export]
pub fn groq_provider(api_key: String) -> Result<Arc<Provider>, GooseError> {
    let json = goose_providers::groq::JSON.replace("${GROQ_API_KEY}", &api_key);
    let provider = goose_providers::declarative::from_json(
        &json,
        None,
        StaticKeyResolver {
            key_name: "GROQ_API_KEY".to_string(),
            key: api_key,
        },
    )?;
    Ok(Provider::new(provider))
}

fn groq_config() -> DeclarativeProviderConfig {
    goose_providers::declarative::deserialize_provider_config(goose_providers::groq::JSON)
        .expect("bundled groq provider config is valid")
}

struct StaticKeyResolver {
    key_name: String,
    key: String,
}

impl goose_providers::declarative::KeyResolver for StaticKeyResolver {
    type Error = std::env::VarError;

    fn resolve_key(&self, key: &str) -> std::result::Result<String, Self::Error> {
        if key == self.key_name {
            Ok(self.key.clone())
        } else {
            std::env::var(key)
        }
    }
}

#[uniffi::export]
pub fn databricks_default_model() -> String {
    goose_providers::databricks::DATABRICKS_DEFAULT_MODEL.to_string()
}

#[uniffi::export]
pub fn databricks_provider(host: String, token: String) -> Result<Arc<Provider>, GooseError> {
    let retry_config = GooseDatabricksProvider::load_retry_config(|key| std::env::var(key).ok());
    let provider = GooseDatabricksProvider::new(
        host,
        DatabricksAuth::token(token),
        retry_config,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    Ok(Provider::new(Box::new(provider)))
}

#[uniffi::export]
pub fn databricks_v2_default_model() -> String {
    goose_providers::databricks_v2::DATABRICKS_V2_DEFAULT_MODEL.to_string()
}

#[uniffi::export]
pub fn databricks_v2_provider(host: String, token: String) -> Result<Arc<Provider>, GooseError> {
    let retry_config = GooseDatabricksV2Provider::load_retry_config(|key| std::env::var(key).ok());
    let provider = GooseDatabricksV2Provider::new(
        host,
        DatabricksAuth::token(token),
        retry_config,
        None,
        None,
        None,
        None,
        None,
    )?;

    Ok(Provider::new(Box::new(provider)))
}

#[derive(uniffi::Object)]
pub struct ProviderStream {
    state: Arc<tokio::sync::Mutex<ProviderStreamState>>,
    timeout_ms: Option<u64>,
}

struct ProviderStreamState {
    stream: MessageStream,
    pending: Vec<StreamChunk>,
    final_usage: Option<Usage>,
    ended: bool,
}

#[uniffi::export]
impl ProviderStream {
    pub async fn next_chunk(&self) -> Result<Option<StreamChunk>, GooseError> {
        let state = Arc::clone(&self.state);
        let timeout_ms = self.timeout_ms;
        run_on_runtime(async move {
            let mut state = state.lock().await;
            loop {
                if let Some(chunk) = state.pending.pop() {
                    return Ok(Some(chunk));
                }

                if state.ended {
                    return Ok(None);
                }

                let next = if let Some(timeout_ms) = timeout_ms {
                    tokio::time::timeout(Duration::from_millis(timeout_ms), state.stream.next())
                        .await
                        .map_err(|_| GooseError::Timeout {
                            details: format!("request timed out after {timeout_ms}ms"),
                        })?
                } else {
                    state.stream.next().await
                };

                match next {
                    Some(Ok((message, usage))) => {
                        if let Some(usage) = usage {
                            state.final_usage = Some(Usage::from_provider_usage(&usage)?);
                        }
                        let Some(message) = message else {
                            continue;
                        };
                        let mut chunks = message_to_chunks(message);
                        if chunks.is_empty() {
                            continue;
                        }
                        chunks.reverse();
                        let first = chunks.pop();
                        state.pending = chunks;
                        return Ok(first);
                    }
                    Some(Err(error)) => {
                        state.ended = true;
                        return Ok(Some(StreamChunk::ErrorChunk {
                            error: GooseStreamError::from(GooseError::from(error)),
                        }));
                    }
                    None => {
                        state.ended = true;
                        return Ok(Some(StreamChunk::EndChunk {
                            usage: state.final_usage.clone(),
                        }));
                    }
                }
            }
        })
        .await?
    }
}

fn message_to_chunks(message: Message) -> Vec<StreamChunk> {
    message
        .content
        .into_iter()
        .filter_map(|content| match content {
            GooseMessageContent::Text(text) if !text.text.is_empty() => {
                Some(StreamChunk::TextChunk {
                    text: text.text.clone(),
                })
            }
            GooseMessageContent::ToolRequest(request) => match request.tool_call {
                Ok(tool_call) => Some(StreamChunk::ToolChunk {
                    id: request.id,
                    name: tool_call.name.to_string(),
                    arguments_json: serde_json::to_string(&tool_call.arguments.unwrap_or_default())
                        .unwrap_or_else(|_| "{}".to_string()),
                }),
                Err(error) => Some(StreamChunk::ErrorChunk {
                    error: GooseStreamError {
                        kind: GooseStreamErrorKind::Generic,
                        message: error.to_string(),
                        retry_after_ms: None,
                    },
                }),
            },
            GooseMessageContent::Thinking(thinking) => Some(StreamChunk::ThinkingChunk {
                thinking: thinking.thinking,
                signature: thinking.signature,
            }),
            GooseMessageContent::RedactedThinking(redacted) => {
                Some(StreamChunk::RedactedThinkingChunk {
                    data: redacted.data,
                })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_model_config() -> ProviderModelConfig {
        ProviderModelConfig {
            model_name: "test".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params_json: None,
            provider_params_json: None,
            reasoning: None,
            timeout_ms: None,
            request_headers: None,
        }
    }

    #[test]
    fn model_config_rejects_invalid_request_params_json() {
        let config = ProviderModelConfig {
            request_params_json: Some("not json".to_string()),
            ..base_model_config()
        };

        assert!(config.to_goose_model_config().is_err());
    }

    #[test]
    fn provider_message_converts_user_text() {
        let message = ProviderMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text {
                text: "what is the capital of France?".to_string(),
            }],
        }
        .to_goose_message()
        .unwrap()
        .unwrap();

        assert_eq!(message.as_concat_text(), "what is the capital of France?");
    }

    #[test]
    fn tool_config_converts_to_rmcp_tool() {
        let tool = ProviderTool {
            name: "lookup".to_string(),
            description: "Lookup a value".to_string(),
            input_schema_json: r#"{"type":"object","properties":{"key":{"type":"string"}}}"#
                .to_string(),
            annotations_json: None,
        }
        .to_goose_tool()
        .unwrap();

        assert_eq!(tool.name.as_ref(), "lookup");
        assert_eq!(tool.input_schema["type"], "object");
    }

    #[test]
    fn tool_result_content_converts() {
        let content = MessageContent::ToolResult {
            id: "call_1".to_string(),
            success: true,
            content_json: r#"{"type":"text","text":"done"}"#.to_string(),
        }
        .to_goose_content()
        .unwrap();

        let GooseMessageContent::ToolResponse(response) = content else {
            panic!("expected tool response");
        };
        let result = response.tool_result.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content[0].as_text().unwrap().text, "done");
    }
}
