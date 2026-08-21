use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use goose_providers::images::ImageFormat;
use serde_json::{json, Value};
use std::collections::HashMap;

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
            .with_header("X-Title", "goose")?
            .with_header("X-OpenRouter-Categories", "cli-agent,productivity")?;

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
    model_name.starts_with("google/")
}

/// Literal token Google's Gemini backend misreads inside a `function_response`.
const GEMINI_REF_TOKEN: &str = "$ref";
const GEMINI_REF_REPLACEMENT: &str = "dollar_ref";
const GEMINI_REF_NOTE: &str = "[goose compatibility note] In the tool output below, every JSON Schema reference key was rewritten to \"dollar_ref\". The original key is a dollar sign followed by \"ref\", and the referenced values are unchanged.\n\n";

/// Rewrite `$ref` only where it stands as a complete token. Tool output is
/// usually file contents or command output, so a bare substring replace would
/// also corrupt legitimate identifiers such as `$refs` or `$refresh_token`.
fn rewrite_ref_tokens(text: &str) -> Option<String> {
    let is_identifier_char = |c: char| c.is_ascii_alphanumeric() || c == '_';

    let mut segments = text.split(GEMINI_REF_TOKEN);
    let mut out = String::with_capacity(text.len());
    out.push_str(segments.next()?);
    let mut replaced = false;

    for segment in segments {
        if segment.starts_with(is_identifier_char) {
            out.push_str(GEMINI_REF_TOKEN);
        } else {
            out.push_str(GEMINI_REF_REPLACEMENT);
            replaced = true;
        }
        out.push_str(segment);
    }

    replaced.then_some(out)
}

/// Google rejects OpenAI `role: "tool"` content containing the literal `$ref`
/// key: it reads the key as Gemini `function_response` metadata and returns
/// `400 INVALID_ARGUMENT`. Because Goose replays persisted history, one such
/// tool result breaks every later turn in the session.
///
/// Tool output is treated as opaque text; it is not required to parse as JSON.
fn rewrite_gemini_ref_token_in_tool_content(payload: &mut Value, model_name: &str) {
    if !is_gemini_model(model_name) {
        return;
    }

    let Some(messages) = payload.get_mut("messages").and_then(Value::as_array_mut) else {
        return;
    };

    for message in messages {
        if message.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let Some(content) = message.get_mut("content") else {
            continue;
        };
        let Some(rewritten) = content.as_str().and_then(rewrite_ref_tokens) else {
            continue;
        };
        *content = Value::String(format!("{GEMINI_REF_NOTE}{rewritten}"));
    }
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

        if !session_id.is_empty() {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("user".to_string(), Value::String(session_id.to_string()));
                obj.insert(
                    "session_id".to_string(),
                    Value::String(session_id.to_string()),
                );
            }
        }

        if CacheSemantics::for_model(OPENROUTER_PROVIDER_NAME, &model_config.model_name)
            .uses_explicit_breakpoints()
            && !model_config.prompt_cache_disabled()
        {
            apply_chat_payload_breakpoints(&mut payload);
        }

        if is_gemini_model(&model_config.model_name) {
            openrouter_format::add_reasoning_details_to_request(&mut payload, messages);
        }
        rewrite_gemini_ref_token_in_tool_content(&mut payload, &model_config.model_name);
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

    fn tool_payload(content: &str) -> Value {
        json!({
            "messages": [
                { "role": "system", "content": "sys $ref" },
                { "role": "user", "content": "user $ref" },
                { "role": "assistant", "content": "assistant $ref" },
                { "role": "tool", "content": content, "tool_call_id": "call_1" }
            ]
        })
    }

    fn tool_content(payload: &Value) -> &str {
        payload["messages"][3]["content"].as_str().unwrap()
    }

    #[test]
    fn gemini_tool_content_ref_key_is_rewritten_and_value_preserved() {
        let mut payload =
            tool_payload("{'properties': {'image': {'$ref': '#/components/schemas/Example'}}}");

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

        let content = tool_content(&payload);
        assert!(content.starts_with(GEMINI_REF_NOTE));
        assert!(content.ends_with(
            "{'properties': {'image': {'dollar_ref': '#/components/schemas/Example'}}}"
        ));
    }

    #[test]
    fn gemini_tool_content_rewrites_every_occurrence() {
        let mut payload = tool_payload("{'a': {'$ref': '#/x'}, 'b': {'$ref': '#/y'}}");

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-2.5-pro");

        assert!(tool_content(&payload)
            .ends_with("{'a': {'dollar_ref': '#/x'}, 'b': {'dollar_ref': '#/y'}}"));
    }

    #[test]
    fn identifiers_that_merely_start_with_the_token_are_preserved() {
        let source = "this.$refs.input; const $refresh_token = 1; $ref_count += $refs.len();";
        let mut payload = tool_payload(source);

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

        assert_eq!(tool_content(&payload), source);
    }

    #[test]
    fn token_is_rewritten_when_followed_by_a_non_identifier_char() {
        for (input, expected) in [
            ("\"$ref\":", "\"dollar_ref\":"),
            ("'$ref' =>", "'dollar_ref' =>"),
            ("trailing $ref", "trailing dollar_ref"),
            ("$ref.path", "dollar_ref.path"),
            ("$ref-dash", "dollar_ref-dash"),
        ] {
            let mut payload = tool_payload(input);
            rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");
            assert!(
                tool_content(&payload).ends_with(expected),
                "{input} should rewrite to {expected}"
            );
        }
    }

    #[test]
    fn mixed_content_rewrites_only_the_standalone_token() {
        let mut payload = tool_payload("$refs and {'$ref': '#/a'} and $refresh");

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

        assert!(tool_content(&payload).ends_with("$refs and {'dollar_ref': '#/a'} and $refresh"));
    }

    #[test]
    fn gemini_guard_leaves_other_roles_untouched() {
        let mut payload = tool_payload("{'$ref': '#/a'}");

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-2.5-flash");

        assert_eq!(payload["messages"][0]["content"], json!("sys $ref"));
        assert_eq!(payload["messages"][1]["content"], json!("user $ref"));
        assert_eq!(payload["messages"][2]["content"], json!("assistant $ref"));
    }

    #[test]
    fn non_google_models_are_untouched() {
        let original = tool_payload("{'$ref': '#/a'}");
        let mut payload = original.clone();

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "anthropic/claude-sonnet-4");

        assert_eq!(payload, original);
    }

    #[test]
    fn tool_content_without_the_token_is_unchanged() {
        for content in ["plain tool output", "this.$refs.input only"] {
            let original = tool_payload(content);
            let mut payload = original.clone();

            rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

            assert_eq!(payload, original, "{content} should be untouched");
        }
    }

    #[test]
    fn note_is_added_once_per_affected_tool_message_only() {
        let mut payload = json!({
            "messages": [
                { "role": "tool", "content": "{'$ref': '#/a'}", "tool_call_id": "a" },
                { "role": "tool", "content": "no marker here", "tool_call_id": "b" },
                { "role": "tool", "content": "{'$ref': '#/c'}", "tool_call_id": "c" }
            ]
        });

        rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

        let messages = payload["messages"].as_array().unwrap();
        for index in [0, 2] {
            let content = messages[index]["content"].as_str().unwrap();
            assert!(content.starts_with(GEMINI_REF_NOTE));
            assert_eq!(content.matches(GEMINI_REF_NOTE).count(), 1);
        }
        assert_eq!(messages[1]["content"], json!("no marker here"));
    }

    #[test]
    fn payloads_without_string_tool_content_are_untouched() {
        for original in [
            json!({}),
            json!({ "messages": "not an array" }),
            json!({ "messages": [{ "role": "tool", "tool_call_id": "a" }] }),
            json!({
                "messages": [{
                    "role": "tool",
                    "content": [{ "type": "text", "text": "$ref" }],
                    "tool_call_id": "a"
                }]
            }),
        ] {
            let mut payload = original.clone();

            rewrite_gemini_ref_token_in_tool_content(&mut payload, "google/gemini-3.7-flash");

            assert_eq!(payload, original);
        }
    }

    #[test]
    fn compatibility_note_matches_the_tokens_it_describes() {
        assert!(GEMINI_REF_NOTE.contains(GEMINI_REF_REPLACEMENT));
        assert!(!GEMINI_REF_NOTE.contains(GEMINI_REF_TOKEN));
    }
}
