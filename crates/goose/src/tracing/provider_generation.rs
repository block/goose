use goose_providers::base::Provider;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use serde_json::{json, Value};
use tracing_futures::Instrument;

use crate::conversation::message::{Message, MessageContent};

pub async fn complete_provider(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    system_prompt: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Message, ProviderUsage), ProviderError> {
    let session_id = crate::session_context::current_session_id().unwrap_or_default();
    let mut generation = ProviderGeneration::new(
        provider.get_name(),
        model_config,
        &session_id,
        system_prompt,
        messages,
        tools,
    );

    match provider
        .complete(model_config, system_prompt, messages, tools)
        .instrument(generation.span())
        .await
    {
        Ok((mut message, usage)) => {
            generation.record_usage(&usage);
            generation.attach_observation_id(&mut message);
            generation.record_output(&message);
            generation.finish();
            Ok((message, usage))
        }
        Err(error) => {
            generation.record_error(&error);
            generation.finish();
            Err(error)
        }
    }
}

pub(crate) struct ProviderGeneration {
    observation_id: String,
    provider_name: String,
    span: Option<tracing::Span>,
    output: Vec<Message>,
}

impl ProviderGeneration {
    pub(crate) fn new(
        provider_name: &str,
        model_config: &ModelConfig,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Self {
        let observation_id = uuid::Uuid::new_v4().to_string();
        let span = super::is_observation_layer_active().then(|| {
            tracing::info_span!(
                target: "goose::tracing::provider_generation",
                "provider_generation",
                observation_type = "GENERATION",
                observation_id = %observation_id,
                session.id = %session_id,
                provider = provider_name,
                model = %model_config.model_name,
                model_parameters_json = tracing::field::Empty,
                input_json = tracing::field::Empty,
                output_json = tracing::field::Empty,
                usage_details_json = tracing::field::Empty,
                cost_details_json = tracing::field::Empty,
                completion_start_time = tracing::field::Empty,
                status_message = tracing::field::Empty,
            )
        });

        if let Some(span) = span.as_ref() {
            record_json(
                span,
                "model_parameters_json",
                &langfuse_model_parameters(model_config),
            );
            record_json(
                span,
                "input_json",
                &json!({
                    "system": system_prompt,
                    "messages": messages,
                    "tools": tools,
                }),
            );
        }

        Self {
            observation_id,
            provider_name: provider_name.to_string(),
            span,
            output: Vec::new(),
        }
    }

    pub(crate) fn span(&self) -> tracing::Span {
        self.span
            .as_ref()
            .cloned()
            .unwrap_or_else(tracing::Span::none)
    }

    pub(crate) fn record_completion_start(&self) {
        if let Some(span) = self.span.as_ref() {
            let completion_start_time = chrono::Utc::now().to_rfc3339();
            span.record("completion_start_time", completion_start_time.as_str());
        }
    }

    pub(crate) fn record_error(&self, error: &impl std::fmt::Display) {
        if let Some(span) = self.span.as_ref() {
            let error_message = error.to_string();
            span.record("status_message", error_message.as_str());
        }
    }

    pub(crate) fn record_usage(&self, usage: &ProviderUsage) {
        let Some(span) = self.span.as_ref() else {
            return;
        };

        span.record("model", usage.model.as_str());
        let token_usage = &usage.usage;
        let mut usage_details = serde_json::Map::new();
        for (name, value) in [
            ("input", token_usage.input_tokens),
            ("output", token_usage.output_tokens),
            ("total", token_usage.total_tokens),
            (
                "cache_read_input_tokens",
                token_usage.cache_read_input_tokens,
            ),
            (
                "cache_write_input_tokens",
                token_usage.cache_write_input_tokens,
            ),
        ] {
            if let Some(value) = value {
                usage_details.insert(name.to_string(), json!(value));
            }
        }
        if !usage_details.is_empty() {
            record_json(span, "usage_details_json", &Value::Object(usage_details));
        }

        let cost = usage.cost.or_else(|| {
            crate::providers::canonical::maybe_get_canonical_model(
                &self.provider_name,
                &usage.model,
            )
            .and_then(|canonical| canonical.cost.estimate_cost(&usage.usage))
        });
        if let Some(cost) = cost {
            record_json(span, "cost_details_json", &json!({ "total": cost }));
        }
    }

    pub(crate) fn attach_observation_id(&self, message: &mut Message) {
        message.metadata = message
            .metadata
            .clone()
            .with_observation_id(self.observation_id.clone());
    }

    pub(crate) fn record_output(&mut self, message: &Message) {
        if self.span.is_some() {
            append_output(&mut self.output, message.clone());
        }
    }

    pub(crate) fn finish(&mut self) {
        let Some(span) = self.span.take() else {
            return;
        };
        if !self.output.is_empty() {
            record_json(
                &span,
                "output_json",
                &serde_json::to_value(&self.output).unwrap_or_default(),
            );
        }
    }
}

impl Drop for ProviderGeneration {
    fn drop(&mut self) {
        self.finish();
    }
}

fn record_json(span: &tracing::Span, field: &'static str, value: &Value) {
    if let Ok(serialized) = serde_json::to_string(value) {
        span.record(field, serialized.as_str());
    }
}

fn langfuse_model_parameters(model_config: &ModelConfig) -> Value {
    let Ok(Value::Object(mut parameters)) = serde_json::to_value(model_config) else {
        return json!({});
    };
    parameters.remove("model_name");

    let request_parameters = parameters
        .remove("request_params")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    parameters.retain(|_, value| is_langfuse_model_parameter(value));
    for (name, value) in request_parameters {
        if is_langfuse_model_parameter(&value) {
            parameters.insert(name, value);
        }
    }
    Value::Object(parameters)
}

fn is_langfuse_model_parameter(value: &Value) -> bool {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => true,
        Value::Array(values) => values.iter().all(Value::is_string),
        Value::Object(values) => values.values().all(Value::is_string),
        Value::Null => false,
    }
}

fn append_output(output: &mut Vec<Message>, message: Message) {
    let Some(previous) = output.last_mut() else {
        output.push(message);
        return;
    };
    if previous.id != message.id || previous.role != message.role {
        output.push(message);
        return;
    }

    for content in message.content {
        match (previous.content.last_mut(), content) {
            (Some(MessageContent::Text(previous)), MessageContent::Text(next))
                if previous
                    .annotations
                    .as_ref()
                    .and_then(|annotations| annotations.audience.as_ref())
                    == next
                        .annotations
                        .as_ref()
                        .and_then(|annotations| annotations.audience.as_ref()) =>
            {
                previous.text.push_str(&next.text);
            }
            (_, content) => previous.content.push(content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracing::test_support::{capturing_layer, wait_for_closed_generation};
    use goose_providers::base::{stream_from_single_message, MessageStream};
    use goose_providers::conversation::token_usage::Usage;
    use tracing_subscriber::prelude::*;

    struct CompleteProvider;

    #[async_trait::async_trait]
    impl Provider for CompleteProvider {
        fn get_name(&self) -> &str {
            "complete-test"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            Ok(stream_from_single_message(
                Message::assistant().with_text("complete response"),
                ProviderUsage::new(
                    "complete-model".to_string(),
                    Usage::new(Some(3), Some(2), Some(5)),
                ),
            ))
        }
    }

    #[tokio::test]
    async fn complete_provider_emits_generation_observation() {
        let (layer, events) = capturing_layer();
        let subscriber = tracing_subscriber::registry().with(layer);
        let _subscriber_guard = tracing::subscriber::set_default(subscriber);

        let (message, _) = crate::session_context::with_session_id(
            Some("complete-session".to_string()),
            complete_provider(
                &CompleteProvider,
                &ModelConfig::new("requested-model"),
                "system",
                &[Message::user().with_text("request")],
                &[],
            ),
        )
        .await
        .unwrap();

        assert!(message.metadata.observation_id.is_some());
        let captured = wait_for_closed_generation(&events).await;
        assert!(captured.iter().any(|(event_type, body)| {
            event_type == "generation-create" && body["model"] == "requested-model"
        }));
        assert!(captured.iter().any(|(event_type, body)| {
            event_type == "generation-update"
                && body["input"]["messages"][0]["content"][0]["text"] == "request"
        }));
        assert!(captured.iter().any(|(event_type, body)| {
            event_type == "generation-update" && body["model"] == "complete-model"
        }));
        assert!(captured.iter().any(|(event_type, body)| {
            event_type == "generation-update" && body["usageDetails"]["total"] == 5
        }));
        assert!(captured.iter().any(|(event_type, body)| {
            event_type == "generation-update"
                && body["output"][0]["content"][0]["text"] == "complete response"
        }));
    }
}
