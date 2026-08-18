use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use goose_providers::images::ImageFormat;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::openai_compatible::{handle_status, stream_openai_compat};
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::providers::formats::openrouter as openrouter_format;
use goose_providers::cache_semantics::{apply_chat_payload_breakpoints, CacheSemantics};
use goose_providers::errors::ProviderError;
use goose_providers::formats::openai::create_request;
use goose_providers::model::ModelConfig;
use goose_providers::request_log::{start_log, LoggerHandleExt};
use rmcp::model::Tool;

pub const OPENROUTER_PROVIDER_NAME: &str = "openrouter";
const OPENROUTER_PARAMETERS_CONFIG_KEY: &str = "OPENROUTER_PARAMETERS";
pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4";
pub const OPENROUTER_DEFAULT_FAST_MODEL: &str = "google/gemini-2.5-flash";

// OpenRouter can run many models, we suggest the default
pub const OPENROUTER_KNOWN_MODELS: &[&str] = &[
    "x-ai/grok-code-fast-1",
    "anthropic/claude-sonnet-4.5",
    "anthropic/claude-sonnet-4",
    "anthropic/claude-opus-4.1",
    "anthropic/claude-opus-4",
    "google/gemini-2.5-pro",
    "google/gemini-2.5-flash",
    "deepseek/deepseek-r1-0528",
    "qwen/qwen3-coder",
    "moonshotai/kimi-k2",
];
pub const OPENROUTER_DOC_URL: &str = "https://openrouter.ai/models";

const GEMINI_SCHEMA_REF_KEY: &str = "$ref";
const GEMINI_SAFE_SCHEMA_REF_KEY_BASE: &str = "dollar_ref";

#[derive(serde::Serialize)]
pub struct OpenRouterProvider {
    #[serde(skip)]
    api_client: ApiClient,
    supports_streaming: bool,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    configured_parameters: Option<HashMap<String, Value>>,
}

impl OpenRouterProvider {
    pub async fn from_env(
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENROUTER_API_KEY")?;
        let host: String = config
            .get_param("OPENROUTER_HOST")
            .unwrap_or_else(|_| "https://openrouter.ai".to_string());

        let configured_parameters = configured_openrouter_parameters()?;

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new_with_tls(host, auth, tls_config)?
            .with_request_builder(crate::session_context::session_id_request_builder())
            .with_header("HTTP-Referer", "https://goose-docs.ai")?
            .with_header("X-Title", "goose")?;

        Ok(Self {
            api_client,
            supports_streaming: true,
            name: OPENROUTER_PROVIDER_NAME.to_string(),
            configured_parameters,
        })
    }

    async fn post_chat_completions(
        &self,
        model_config: &ModelConfig,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        self.with_retry(|| async {
            let resp = self
                .api_client
                .request("api/v1/chat/completions")
                .model_headers(model_config)?
                .streaming(true)
                .response_post(payload)
                .await?;
            handle_status(resp).await
        })
        .await
    }
}

fn is_mandatory_reasoning_error(error: &ProviderError) -> bool {
    matches!(error, ProviderError::RequestFailed(message) if message.contains("Reasoning is mandatory"))
}

fn is_gemini_model(model_name: &str) -> bool {
    model_name.starts_with("google/gemini")
}

#[derive(Default)]
struct JsonObjectKeyScan {
    occupied_keys: HashSet<String>,
    schema_ref_key_spans: Vec<Range<usize>>,
}

fn decode_json_string_literal(content: &str, span: &Range<usize>) -> Option<String> {
    serde_json::from_str(content.get(span.clone())?).ok()
}

/// Tolerantly scans JSON-like text without requiring the entire input to parse.
/// A string is treated as an object key only when it is inside an object,
/// follows `{` or `,`, and is followed by `:` outside the string.
fn scan_json_object_keys(content: &str) -> JsonObjectKeyScan {
    let bytes = content.as_bytes();
    let mut scan = JsonObjectKeyScan::default();
    let mut containers = Vec::new();
    let mut previous_non_whitespace = None;
    let mut pending_key_span = None;
    let mut string_start = 0;
    let mut string_container = None;
    let mut string_preceding_byte = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, byte) in bytes.iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
                let span = string_start..index + 1;
                if string_container == Some(b'{')
                    && matches!(string_preceding_byte, Some(b'{') | Some(b','))
                {
                    pending_key_span = Some(span);
                }
                previous_non_whitespace = Some(b'"');
            }
            continue;
        }

        if byte.is_ascii_whitespace() {
            continue;
        }

        if let Some(span) = pending_key_span.take() {
            if byte == b':' {
                if let Some(key) = decode_json_string_literal(content, &span) {
                    if key == GEMINI_SCHEMA_REF_KEY {
                        scan.schema_ref_key_spans.push(span);
                    }
                    scan.occupied_keys.insert(key);
                }
            }
        }

        match byte {
            b'"' => {
                in_string = true;
                escaped = false;
                string_start = index;
                string_container = containers.last().copied();
                string_preceding_byte = previous_non_whitespace;
            }
            b'{' | b'[' => containers.push(byte),
            b'}' if containers.last() == Some(&b'{') => {
                containers.pop();
            }
            b']' if containers.last() == Some(&b'[') => {
                containers.pop();
            }
            _ => {}
        }
        previous_non_whitespace = Some(byte);
    }

    scan
}

fn replace_json_key_spans(content: &str, spans: &[Range<usize>], replacement: &str) -> String {
    let serialized_replacement =
        serde_json::to_string(replacement).expect("JSON object keys must serialize successfully");
    let mut rewritten = String::with_capacity(content.len());
    let mut copied_through = 0;

    for span in spans {
        rewritten.push_str(
            content
                .get(copied_through..span.start)
                .expect("scanned JSON key spans must lie on UTF-8 boundaries"),
        );
        rewritten.push_str(&serialized_replacement);
        copied_through = span.end;
    }
    rewritten.push_str(
        content
            .get(copied_through..)
            .expect("scanned JSON key spans must lie on UTF-8 boundaries"),
    );

    rewritten
}

fn collision_free_gemini_schema_ref_key(occupied_keys: &HashSet<String>) -> String {
    for suffix in 1.. {
        let candidate = if suffix == 1 {
            GEMINI_SAFE_SCHEMA_REF_KEY_BASE.to_string()
        } else {
            format!("{GEMINI_SAFE_SCHEMA_REF_KEY_BASE}_{suffix}")
        };
        if !occupied_keys.contains(&candidate) {
            return candidate;
        }
    }

    unreachable!()
}

fn gemini_schema_ref_note(safe_key: &str) -> String {
    format!(
        "[OpenRouter/Gemini compatibility: interpret `{safe_key}` as the JSON Schema key formed by `$` followed by `ref`.]\n"
    )
}

fn apply_gemini_compatibility(model_name: &str, payload: &mut Value, messages: &[Message]) {
    if is_gemini_model(model_name) {
        escape_gemini_schema_ref_keys_in_tool_responses(payload);
        openrouter_format::add_reasoning_details_to_request(payload, messages);
    }
}

/// OpenRouter translates OpenAI `role: tool` messages into Gemini
/// `function_response` parts. Gemini rejects a response containing a literal
/// JSON Schema `$ref` key, treating its value as a function-response part name
/// instead of arbitrary tool text. Escape object keys in otherwise opaque tool
/// text and add a reversible note so the model can reconstruct the original
/// text. All bytes outside matching key spans remain unchanged.
fn escape_gemini_schema_ref_keys_in_tool_responses(payload: &mut Value) -> usize {
    let Some(messages) = payload.get_mut("messages").and_then(Value::as_array_mut) else {
        return 0;
    };

    let mut occupied_keys = HashSet::new();
    let mut scanned_tool_results = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }

        let Some(content_text) = message.get("content").and_then(Value::as_str) else {
            continue;
        };
        let scan = scan_json_object_keys(content_text);
        occupied_keys.extend(scan.occupied_keys);
        scanned_tool_results.push((message_index, scan.schema_ref_key_spans));
    }

    let safe_key = collision_free_gemini_schema_ref_key(&occupied_keys);
    let note = gemini_schema_ref_note(&safe_key);
    let mut escaped = 0;
    for (message_index, schema_ref_key_spans) in scanned_tool_results {
        if schema_ref_key_spans.is_empty() {
            continue;
        }

        let content_text = messages[message_index]["content"]
            .as_str()
            .expect("scanned tool result content must remain a string");
        let sanitized = replace_json_key_spans(content_text, &schema_ref_key_spans, &safe_key);
        messages[message_index]["content"] = Value::String(format!("{note}{sanitized}"));
        escaped += schema_ref_key_spans.len();
    }

    escaped
}

fn parse_openrouter_parameters(raw: Value) -> Result<HashMap<String, Value>> {
    match raw {
        Value::Object(params) => Ok(params.into_iter().collect()),
        Value::String(raw_json) => match serde_json::from_str::<Value>(&raw_json)? {
            Value::Object(params) => Ok(params.into_iter().collect()),
            _ => bail!("{OPENROUTER_PARAMETERS_CONFIG_KEY} must be a JSON object"),
        },
        _ => bail!("{OPENROUTER_PARAMETERS_CONFIG_KEY} must be a JSON object"),
    }
}

fn configured_openrouter_parameters() -> Result<Option<HashMap<String, Value>>> {
    let config = crate::config::Config::global();
    match config.get_param::<Value>(OPENROUTER_PARAMETERS_CONFIG_KEY) {
        Ok(raw) => parse_openrouter_parameters(raw).map(Some),
        Err(crate::config::ConfigError::NotFound(_)) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn merge_request_params(
    request_params: &mut Option<HashMap<String, Value>>,
    params: HashMap<String, Value>,
) {
    request_params
        .get_or_insert_with(HashMap::new)
        .extend(params);
}

fn merge_openrouter_parameters(model: &mut ModelConfig, params: HashMap<String, Value>) {
    merge_request_params(&mut model.request_params, params);
}

impl goose_providers::base::ProviderDescriptor for OpenRouterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OPENROUTER_PROVIDER_NAME,
            "OpenRouter",
            "Router for many model providers",
            OPENROUTER_DEFAULT_MODEL,
            OPENROUTER_KNOWN_MODELS.to_vec(),
            OPENROUTER_DOC_URL,
            vec![
                ConfigKey::new("OPENROUTER_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "OPENROUTER_HOST",
                    false,
                    false,
                    Some("https://openrouter.ai"),
                    false,
                ),
                ConfigKey::new(OPENROUTER_PARAMETERS_CONFIG_KEY, false, false, None, false),
            ],
        )
        .with_setup(
            crate::providers::catalog::ProviderSetupMetadata::api_key(
                crate::providers::catalog::ProviderSetupGroup::Default,
            )
            .with_docs_url("https://openrouter.ai/keys"),
        )
        .with_setup_steps(vec![
            "Go to https://openrouter.ai/settings/keys",
            "Click 'Create' or use an existing API key",
            "Copy the key and paste it above",
        ])
        .with_fast_model(OPENROUTER_DEFAULT_FAST_MODEL)
    }
}

impl ProviderDef for OpenRouterProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(tls_config))
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        true
    }

    async fn fetch_recommended_models(&self, toolshim: bool) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request("api/v1/models")
            .response_get()
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to fetch models from OpenRouter API: {}",
                    e
                ))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse OpenRouter API response as JSON: {}",
                e
            ))
        })?;

        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::RequestFailed(format!(
                "OpenRouter API returned an error: {}",
                msg
            )));
        }

        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|model| {
                let id = model.get("id").and_then(|v| v.as_str())?;
                if toolshim {
                    return Some(id.to_string());
                }
                let supports_tools = model
                    .get("supported_parameters")
                    .and_then(|v| v.as_array())
                    .is_some_and(|params| params.iter().any(|p| p.as_str() == Some("tools")));
                if supports_tools {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        models.sort();
        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = crate::session_context::current_session_id().unwrap_or_default();

        let mut merged_model;
        let model_config = if let Some(params) = &self.configured_parameters {
            merged_model = model_config.clone();
            merge_openrouter_parameters(&mut merged_model, params.clone());
            &merged_model
        } else {
            model_config
        };

        let mut payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;

        // Add user field for OpenRouter attribution/rate-limiting
        if !session_id.is_empty() {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("user".to_string(), Value::String(session_id.to_string()));
            }
        }

        if CacheSemantics::for_model(OPENROUTER_PROVIDER_NAME, &model_config.model_name)
            .uses_explicit_breakpoints()
            && !model_config.prompt_cache_disabled()
        {
            apply_chat_payload_breakpoints(&mut payload);
        }

        apply_gemini_compatibility(&model_config.model_name, &mut payload, messages);
        let sent_reasoning_disable =
            openrouter_format::apply_reasoning_config(&mut payload, model_config);

        if let Some(obj) = payload.as_object_mut() {
            obj.insert("transforms".to_string(), json!(["middle-out"]));
            obj.insert("usage".to_string(), json!({ "include": true }));
        }

        let mut log = start_log(model_config, &payload)?;

        let response = match self.post_chat_completions(model_config, &payload).await {
            // Mandatory-reasoning endpoints reject the disable request, so
            // downgrade to the lowest effort they all accept and retry once.
            Err(error) if sent_reasoning_disable && is_mandatory_reasoning_error(&error) => {
                let _ = log.error(&error);
                payload["reasoning"] = json!({ "effort": "low" });
                log = start_log(model_config, &payload)?;
                self.post_chat_completions(model_config, &payload).await
            }
            result => result,
        }
        .inspect_err(|e| {
            let _ = log.error(e);
        })?;

        stream_openai_compat(response, log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_providers::base::ProviderDescriptor;

    fn model_config(model_name: &str) -> ModelConfig {
        ModelConfig {
            model_name: model_name.to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params: None,
            reasoning: None,
            request_headers: None,
        }
    }

    #[test]
    fn metadata_includes_openrouter_parameters_config_key() {
        let metadata = OpenRouterProvider::metadata();

        assert!(metadata
            .config_keys
            .iter()
            .any(|key| key.name == OPENROUTER_PARAMETERS_CONFIG_KEY));
    }

    #[test]
    fn parse_openrouter_parameters_accepts_object_value() {
        let params = parse_openrouter_parameters(json!({
            "verbosity": "xhigh",
            "reasoning": { "effort": "high" }
        }))
        .unwrap();

        assert_eq!(params["verbosity"], json!("xhigh"));
        assert_eq!(params["reasoning"], json!({ "effort": "high" }));
    }

    #[test]
    fn parse_openrouter_parameters_accepts_json_string_value() {
        let params = parse_openrouter_parameters(json!(
            r#"{"plugins":[{"id":"web"}],"reasoning":{"max_tokens":2000}}"#
        ))
        .unwrap();

        assert_eq!(params["plugins"], json!([{ "id": "web" }]));
        assert_eq!(params["reasoning"], json!({ "max_tokens": 2000 }));
    }

    #[test]
    fn parse_openrouter_parameters_rejects_non_object_json_string() {
        let err = parse_openrouter_parameters(json!(r#"["web"]"#)).unwrap_err();

        assert!(err
            .to_string()
            .contains("OPENROUTER_PARAMETERS must be a JSON object"));
    }

    #[test]
    fn merge_openrouter_parameters_updates_model_request_params() {
        let mut model = model_config("anthropic/claude-sonnet-4");
        model.request_params = Some(HashMap::from([("verbosity".to_string(), json!("low"))]));

        let params = parse_openrouter_parameters(json!({
            "plugins": [{ "id": "web" }],
            "verbosity": "xhigh"
        }))
        .unwrap();

        merge_openrouter_parameters(&mut model, params);

        let request_params = model.request_params.as_ref().unwrap();
        assert_eq!(request_params["plugins"], json!([{ "id": "web" }]));
        assert_eq!(request_params["verbosity"], json!("xhigh"));
    }

    #[tokio::test]
    async fn stream_downgrades_reasoning_disable_on_mandatory_endpoint() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/chat/completions"))
            .and(body_partial_json(
                json!({ "reasoning": { "enabled": false } }),
            ))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": { "message": "Reasoning is mandatory for this endpoint and cannot be disabled." }
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/chat/completions"))
            .and(body_partial_json(
                json!({ "reasoning": { "effort": "low" } }),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: [DONE]\n\n"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = OpenRouterProvider {
            api_client: ApiClient::new_with_tls(
                server.uri(),
                AuthMethod::BearerToken("test-key".to_string()),
                None,
            )
            .unwrap(),
            supports_streaming: true,
            name: OPENROUTER_PROVIDER_NAME.to_string(),
            configured_parameters: None,
        };

        let mut config = model_config("google/gemini-3.5-flash");
        config.reasoning = Some(true);
        config.request_params = Some(HashMap::from([(
            "thinking_effort".to_string(),
            json!("off"),
        )]));

        let _stream = provider
            .stream(&config, "system", &[Message::user().with_text("hi")], &[])
            .await
            .unwrap();
    }

    #[test]
    fn gemini_tool_result_schema_ref_keys_are_escaped_reversibly() {
        let mut payload = json!({
            "messages": [
                {
                    "role": "assistant",
                    "content": "Keep {'$ref': '#/components/schemas/AssistantText'} unchanged"
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "{\"$ref\": \"#/components/schemas/Usage\", \"nested\": {\"$ref\" : \"#/components/schemas/Base64Image\"}, \"items\": [{\"$ref\": \"#/components/schemas/Item\"}], \"description\": \"use $ref here\", \"literal\": \"$ref\", \"identifier\": \"$reference\"}"
                }
            ]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            3
        );
        assert_eq!(
            payload["messages"][0]["content"],
            "Keep {'$ref': '#/components/schemas/AssistantText'} unchanged"
        );
        assert_eq!(
            payload["messages"][1]["content"],
            format!(
                "{}{{\"dollar_ref\": \"#/components/schemas/Usage\", \"nested\": {{\"dollar_ref\" : \"#/components/schemas/Base64Image\"}}, \"items\": [{{\"dollar_ref\": \"#/components/schemas/Item\"}}], \"description\": \"use $ref here\", \"literal\": \"$ref\", \"identifier\": \"$reference\"}}",
                gemini_schema_ref_note("dollar_ref")
            )
        );
        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
    }

    #[test]
    fn gemini_compatibility_only_applies_to_gemini_models() {
        let tool_result = r##"{"$ref":"#/components/schemas/Usage"}"##;
        let mut gemma_payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": tool_result
            }]
        });
        let original_gemma_payload = gemma_payload.clone();

        apply_gemini_compatibility("google/gemma-3-27b-it", &mut gemma_payload, &[]);

        assert_eq!(gemma_payload, original_gemma_payload);

        let mut anthropic_payload = original_gemma_payload.clone();
        apply_gemini_compatibility("anthropic/claude-sonnet-4.5", &mut anthropic_payload, &[]);

        assert_eq!(anthropic_payload, original_gemma_payload);

        let mut gemini_payload = original_gemma_payload;
        apply_gemini_compatibility("google/gemini-2.5-flash", &mut gemini_payload, &[]);

        assert_eq!(
            gemini_payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref\":\"#/components/schemas/Usage\"}}",
                gemini_schema_ref_note("dollar_ref")
            )
        );
        assert!(is_gemini_model("google/gemini-2.0-flash-exp:free"));
        assert!(!is_gemini_model("google/gemma-3-27b-it"));
    }

    #[test]
    fn gemini_schema_ref_escape_leaves_values_unchanged() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"description\":\"use $ref here\",\"literal\":\"$ref\",\"identifier\":\"$reference\"}"
            }]
        });
        let original = payload.clone();

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(payload, original);
    }

    #[test]
    fn gemini_schema_ref_escape_leaves_non_json_prose_unchanged() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "log entry, \"$ref\": not a JSON key"
            }]
        });
        let original = payload.clone();

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(payload, original);
    }

    #[test]
    fn gemini_schema_ref_escape_preserves_duplicate_ref_members() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\":\"A\",\"$ref\":\"B\"}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            2
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref\":\"A\",\"dollar_ref\":\"B\"}}",
                gemini_schema_ref_note("dollar_ref")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_preserves_unrelated_duplicate_members() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"duplicate\":\"first\",\"$ref\":\"A\",\"duplicate\":\"second\"}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"duplicate\":\"first\",\"dollar_ref\":\"A\",\"duplicate\":\"second\"}}",
                gemini_schema_ref_note("dollar_ref")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_rewrites_json_embedded_in_prose() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "before output { \"$ref\" : \"A\" } after output"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}before output {{ \"dollar_ref\" : \"A\" }} after output",
                gemini_schema_ref_note("dollar_ref")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_rewrites_multiple_occurrences_in_one_result() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\":\"A\",\"nested\":{\"$ref\":\"B\"}}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            2
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref\":\"A\",\"nested\":{{\"dollar_ref\":\"B\"}}}}",
                gemini_schema_ref_note("dollar_ref")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_matches_decoded_keys_and_collisions() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": r#"{"\u0024ref":"A","\u0064ollar_ref":"B"}"#
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                r#"{}{{"dollar_ref_2":"A","\u0064ollar_ref":"B"}}"#,
                gemini_schema_ref_note("dollar_ref_2")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_honors_escaped_quotes_and_backslashes() {
        let original_content =
            r#"{"text":"escaped quote \" then \\ and \"$ref\": still text","$ref":"A"}"#;
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": original_content
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                r#"{}{{"text":"escaped quote \" then \\ and \"$ref\": still text","dollar_ref":"A"}}"#,
                gemini_schema_ref_note("dollar_ref")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_preserves_absent_content_bytes() {
        let original_content = "prefix { \"ordinary\" : [ 1, 2 ] } suffix";
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": original_content
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(
            payload["messages"][0]["content"]
                .as_str()
                .unwrap()
                .as_bytes(),
            original_content.as_bytes()
        );
    }

    #[test]
    fn gemini_schema_ref_escape_tolerates_malformed_json() {
        let mut payload = json!({
            "messages": [
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "{\"$ref\": \"A\""
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_2",
                    "content": "}"
                }
            ]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref\": \"A\"",
                gemini_schema_ref_note("dollar_ref")
            )
        );
        assert_eq!(payload["messages"][1]["content"], "}");
    }

    #[test]
    fn gemini_schema_ref_escape_avoids_existing_safe_key() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\"}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref_2\":\"A\",\"dollar_ref\":\"B\"}}",
                gemini_schema_ref_note("dollar_ref_2")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_advances_past_multiple_key_collisions() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\",\"dollar_ref_2\":\"C\"}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref_3\":\"A\",\"dollar_ref\":\"B\",\"dollar_ref_2\":\"C\"}}",
                gemini_schema_ref_note("dollar_ref_3")
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_advances_past_deep_collision_ladder() {
        let mut payload = json!({
            "messages": [
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "{\"$ref\":\"A\",\"dollar_ref\":\"B\",\"nested\":{\"dollar_ref_2\":\"C\",\"items\":[{\"dollar_ref_3\":\"D\"},{\"dollar_ref_4\":\"E\"}]}}"
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_2",
                    "content": "{\"dollar_ref_5\":\"F\"}"
                }
            ]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            format!(
                "{}{{\"dollar_ref_6\":\"A\",\"dollar_ref\":\"B\",\"nested\":{{\"dollar_ref_2\":\"C\",\"items\":[{{\"dollar_ref_3\":\"D\"}},{{\"dollar_ref_4\":\"E\"}}]}}}}",
                gemini_schema_ref_note("dollar_ref_6")
            )
        );
        assert_eq!(
            payload["messages"][1]["content"],
            "{\"dollar_ref_5\":\"F\"}"
        );
    }

    #[test]
    fn gemini_schema_ref_escape_ignores_non_tool_content_and_safe_tool_results() {
        let mut payload = json!({
            "messages": [
                { "role": "user", "content": "{\"$ref\":\"#/components/schemas/UserText\"}" },
                { "role": "assistant", "content": "{\"$ref\":\"#/components/schemas/AssistantText\"}" },
                { "role": "tool", "tool_call_id": "call_1", "content": "ordinary output" },
                { "role": "tool", "tool_call_id": "call_2", "content": [{ "type": "text", "text": "{'$ref': '#/components/schemas/Structured'}" }] }
            ]
        });
        let original = payload.clone();

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(payload, original);
    }

    #[test]
    fn gemini_schema_ref_escape_ignores_braces_inside_string_literals() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": r#"{"a":"}}}}","$ref":"B"}"#
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            Value::String(
                gemini_schema_ref_note("dollar_ref") + r#"{"a":"}}}}","dollar_ref":"B"}"#
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_ignores_brackets_inside_string_literals() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": r#"{"a":"[[[","$ref":"B"}"#
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            Value::String(gemini_schema_ref_note("dollar_ref") + r#"{"a":"[[[","dollar_ref":"B"}"#)
        );
    }

    #[test]
    fn gemini_schema_ref_escape_avoids_safe_key_used_in_another_tool_result() {
        let untouched = r#"{"dollar_ref":"x"}"#;
        let mut payload = json!({
            "messages": [
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": r#"{"$ref":"A"}"#
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_2",
                    "content": untouched
                }
            ]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            Value::String(gemini_schema_ref_note("dollar_ref_2") + r#"{"dollar_ref_2":"A"}"#)
        );
        assert_eq!(
            payload["messages"][1]["content"],
            Value::String(untouched.to_string())
        );
    }

    #[test]
    fn gemini_schema_ref_escape_leaves_embedded_json_string_values_unchanged() {
        let original = r#"{"payload":"{\"$ref\":\"A\"}"}"#;
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": original
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(
            payload["messages"][0]["content"],
            Value::String(original.to_string())
        );
    }

    #[test]
    fn gemini_schema_ref_escape_preserves_multibyte_content() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"\u{540d}\u{524d}\":\"\u{1f389} ok\",\"$ref\":\"A\"}"
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            1
        );
        assert_eq!(
            payload["messages"][0]["content"],
            Value::String(
                gemini_schema_ref_note("dollar_ref")
                    + "{\"\u{540d}\u{524d}\":\"\u{1f389} ok\",\"dollar_ref\":\"A\"}"
            )
        );
    }

    #[test]
    fn gemini_schema_ref_escape_is_idempotent() {
        let mut payload = json!({
            "messages": [{
                "role": "tool",
                "tool_call_id": "call_1",
                "content": r#"{"$ref":"A","b":[{"$ref":"B"}]}"#
            }]
        });

        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            2
        );
        let after_first_pass = payload.clone();
        assert_eq!(
            escape_gemini_schema_ref_keys_in_tool_responses(&mut payload),
            0
        );
        assert_eq!(payload, after_first_pass);
    }
}
