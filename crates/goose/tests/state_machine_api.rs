use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use goose::agents::state_machine::{
    yielded_with, Emitter, Inference, InferenceInput, Operation, OperationResult, StateEffect,
    StateMachine, Step,
};
use goose::agents::AgentEvent;
use goose::config::GooseMode;
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use goose::providers::base::ProviderUsage;
use goose::session::session_manager::token_state_from_session_and_totals;
use goose::session::{Session, SessionManager, SessionType};
use goose_providers::conversation::token_usage::Usage;
use goose_providers::model::ModelConfig;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::Subscriber;
use tracing_futures::Instrument;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

struct PromptPart;

#[async_trait]
impl Operation for PromptPart {
    fn name(&self) -> &'static str {
        "prompt_part"
    }

    async fn prompt_parts(
        &self,
        _session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        Ok(vec![("test".to_string(), "custom context".to_string())])
    }
}

struct TestInference;

#[async_trait]
impl Operation for TestInference {
    fn name(&self) -> &'static str {
        "test_inference"
    }
}

#[async_trait]
impl Inference for TestInference {
    fn applies(&self, _conversation: &Conversation) -> bool {
        true
    }

    async fn infer(
        &self,
        _session: &Session,
        conversation: &Conversation,
        input: InferenceInput,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        assert_eq!(
            input.prompt_parts,
            [("test".to_string(), "custom context".to_string())]
        );
        let prompt = conversation
            .messages()
            .iter()
            .rev()
            .find(|message| message.role == rmcp::model::Role::User)
            .map(Message::as_concat_text)
            .unwrap();
        let message = emit
            .message(Message::assistant().with_text(format!("{prompt} answered")))
            .await;
        yielded_with([
            StateEffect::from(message),
            StateEffect::RecordUsage(ProviderUsage::new(
                "test-model".to_string(),
                Usage::new(Some(5), Some(7), Some(12)),
            )),
        ])
    }
}

#[derive(Clone, Default)]
struct TraceFields(Arc<Mutex<HashMap<String, String>>>);

impl<S> Layer<S> for TraceFields
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        if context
            .span(id)
            .is_some_and(|span| span.metadata().name() == "state_machine_test")
        {
            attributes.record(&mut FieldVisitor(self.0.clone()));
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, context: Context<'_, S>) {
        if context
            .span(id)
            .is_some_and(|span| span.metadata().name() == "state_machine_test")
        {
            values.record(&mut FieldVisitor(self.0.clone()));
        }
    }
}

struct FieldVisitor(Arc<Mutex<HashMap<String, String>>>);

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .lock()
            .unwrap()
            .insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .lock()
            .unwrap()
            .insert(field.name().to_string(), value.to_string());
    }
}

#[tokio::test]
async fn custom_pipeline_supports_step_apply_run_tracing_and_usage() -> Result<()> {
    let _env = goose_test_support::otel::clear_otel_env(&[(
        "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT",
        "true",
    )]);
    let temp_dir = tempfile::tempdir()?;
    let session_manager = SessionManager::new(temp_dir.path().to_path_buf());
    let session = session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            "state-machine-api".to_string(),
            SessionType::Hidden,
            GooseMode::Auto,
        )
        .await?;
    session_manager
        .update(&session.id)
        .model_config(ModelConfig::new("test-model").with_context_limit(Some(1_000)))
        .apply()
        .await?;
    session_manager
        .add_message(&session.id, &Message::user().with_text("first turn"))
        .await?;

    let cancel = CancellationToken::new();
    let (tx, mut rx) = mpsc::channel(16);
    let emit = Emitter::new(tx, cancel.clone());
    let machine = StateMachine::new(
        vec![
            Step::Operation(Arc::new(PromptPart)),
            Step::Inference(Arc::new(TestInference)),
        ],
        cancel,
    );

    let session = session_manager.get_session(&session.id, true).await?;
    let mut result = machine.step(&session, &emit).await?.unwrap();
    machine
        .apply(&session_manager, &session, &mut result, &emit)
        .await?;
    assert!(result.yield_to_client);
    let session = session_manager.get_session(&session.id, true).await?;
    let persisted = session
        .conversation
        .as_ref()
        .and_then(Conversation::last)
        .expect("persisted inference response");
    let emitted = match rx.recv().await {
        Some(AgentEvent::Message(message)) => message,
        other => panic!("expected emitted inference response, got {other:?}"),
    };
    assert_eq!(emitted.id, persisted.id);
    assert_eq!(persisted.as_concat_text(), "first turn answered");

    session_manager
        .add_message(&session.id, &Message::user().with_text("second turn"))
        .await?;
    let fields = TraceFields::default();
    let subscriber = tracing_subscriber::registry().with(fields.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    let span = tracing::info_span!(
        "state_machine_test",
        trace_input = tracing::field::Empty,
        trace_output = tracing::field::Empty,
        gen_ai.output.messages = tracing::field::Empty,
        gen_ai.usage.input_tokens = tracing::field::Empty,
        gen_ai.usage.output_tokens = tracing::field::Empty,
    );
    let session = machine
        .run(&session_manager, &session.id, &emit)
        .instrument(span)
        .await?;

    assert_eq!(
        session
            .conversation
            .as_ref()
            .and_then(Conversation::last)
            .map(Message::as_concat_text)
            .as_deref(),
        Some("second turn answered")
    );
    assert_eq!(session.usage.total_tokens, Some(12));
    let usage = session
        .conversation
        .as_ref()
        .and_then(Conversation::last)
        .and_then(|message| message.metadata.usage.as_deref())
        .unwrap();
    assert_eq!(usage.total_tokens, Some(12));
    let totals = session_manager
        .get_session_usage_totals(&session.id)
        .await?;
    let terminal_usage = token_state_from_session_and_totals(&session, &totals);
    assert_eq!(terminal_usage.total_tokens, 12);
    assert_eq!(terminal_usage.accumulated_total_tokens, 24);

    let fields = fields.0.lock().unwrap();
    assert_eq!(
        fields.get("trace_input").map(String::as_str),
        Some("second turn")
    );
    assert_eq!(
        fields.get("trace_output").map(String::as_str),
        Some("second turn answered")
    );
    assert_eq!(
        fields.get("gen_ai.usage.input_tokens").map(String::as_str),
        Some("5")
    );
    assert_eq!(
        fields.get("gen_ai.usage.output_tokens").map(String::as_str),
        Some("7")
    );
    let output: serde_json::Value = serde_json::from_str(&fields["gen_ai.output.messages"])?;
    assert_eq!(
        output,
        serde_json::json!([{
            "role": "assistant",
            "content": "second turn answered",
            "finish_reason": "stop",
        }])
    );

    Ok(())
}
