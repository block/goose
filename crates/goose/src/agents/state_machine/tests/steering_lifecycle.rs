use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::{CallToolRequestParams, Tool};
use tokio::sync::{mpsc, Mutex as AsyncMutex, Notify};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::pipeline::{self, test_pipeline, MessageKind::Agent};
use crate::agents::provider_stream_coordinator::ProviderStreamItem;
use crate::agents::state_machine::Emitter;
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider};

const SUMMARIZE_HISTORY: &str = "Please summarize the conversation history";
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

struct NativeTestProvider {
    streams: AsyncMutex<VecDeque<MessageStream>>,
    native_results: Mutex<VecDeque<Result<bool, ProviderError>>>,
    prompts: Mutex<Vec<Vec<Message>>>,
    stream_calls: AtomicUsize,
    native_calls: AtomicUsize,
    stream_called: Notify,
    native_called: Notify,
}

impl NativeTestProvider {
    fn new(
        streams: impl IntoIterator<Item = MessageStream>,
        native_results: impl IntoIterator<Item = Result<bool, ProviderError>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            streams: AsyncMutex::new(streams.into_iter().collect()),
            native_results: Mutex::new(native_results.into_iter().collect()),
            prompts: Mutex::new(Vec::new()),
            stream_calls: AtomicUsize::new(0),
            native_calls: AtomicUsize::new(0),
            stream_called: Notify::new(),
            native_called: Notify::new(),
        })
    }

    async fn wait_for_stream_calls(&self, expected: usize) {
        while self.stream_calls.load(Ordering::SeqCst) < expected {
            self.stream_called.notified().await;
        }
    }

    async fn wait_for_native_calls(&self, expected: usize) {
        while self.native_calls.load(Ordering::SeqCst) < expected {
            self.native_called.notified().await;
        }
    }

    fn prompts(&self) -> Vec<Vec<Message>> {
        self.prompts.lock().expect("prompts lock").clone()
    }
}

#[async_trait]
impl Provider for NativeTestProvider {
    fn get_name(&self) -> &str {
        "native-test"
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        self.prompts
            .lock()
            .expect("prompts lock")
            .push(messages.to_vec());
        self.stream_called.notify_one();
        self.streams
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| ProviderError::ExecutionError("unexpected provider prompt".to_string()))
    }

    async fn steer_natively(
        &self,
        _session_id: &str,
        _message: &Message,
    ) -> Result<bool, ProviderError> {
        self.native_calls.fetch_add(1, Ordering::SeqCst);
        self.native_called.notify_one();
        self.native_results
            .lock()
            .expect("native results lock")
            .pop_front()
            .expect("native steering result")
    }
}

fn controlled_stream() -> (mpsc::UnboundedSender<ProviderStreamItem>, MessageStream) {
    let (tx, rx) = mpsc::unbounded_channel();
    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    (tx, Box::pin(stream))
}

fn completed_stream(message: Message) -> MessageStream {
    Box::pin(stream::iter([Ok((Some(message), None))]))
}

fn reported_usage() -> ProviderUsage {
    ProviderUsage::new(
        "test-model".to_string(),
        Usage::new(Some(12), Some(3), Some(15)),
    )
}

async fn next_message(
    events: &mut mpsc::Receiver<AgentEvent>,
    predicate: impl Fn(&Message) -> bool,
) -> Message {
    loop {
        match events.recv().await.expect("agent event") {
            AgentEvent::Message(message) if predicate(&message) => return message,
            _ => {}
        }
    }
}

#[tokio::test]
async fn native_steering_cancels_unstarted_tool_request() -> Result<()> {
    let (stream_tx, provider_stream) = controlled_stream();
    let provider = NativeTestProvider::new([provider_stream], [Ok(true)]);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline.with_provider(provider.clone());
    pipeline
        .seed([Message::user().with_text("start cancellable work")])
        .await?;

    let cancel = CancellationToken::new();
    let machine = pipeline.machine(cancel.clone());
    let (event_tx, mut events) = mpsc::channel(32);
    let emit = Emitter::new(event_tx, cancel.clone());
    let run = machine.run(
        pipeline.session_manager.as_ref(),
        &pipeline.session_id,
        &emit,
    );
    let observe = async {
        provider.wait_for_stream_calls(1).await;
        let tool_request = Message::assistant().with_tool_request(
            "native-cancel-tool",
            Ok(CallToolRequestParams::new("not_executed")),
        );
        stream_tx
            .send(Ok((Some(tool_request), Some(reported_usage()))))
            .expect("provider stream receiver");
        next_message(&mut events, |message| {
            message
                .get_tool_request_ids()
                .contains("native-cancel-tool")
        })
        .await;

        pipeline
            .steer(
                Message::user()
                    .with_id("native-cancel-steer")
                    .with_text("change direction"),
            )
            .await;
        let emitted = next_message(&mut events, |message| {
            message.as_concat_text() == "change direction"
        })
        .await;
        let persisted = pipeline.session().await.expect("persisted session");
        let messages = persisted
            .conversation
            .as_ref()
            .expect("persisted conversation")
            .messages();
        let steer = messages
            .iter()
            .find(|message| message.as_concat_text() == "change direction")
            .expect("persisted native steer");
        assert_eq!(steer.id, emitted.id);
        assert_eq!(steer.id.as_deref(), Some("native-cancel-steer"));
        cancel.cancel();
    };

    let (session, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, observe) })
        .await
        .expect("native delivery should settle");
    let session = session?;
    let messages = session
        .conversation
        .as_ref()
        .expect("final conversation")
        .messages();
    let request = messages
        .iter()
        .position(|message| {
            message
                .get_tool_request_ids()
                .contains("native-cancel-tool")
        })
        .expect("persisted tool request");
    let steer = messages
        .iter()
        .position(|message| message.as_concat_text() == "change direction")
        .expect("persisted steer");
    let responses = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| {
            message
                .get_tool_response_ids()
                .contains("native-cancel-tool")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.as_concat_text() == "change direction")
            .count(),
        1
    );
    assert_eq!(responses.len(), 1);
    assert!(request < responses[0].0);
    assert!(responses[0].0 < steer);
    assert!(responses[0].1.content.iter().any(|content| {
        content
            .as_tool_response_text()
            .is_some_and(|text| text.contains("steering arrived before execution"))
    }));
    assert!(!pipeline.has_pending_steers().await);
    Ok(())
}

#[tokio::test]
async fn native_delivery_preserves_order_estimated_usage_and_prompt_boundary() -> Result<()> {
    let (stream_tx, provider_stream) = controlled_stream();
    let provider = NativeTestProvider::new([provider_stream], [Ok(true)]);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline.with_provider(provider.clone());
    pipeline
        .seed([Message::user().with_text("start native work")])
        .await?;

    let cancel = CancellationToken::new();
    let machine = pipeline.machine(cancel.clone());
    let (event_tx, mut events) = mpsc::channel(32);
    let emit = Emitter::new(event_tx, cancel);
    let run = machine.run(
        pipeline.session_manager.as_ref(),
        &pipeline.session_id,
        &emit,
    );
    let observe = async {
        provider.wait_for_stream_calls(1).await;
        stream_tx
            .send(Ok((
                Some(Message::assistant().with_text("before steer")),
                None,
            )))
            .expect("provider stream receiver");
        next_message(&mut events, |message| {
            message.as_concat_text() == "before steer"
        })
        .await;
        pipeline
            .steer(
                Message::user()
                    .with_id("native-order-steer")
                    .with_text("new direction"),
            )
            .await;
        next_message(&mut events, |message| {
            message.as_concat_text() == "new direction"
        })
        .await;
        drop(stream_tx);
    };

    let (session, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, observe) })
        .await
        .expect("native delivery should settle");
    let session = session?;
    let messages = session
        .conversation
        .as_ref()
        .expect("final conversation")
        .messages();
    let assistant = messages
        .iter()
        .position(|message| message.as_concat_text() == "before steer")
        .expect("persisted assistant prefix");
    let steer = messages
        .iter()
        .position(|message| message.as_concat_text() == "new direction")
        .expect("persisted steer");
    assert!(assistant < steer);
    assert!(messages[assistant].metadata.usage.is_some());
    assert_eq!(messages[steer].id.as_deref(), Some("native-order-steer"));
    assert!(messages[steer].metadata.steer);
    assert_eq!(
        messages[steer]
            .metadata
            .operation_note("llm", "native_steer_delivered")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(!messages.iter().any(|message| {
        message
            .as_concat_text()
            .contains("The model returned an empty response")
    }));
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    timeout(TEST_TIMEOUT, pipeline.resume())
        .await
        .expect("resume should not start another prompt")?;
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn usage_reported_after_native_delivery_updates_the_flushed_assistant() -> Result<()> {
    let (stream_tx, provider_stream) = controlled_stream();
    let provider = NativeTestProvider::new([provider_stream], [Ok(true)]);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline.with_provider(provider.clone());
    pipeline
        .seed([Message::user().with_text("start usage work")])
        .await?;

    let cancel = CancellationToken::new();
    let machine = pipeline.machine(cancel.clone());
    let (event_tx, mut events) = mpsc::channel(32);
    let emit = Emitter::new(event_tx, cancel);
    let run = machine.run(
        pipeline.session_manager.as_ref(),
        &pipeline.session_id,
        &emit,
    );
    let observe = async {
        provider.wait_for_stream_calls(1).await;
        stream_tx
            .send(Ok((
                Some(Message::assistant().with_text("usage prefix")),
                None,
            )))
            .expect("provider stream receiver");
        next_message(&mut events, |message| {
            message.as_concat_text() == "usage prefix"
        })
        .await;
        pipeline
            .steer(Message::user().with_text("usage steer"))
            .await;
        next_message(&mut events, |message| {
            message.as_concat_text() == "usage steer"
        })
        .await;
        stream_tx
            .send(Ok((None, Some(reported_usage()))))
            .expect("provider stream receiver");
        drop(stream_tx);
    };

    let (session, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, observe) })
        .await
        .expect("usage tail should settle");
    let session = session?;
    let assistant = session
        .conversation
        .as_ref()
        .expect("final conversation")
        .messages()
        .iter()
        .find(|message| message.as_concat_text() == "usage prefix")
        .expect("persisted assistant prefix");
    let usage = assistant.metadata.usage.as_deref().expect("message usage");
    assert_eq!(usage.input_tokens, Some(12));
    assert_eq!(usage.output_tokens, Some(3));
    Ok(())
}

#[tokio::test]
async fn native_steering_error_uses_one_next_prompt_fallback() -> Result<()> {
    let (first_stream_tx, first_stream) = controlled_stream();
    let second_stream = completed_stream(Message::assistant().with_text("fallback complete"));
    let provider = NativeTestProvider::new(
        [first_stream, second_stream],
        [Err(ProviderError::ExecutionError(
            "native delivery failed".to_string(),
        ))],
    );
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline.with_provider(provider.clone());

    let run = pipeline.run(["start fallback work"]);
    let steer = async {
        provider.wait_for_stream_calls(1).await;
        pipeline
            .steer(
                Message::user()
                    .with_id("fallback-steer")
                    .with_text("fallback direction"),
            )
            .await;
        provider.wait_for_native_calls(1).await;
        first_stream_tx
            .send(Ok((
                Some(Message::assistant().with_text("first prompt complete")),
                None,
            )))
            .expect("provider stream receiver");
        drop(first_stream_tx);
    };
    let (result, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, steer) })
        .await
        .expect("fallback should settle");
    let result = result?;

    result.assert_message(-1, Agent, "fallback complete");
    let steers = result
        .conversation()
        .messages()
        .iter()
        .filter(|message| message.as_concat_text() == "fallback direction")
        .collect::<Vec<_>>();
    assert_eq!(steers.len(), 1);
    assert_eq!(steers[0].id.as_deref(), Some("fallback-steer"));
    assert!(steers[0].metadata.steer);
    assert!(steers[0]
        .metadata
        .operation_note("llm", "native_steer_delivered")
        .is_none());
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 2);
    assert_eq!(provider.native_calls.load(Ordering::SeqCst), 1);
    assert!(provider.prompts()[1]
        .iter()
        .any(|message| message.as_concat_text().contains("fallback direction")));
    assert!(!pipeline.has_pending_steers().await);
    assert!(!result
        .conversation()
        .messages()
        .iter()
        .any(|message| { message.as_concat_text().contains("native delivery failed") }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steering_is_fifo_during_inference_and_survives_compaction() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let first_response = api.on("paint it").hold_reply("starting");
    api.on("then make it matte").reply("followed both changes");

    let run = pipeline.run(["paint it"]);
    let steer = async {
        first_response.entered().await;
        pipeline
            .steer(Message::user().with_text("first use blue"))
            .await;
        pipeline
            .steer(Message::user().with_text("then make it matte"))
            .await;
        first_response.release();
    };
    let (result, ()) = tokio::join!(run, steer);
    let result = result?;

    result.assert_message(-1, Agent, "followed both changes");
    let messages = result.conversation().messages();
    let first = messages
        .iter()
        .position(|message| message.as_concat_text() == "first use blue")
        .expect("first steer was persisted");
    let second = messages
        .iter()
        .position(|message| message.as_concat_text() == "then make it matte")
        .expect("second steer was persisted");
    assert!(first < second);
    assert!(messages[first].metadata.steer);
    assert!(messages[second].metadata.steer);
    assert!(!pipeline.has_pending_steers().await);
    let calls = api.calls();
    assert!(calls[1].input_contains("first use blue"));
    assert!(calls[1].input_contains("then make it matte"));

    let large_response = "x"
        .repeat((pipeline.context_limit() as f64 * pipeline::COMPACTION_THRESHOLD * 1.01) as usize);
    let held_response = api.on("continue near the limit").hold_reply(large_response);
    api.on(SUMMARIZE_HISTORY).reply("steered work summarized");
    api.on("Your context was compacted")
        .reply("continued after steering compaction");

    let run = pipeline.run(["continue near the limit"]);
    let steer = async {
        held_response.entered().await;
        pipeline
            .steer(Message::user().with_text("redirect before compaction"))
            .await;
        held_response.release();
    };
    let (result, ()) = tokio::join!(run, steer);
    let result = result?;

    result.assert_message(-1, Agent, "continued after steering compaction");
    assert_eq!(result.history_replacements(), 1);
    assert!(!pipeline.has_pending_steers().await);
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("compaction request");
    assert!(summarization.system_contains("redirect before compaction"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_preserves_queued_steering_for_resume() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let held_response = api
        .on("cancel current work")
        .hold_reply("current work finished");
    api.on("use the queued direction")
        .reply("queued direction applied");
    let cancel = CancellationToken::new();

    let run = pipeline.run_with_cancel("cancel current work", cancel.clone());
    let steer = async {
        held_response.entered().await;
        pipeline
            .steer(Message::user().with_text("use the queued direction"))
            .await;
        cancel.cancel();
        held_response.release();
    };
    let (cancelled, ()) = tokio::join!(run, steer);
    let cancelled = cancelled?;
    assert!(pipeline.has_pending_steers().await);
    assert!(!cancelled
        .conversation()
        .messages()
        .iter()
        .any(|message| message.as_concat_text() == "use the queued direction"));

    let resumed = pipeline.resume().await?;
    resumed.assert_message(-1, Agent, "queued direction applied");
    assert!(!pipeline.has_pending_steers().await);
    let steers = resumed
        .conversation()
        .messages()
        .iter()
        .filter(|message| message.as_concat_text() == "use the queued direction")
        .collect::<Vec<_>>();
    assert_eq!(steers.len(), 1);
    assert!(steers[0].metadata.steer);
    assert!(api
        .calls()
        .last()
        .expect("resumed inference request")
        .input_contains("use the queued direction"));

    Ok(())
}
