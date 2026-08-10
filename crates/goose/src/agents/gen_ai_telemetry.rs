use crate::conversation::message::{Message, MessageContent, ToolResult};
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use rmcp::model::{CallToolRequestParams, CallToolResult, Role};
use serde_json::{json, Value};
use tracing::Span;

pub(super) const CAPTURE_MESSAGE_CONTENT_ENV: &str =
    "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT";

pub(super) fn capture_message_content() -> bool {
    std::env::var(CAPTURE_MESSAGE_CONTENT_ENV).is_ok_and(|value| value.eq_ignore_ascii_case("true"))
}

pub(super) fn input_messages_json(messages: &[Message]) -> String {
    Value::Array(messages.iter().map(message_json).collect()).to_string()
}

pub(super) fn simple_input_json(text: &str) -> String {
    json!([{"role": "user", "content": text}]).to_string()
}

pub(super) fn simple_output_json(text: &str) -> String {
    json!([{"role": "assistant", "content": text, "finish_reason": "stop"}]).to_string()
}

pub(super) fn output_message_json(message: &Message) -> String {
    // Message does not retain provider finish reasons; tool requests are the only
    // distinct completion signal available after streaming.
    let finish_reason = if message.content.iter().any(|content| {
        matches!(
            content,
            MessageContent::ToolRequest(_) | MessageContent::FrontendToolRequest(_)
        )
    }) {
        "tool_call"
    } else {
        "stop"
    };
    let mut value = message_json(message);
    value["finish_reason"] = Value::String(finish_reason.to_string());
    Value::Array(vec![value]).to_string()
}

pub(super) fn append_message(accumulated: &mut Option<Message>, message: &Message) {
    match accumulated {
        Some(accumulated) => accumulated.content.extend(message.content.iter().cloned()),
        None => *accumulated = Some(message.clone()),
    }
}

pub(super) fn record_usage(span: &Span, usage: &Usage) {
    if let Some(tokens) = usage.input_tokens {
        span.record("gen_ai.usage.input_tokens", tokens);
    }
    if let Some(tokens) = usage.output_tokens {
        span.record("gen_ai.usage.output_tokens", tokens);
    }
    if let Some(tokens) = usage.cache_read_input_tokens {
        span.record("gen_ai.usage.cache_read.input_tokens", tokens);
    }
    if let Some(tokens) = usage.cache_write_input_tokens {
        span.record("gen_ai.usage.cache_creation.input_tokens", tokens);
    }
}

pub(super) fn record_provider_usage(span: &Span, usage: &ProviderUsage) {
    span.record("gen_ai.response.model", usage.model.as_str());
    record_usage(span, &usage.usage);
}

pub(super) fn tool_result_json(result: &ToolResult<CallToolResult>) -> String {
    match result {
        Ok(result) if result.is_error != Some(true) => json!({
            "status": "success",
            "value": result,
        }),
        Ok(result) => json!({
            "status": "error",
            "value": result,
        }),
        Err(error) => json!({
            "status": "error",
            "error": error.to_string(),
        }),
    }
    .to_string()
}

pub(super) fn successful_tool_result_json(result: &ToolResult<CallToolResult>) -> Option<String> {
    match result {
        Ok(result) if result.is_error != Some(true) => {
            Some(serde_json::to_string(result).expect("CallToolResult must serialize"))
        }
        _ => None,
    }
}

fn message_json(message: &Message) -> Value {
    let role = if !message.content.is_empty()
        && message
            .content
            .iter()
            .all(|content| matches!(content, MessageContent::ToolResponse(_)))
    {
        "tool"
    } else {
        match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    };

    let parts = consolidated_parts(&message.content);
    json!({
        "role": role,
        "parts": parts,
    })
}

/// Merge consecutive text and reasoning parts into single entries so that
/// streaming tokens don't each get their own JSON object in the OTEL output.
fn consolidated_parts(content: &[MessageContent]) -> Vec<Value> {
    let mut result: Vec<Value> = Vec::new();
    for item in content {
        let value = message_part_json(item);
        let item_type = value.get("type").and_then(|v| v.as_str());
        if matches!(item_type, Some("text" | "reasoning")) {
            if let Some(last) = result.last_mut() {
                if last.get("type") == value.get("type") {
                    if let (Some(existing), Some(new_content)) = (
                        last.get("content").and_then(|v| v.as_str()),
                        value.get("content").and_then(|v| v.as_str()),
                    ) {
                        last["content"] = Value::String(format!("{}{}", existing, new_content));
                        continue;
                    }
                }
            }
        }
        result.push(value);
    }
    result
}

fn tool_call_part(id: &str, tool_call: &ToolResult<CallToolRequestParams>) -> Value {
    match tool_call {
        Ok(tool_call) => json!({
            "type": "tool_call",
            "id": id,
            "name": tool_call.name,
            "arguments": tool_call
                .arguments
                .as_ref()
                .map(|arguments| Value::Object(arguments.clone()))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        }),
        Err(error) => json!({
            "type": "tool_call_error",
            "id": id,
            "error": error.to_string(),
        }),
    }
}

fn message_part_json(content: &MessageContent) -> Value {
    match content {
        MessageContent::Text(text) => json!({
            "type": "text",
            "content": text.text,
        }),
        MessageContent::Image(image) => json!({
            "type": "blob",
            "modality": "image",
            "mime_type": image.mime_type,
            "content": image.data,
        }),
        MessageContent::ToolRequest(request) => tool_call_part(&request.id, &request.tool_call),
        MessageContent::ToolResponse(response) => json!({
            "type": "tool_call_response",
            "id": response.id,
            "response": match &response.tool_result {
                Ok(result) => serde_json::to_value(result)
                    .expect("CallToolResult must serialize"),
                Err(error) => json!({ "error": error.to_string() }),
            },
        }),
        MessageContent::FrontendToolRequest(request) => {
            tool_call_part(&request.id, &request.tool_call)
        }
        MessageContent::Thinking(thinking) => json!({
            "type": "reasoning",
            "content": thinking.thinking,
        }),
        MessageContent::RedactedThinking(_) => json!({
            "type": "redacted_reasoning",
        }),
        MessageContent::ToolConfirmationRequest(request) => json!({
            "type": "tool_confirmation",
            "id": request.id,
            "name": request.tool_name,
            "arguments": request.arguments,
        }),
        MessageContent::ActionRequired(action) => json!({
            "type": "action_required",
            "data": action.data,
        }),
        MessageContent::SystemNotification(notification) => json!({
            "type": "system_notification",
            "content": notification.msg,
        }),
        MessageContent::Error(error) => json!({
            "type": "error",
            "kind": error.kind,
            "content": error.message,
        }),
    }
}

#[cfg(test)]
pub(super) mod test_support {
    use serde_json::{Map, Number, Value};
    use std::fmt;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{subscriber::DefaultGuard, Subscriber};
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::registry::LookupSpan;
    use tracing_subscriber::Layer;

    #[derive(Clone)]
    pub struct SpanFieldCapture {
        span_name: &'static str,
        fields: Arc<Mutex<Map<String, Value>>>,
    }

    impl SpanFieldCapture {
        pub fn new(span_name: &'static str) -> Self {
            Self {
                span_name,
                fields: Arc::new(Mutex::new(Map::new())),
            }
        }

        pub fn fields(&self) -> Map<String, Value> {
            self.fields.lock().unwrap().clone()
        }

        pub fn set_default(self) -> DefaultGuard {
            use tracing_subscriber::prelude::*;

            tracing::subscriber::set_default(tracing_subscriber::registry().with(self))
        }
    }

    impl<S> Layer<S> for SpanFieldCapture
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
            if attrs.metadata().name() == self.span_name {
                attrs.record(&mut FieldVisitor {
                    fields: &self.fields,
                });
            }
        }

        fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
            if ctx
                .span(id)
                .is_some_and(|span| span.metadata().name() == self.span_name)
            {
                values.record(&mut FieldVisitor {
                    fields: &self.fields,
                });
            }
        }
    }

    struct FieldVisitor<'a> {
        fields: &'a Mutex<Map<String, Value>>,
    }

    impl FieldVisitor<'_> {
        fn insert(&self, field: &Field, value: Value) {
            self.fields
                .lock()
                .unwrap()
                .insert(field.name().to_string(), value);
        }
    }

    impl Visit for FieldVisitor<'_> {
        fn record_i64(&mut self, field: &Field, value: i64) {
            self.insert(field, Value::Number(value.into()));
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.insert(field, Value::Number(value.into()));
        }

        fn record_f64(&mut self, field: &Field, value: f64) {
            if let Some(value) = Number::from_f64(value) {
                self.insert(field, Value::Number(value));
            }
        }

        fn record_bool(&mut self, field: &Field, value: bool) {
            self.insert(field, Value::Bool(value));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.insert(field, Value::String(value.to_string()));
        }

        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            self.insert(field, Value::String(format!("{value:?}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_test_support::otel::clear_otel_env;
    use rmcp::{model::CallToolRequestParams, object};

    #[test]
    fn content_capture_requires_explicit_opt_in() {
        let _env = clear_otel_env(&[]);
        assert!(!capture_message_content());

        drop(_env);
        let _env = clear_otel_env(&[(CAPTURE_MESSAGE_CONTENT_ENV, "TrUe")]);
        assert!(capture_message_content());
    }

    #[test]
    fn messages_follow_gen_ai_semantic_convention_shape() {
        let request = CallToolRequestParams::new("get_weather")
            .with_arguments(object!({ "location": "Paris" }));
        let messages = vec![
            Message::user().with_text("Weather?"),
            Message::assistant().with_tool_request("call-1", Ok(request)),
        ];

        let value: Value = serde_json::from_str(&input_messages_json(&messages)).unwrap();
        assert_eq!(value[0]["role"], "user");
        assert_eq!(value[0]["parts"][0]["type"], "text");
        assert_eq!(value[0]["parts"][0]["content"], "Weather?");
        assert_eq!(value[1]["parts"][0]["type"], "tool_call");
        assert_eq!(value[1]["parts"][0]["name"], "get_weather");
        assert_eq!(value[1]["parts"][0]["arguments"]["location"], "Paris");
    }

    #[test]
    fn output_messages_include_finish_reason() {
        let message = Message::assistant().with_text("Sunny");
        let value: Value = serde_json::from_str(&output_message_json(&message)).unwrap();

        assert_eq!(value[0]["role"], "assistant");
        assert_eq!(value[0]["finish_reason"], "stop");
        assert_eq!(value[0]["parts"][0]["content"], "Sunny");

        let request = CallToolRequestParams::new("get_weather");
        let message = Message::assistant().with_tool_request("call-1", Ok(request));
        let value: Value = serde_json::from_str(&output_message_json(&message)).unwrap();
        assert_eq!(value[0]["finish_reason"], "tool_call");
    }
}
