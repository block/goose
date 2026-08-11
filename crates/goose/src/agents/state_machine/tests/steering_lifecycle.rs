use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use futures::stream;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use rmcp::model::CallToolRequestParams;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::hooks_lifecycle::{HookTestEnv, LOG_AND_ALLOW_SCRIPT};
use super::pipeline::{self, test_pipeline, MessageKind::Agent, MessageKind::Error};
use crate::agents::state_machine::Emitter;
use crate::agents::steering::was_native_steer_delivered;
use crate::agents::test_support::{controlled_stream, NativeSteeringTestProvider};
use crate::agents::types::{RetryConfig, SuccessCheck};
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::providers::base::MessageStream;
use crate::recipe::Recipe;

const SUMMARIZE_HISTORY: &str = "Please summarize the conversation history";
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

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
    let provider = NativeSteeringTestProvider::new([provider_stream], [Ok(true)]);
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
        cancel.cancel();
        emitted.id
    };

    let (session, emitted_steer_id) = timeout(TEST_TIMEOUT, async { tokio::join!(run, observe) })
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
    assert_eq!(messages[steer].id, emitted_steer_id);
    assert_eq!(messages[steer].id.as_deref(), Some("native-cancel-steer"));
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
    let provider = NativeSteeringTestProvider::new([provider_stream], [Ok(true)]);
    let stop_hook = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline
        .with_provider(provider.clone())
        .with_hook_manager(stop_hook.hook_manager());
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
    let turn_context = messages
        .iter()
        .position(Message::is_turn_context)
        .expect("persisted turn context");
    let assistant = messages
        .iter()
        .position(|message| message.as_concat_text() == "before steer")
        .expect("persisted assistant prefix");
    let steer = messages
        .iter()
        .position(|message| message.as_concat_text() == "new direction")
        .expect("persisted steer");
    assert!(turn_context < assistant);
    assert!(assistant < steer);
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.is_turn_context())
            .count(),
        1
    );
    assert!(messages[assistant].metadata.usage.is_some());
    assert_eq!(messages[steer].id.as_deref(), Some("native-order-steer"));
    assert!(messages[steer].metadata.steer);
    assert!(was_native_steer_delivered(&messages[steer]));
    assert!(!messages.iter().any(|message| {
        message
            .as_concat_text()
            .contains("The model returned an empty response")
    }));
    assert_eq!(stop_hook.invocations(), 1);
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    timeout(TEST_TIMEOUT, pipeline.resume())
        .await
        .expect("resume should not start another prompt")?;
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn consecutive_native_steers_run_stop_hook_once() -> Result<()> {
    let (stream_tx, provider_stream) = controlled_stream();
    let provider = NativeSteeringTestProvider::new([provider_stream], [Ok(true), Ok(true)]);
    let stop_hook = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline
        .with_provider(provider.clone())
        .with_hook_manager(stop_hook.hook_manager());
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
    let steer = async {
        provider.wait_for_stream_calls(1).await;
        stream_tx
            .send(Ok((
                Some(Message::assistant().with_text("before steers")),
                None,
            )))
            .expect("provider stream receiver");
        next_message(&mut events, |message| {
            message.as_concat_text() == "before steers"
        })
        .await;

        pipeline
            .steer(Message::user().with_text("first direction"))
            .await;
        next_message(&mut events, |message| {
            message.as_concat_text() == "first direction"
        })
        .await;
        pipeline
            .steer(Message::user().with_text("second direction"))
            .await;
        next_message(&mut events, |message| {
            message.as_concat_text() == "second direction"
        })
        .await;
        drop(stream_tx);
    };

    let (session, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, steer) })
        .await
        .expect("native delivery should settle");
    let session = session?;
    let messages = session
        .conversation
        .as_ref()
        .expect("final conversation")
        .messages();
    assert_eq!(
        messages
            .iter()
            .filter(|message| was_native_steer_delivered(message))
            .count(),
        2
    );
    assert_eq!(stop_hook.invocations(), 1);
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn recipe_retry_restarts_from_original_kickoff_after_native_steer() -> Result<()> {
    let (first_stream_tx, first_stream) = controlled_stream();
    let second_stream = completed_stream(Message::assistant().with_text("retried response"));
    let provider = NativeSteeringTestProvider::new([first_stream, second_stream], [Ok(true)]);
    let (pipeline, _) = test_pipeline().await?;
    let pipeline = pipeline.with_provider(provider.clone());
    pipeline
        .set_recipe(
            Recipe::builder()
                .title("native steering retry")
                .description("native steering retry")
                .prompt("start retry work")
                .retry(RetryConfig {
                    max_retries: 1,
                    checks: vec![SuccessCheck::Shell {
                        command: "exit 1".to_string(),
                    }],
                    on_failure: None,
                    timeout_seconds: None,
                    on_failure_timeout_seconds: None,
                })
                .build()
                .expect("valid recipe"),
        )
        .await?;

    let run = pipeline.run(["start retry work"]);
    let steer = async {
        provider.wait_for_stream_calls(1).await;
        pipeline
            .steer(Message::user().with_text("change retry direction"))
            .await;
        provider.wait_for_native_calls(1).await;
        first_stream_tx
            .send(Ok((
                Some(Message::assistant().with_text("response after steer")),
                None,
            )))
            .expect("provider stream receiver");
        drop(first_stream_tx);
    };

    let (result, ()) = timeout(TEST_TIMEOUT, async { tokio::join!(run, steer) })
        .await
        .expect("recipe retry should settle");
    let result = result?;
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 2);
    let prompts = provider.prompts();
    assert!(prompts[1]
        .iter()
        .any(|message| message.as_concat_text().contains("start retry work")));
    assert!(!prompts[1].iter().any(|message| message.metadata.steer));
    result.assert_message(-1, Error, "Maximum retry attempts (1) exceeded");
    Ok(())
}

#[tokio::test]
async fn usage_reported_after_native_delivery_updates_the_assistant_before_steer() -> Result<()> {
    let (stream_tx, provider_stream) = controlled_stream();
    let provider = NativeSteeringTestProvider::new([provider_stream], [Ok(true)]);
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
async fn native_steering_error_after_empty_completion_falls_back_once() -> Result<()> {
    let (first_stream_tx, first_stream) = controlled_stream();
    let second_stream = completed_stream(Message::assistant().with_text("fallback complete"));
    let provider = NativeSteeringTestProvider::new(
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
    assert!(!was_native_steer_delivered(steers[0]));
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
