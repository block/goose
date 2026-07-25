use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[derive(Clone)]
enum ApiResponse {
    Reply(String),
    ToolCall { name: String, arguments: String },
    ContextLimitError(String),
    ServerError(String),
    ReplyThenServerError { reply: String, error: String },
}

struct ApiRule {
    matcher: ApiMatcher,
    response: ApiResponse,
}

enum ApiMatcher {
    InputContains(String),
    SystemContains(String),
}

struct DummyApiState {
    rules: Mutex<Vec<ApiRule>>,
    calls: Mutex<Vec<ApiCall>>,
    next_response_id: AtomicUsize,
}

pub(super) struct DummyApi {
    server: MockServer,
    state: Arc<DummyApiState>,
}

#[derive(Clone)]
pub(super) struct ApiCall {
    body: Value,
}

impl ApiCall {
    pub(super) fn input_tokens(&self) -> i32 {
        serialized_chars(&self.body)
    }

    pub(super) fn input_contains(&self, needle: &str) -> bool {
        request_input(&self.body).contains(needle)
    }

    pub(super) fn advertises_tool(&self, name: &str) -> bool {
        self.body["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|tool| tool["function"]["name"].as_str() == Some(name))
    }
}

impl DummyApi {
    pub(super) async fn start() -> Self {
        let server = MockServer::start().await;
        let state = Arc::new(DummyApiState {
            rules: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
            next_response_id: AtomicUsize::new(1),
        });
        let responder = state.clone();
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(move |request: &Request| responder.respond(request))
            .mount(&server)
            .await;
        Self { server, state }
    }

    pub(super) fn uri(&self) -> String {
        self.server.uri()
    }

    pub(super) fn on(&self, needle: impl Into<String>) -> ApiRuleBuilder<'_> {
        ApiRuleBuilder {
            api: self,
            matcher: ApiMatcher::InputContains(needle.into()),
        }
    }

    pub(super) fn on_system(&self, needle: impl Into<String>) -> ApiRuleBuilder<'_> {
        ApiRuleBuilder {
            api: self,
            matcher: ApiMatcher::SystemContains(needle.into()),
        }
    }

    pub(super) fn calls(&self) -> Vec<ApiCall> {
        self.state.calls.lock().unwrap().clone()
    }

    pub(super) fn call_count(&self) -> usize {
        self.state.calls.lock().unwrap().len()
    }

    fn add_rule(&self, matcher: ApiMatcher, response: ApiResponse) -> usize {
        let mut rules = self.state.rules.lock().unwrap();
        rules.push(ApiRule { matcher, response });
        rules.len() - 1
    }
}

pub(super) struct ApiRuleBuilder<'a> {
    api: &'a DummyApi,
    matcher: ApiMatcher,
}

pub(super) struct ConfiguredResponse<'a> {
    api: &'a DummyApi,
    rule: usize,
}

impl<'a> ApiRuleBuilder<'a> {
    pub(super) fn reply(self, text: impl Into<String>) -> ConfiguredResponse<'a> {
        self.configured(ApiResponse::Reply(text.into()))
    }

    pub(super) fn call(self, name: impl Into<String>, arguments: Value) -> ConfiguredResponse<'a> {
        self.configured(ApiResponse::ToolCall {
            name: name.into(),
            arguments: arguments.to_string(),
        })
    }

    pub(super) fn malformed_tool_call(
        self,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> ConfiguredResponse<'a> {
        self.configured(ApiResponse::ToolCall {
            name: name.into(),
            arguments: arguments.into(),
        })
    }

    pub(super) fn context_limit_error(self, message: impl Into<String>) -> ConfiguredResponse<'a> {
        self.configured(ApiResponse::ContextLimitError(message.into()))
    }

    pub(super) fn server_error(self, message: impl Into<String>) -> ConfiguredResponse<'a> {
        self.configured(ApiResponse::ServerError(message.into()))
    }

    fn configured(self, response: ApiResponse) -> ConfiguredResponse<'a> {
        ConfiguredResponse {
            api: self.api,
            rule: self.api.add_rule(self.matcher, response),
        }
    }
}

impl<'a> ConfiguredResponse<'a> {
    pub(super) fn server_error(self, error: impl Into<String>) -> &'a DummyApi {
        let mut rules = self.api.state.rules.lock().unwrap();
        let response = &mut rules[self.rule].response;
        let ApiResponse::Reply(reply) = response else {
            panic!("server_error can only follow reply");
        };
        *response = ApiResponse::ReplyThenServerError {
            reply: std::mem::take(reply),
            error: error.into(),
        };
        self.api
    }
}

impl DummyApiState {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = request.body_json().expect("OpenAI request body");
        self.calls
            .lock()
            .unwrap()
            .push(ApiCall { body: body.clone() });

        let input_tokens = serialized_chars(&body);
        let model = body["model"].as_str().expect("OpenAI request model");
        let context_limit = goose_providers::model::ModelConfig::new(model)
            .with_canonical_limits("openai")
            .context_limit();
        if input_tokens as usize > context_limit {
            return context_limit_response(input_tokens, context_limit);
        }

        let input = request_input(&body);
        let system = request_system(&body);
        let response = {
            let rules = self.rules.lock().unwrap();
            let rule = rules
                .iter()
                .rev()
                .find(|rule| match &rule.matcher {
                    ApiMatcher::InputContains(needle) => input.contains(needle),
                    ApiMatcher::SystemContains(needle) => system.contains(needle),
                })
                .unwrap_or_else(|| {
                    panic!("dummy API has no rule matching input {input:?}, system {system:?}")
                });
            rule.response.clone()
        };

        let id = format!(
            "chatcmpl-test-{}",
            self.next_response_id.fetch_add(1, Ordering::Relaxed)
        );
        match response {
            ApiResponse::Reply(text) => sse_response(reply_events(
                &id,
                model,
                &text,
                input_tokens,
                text.chars().count() as i32,
                None,
            )),
            ApiResponse::ToolCall { name, arguments } => {
                let output_tokens = name.chars().count() as i32 + arguments.chars().count() as i32;
                sse_response(tool_call_events(
                    &id,
                    model,
                    &name,
                    &arguments,
                    input_tokens,
                    output_tokens,
                ))
            }
            ApiResponse::ContextLimitError(message) => ResponseTemplate::new(400).set_body_json(
                context_limit_error(format!("context_length_exceeded: {message}")),
            ),
            ApiResponse::ServerError(message) => {
                sse_response(format!("data: {}\n\n", api_error(message)))
            }
            ApiResponse::ReplyThenServerError { reply, error } => sse_response(reply_events(
                &id,
                model,
                &reply,
                input_tokens,
                reply.chars().count() as i32,
                Some(&error),
            )),
        }
    }
}

fn serialized_chars(value: &Value) -> i32 {
    value.to_string().chars().count() as i32
}

fn context_limit_response(input_tokens: i32, context_limit: usize) -> ResponseTemplate {
    ResponseTemplate::new(400).set_body_json(context_limit_error(format!(
        "This model's maximum context length is {context_limit} tokens, but the request contains {input_tokens} tokens"
    )))
}

fn context_limit_error(message: impl Into<String>) -> Value {
    json!({
        "error": {
            "message": message.into(),
            "type": "invalid_request_error",
            "code": "context_length_exceeded"
        }
    })
}

fn request_input(body: &Value) -> String {
    let mut values = Vec::new();
    for message in body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|message| message["role"] != "system")
    {
        collect_strings(message, &mut values);
    }
    values.join("\n")
}

fn request_system(body: &Value) -> String {
    let mut values = Vec::new();
    for message in body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|message| message["role"] == "system")
    {
        collect_strings(message, &mut values);
    }
    values.join("\n")
}

fn collect_strings<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
    match value {
        Value::String(value) => strings.push(value),
        Value::Array(values) => {
            for value in values {
                collect_strings(value, strings);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_strings(value, strings);
            }
        }
        _ => {}
    }
}

fn reply_events(
    id: &str,
    model: &str,
    text: &str,
    input_tokens: i32,
    output_tokens: i32,
    error: Option<&str>,
) -> String {
    let mut events = String::new();
    for chunk in split_reply(text) {
        push_event(
            &mut events,
            json!({
                "id": id,
                "object": "chat.completion.chunk",
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": { "content": chunk },
                    "finish_reason": null
                }]
            }),
        );
    }
    push_event(
        &mut events,
        json!({
            "id": id,
            "object": "chat.completion.chunk",
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        }),
    );
    push_event(
        &mut events,
        usage_event(id, model, input_tokens, output_tokens),
    );
    if let Some(error) = error {
        push_event(&mut events, api_error(error));
    } else {
        events.push_str("data: [DONE]\n\n");
    }
    events
}

fn tool_call_events(
    id: &str,
    model: &str,
    name: &str,
    arguments: &str,
    input_tokens: i32,
    output_tokens: i32,
) -> String {
    let argument_chunks = split_arguments(arguments);
    let tool_call_id = format!(
        "dummy-tool-call-{}",
        id.strip_prefix("chatcmpl-test-").unwrap()
    );
    let mut events = String::new();
    push_event(
        &mut events,
        json!({
            "id": id,
            "object": "chat.completion.chunk",
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": tool_call_id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": argument_chunks.first().cloned().unwrap_or_default()
                        }
                    }]
                },
                "finish_reason": null
            }]
        }),
    );
    for (index, chunk) in argument_chunks.iter().enumerate().skip(1) {
        let finish_reason = (index + 1 == argument_chunks.len()).then_some("tool_calls");
        push_event(
            &mut events,
            json!({
                "id": id,
                "object": "chat.completion.chunk",
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": {
                        "tool_calls": [{
                            "index": 0,
                            "function": { "arguments": chunk }
                        }]
                    },
                    "finish_reason": finish_reason
                }],
            }),
        );
    }
    push_event(
        &mut events,
        usage_event(id, model, input_tokens, output_tokens),
    );
    events.push_str("data: [DONE]\n\n");
    events
}

fn usage_event(id: &str, model: &str, input_tokens: i32, output_tokens: i32) -> Value {
    json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens
        }
    })
}

fn api_error(message: impl Into<String>) -> Value {
    json!({
        "error": {
            "message": message.into(),
            "type": "server_error"
        }
    })
}

fn push_event(events: &mut String, value: Value) {
    events.push_str("data: ");
    events.push_str(&value.to_string());
    events.push_str("\n\n");
}

fn split_reply(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut spaces = 0;
    for (index, character) in text.char_indices() {
        if character == ' ' {
            spaces += 1;
            if spaces == 2 {
                let end = index + character.len_utf8();
                chunks.push(text[start..end].to_string());
                start = end;
                spaces = 0;
            }
        }
    }
    if start < text.len() {
        chunks.push(text[start..].to_string());
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

fn split_arguments(arguments: &str) -> Vec<String> {
    let characters = arguments.chars().collect::<Vec<_>>();
    let midpoint = characters.len().div_ceil(2);
    vec![
        characters[..midpoint].iter().collect(),
        characters[midpoint..].iter().collect(),
    ]
}

fn sse_response(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body, "text/event-stream")
}
