use std::sync::Arc;

use futures::{future::BoxFuture, StreamExt};
use goose_providers::errors::ProviderError;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use super::steering::SteeringQueue;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider, ProviderUsage};

pub(crate) type ProviderStreamItem =
    Result<(Option<Message>, Option<ProviderUsage>), ProviderError>;

type NativeSteerFuture = BoxFuture<'static, (u64, Result<bool, ProviderError>)>;

#[expect(
    clippy::large_enum_variant,
    reason = "boxing provider output would allocate for every streamed chunk"
)]
pub(crate) enum ProviderStreamEvent {
    ProviderOutput(ProviderStreamItem),
    NativeSteerDelivered(Message),
}

pub(crate) struct ProviderStreamCoordinator {
    stream: MessageStream,
    provider: Arc<dyn Provider>,
    steering_queue: Arc<SteeringQueue>,
    session_id: String,
    pending_native_steer: Option<NativeSteerFuture>,
    fallback_to_next_prompt: bool,
}

impl ProviderStreamCoordinator {
    pub(crate) fn new(
        stream: MessageStream,
        provider: Arc<dyn Provider>,
        steering_queue: Arc<SteeringQueue>,
        session_id: &str,
    ) -> Self {
        Self {
            stream,
            provider,
            steering_queue,
            session_id: session_id.to_string(),
            pending_native_steer: None,
            fallback_to_next_prompt: false,
        }
    }

    pub(crate) async fn next_event(
        &mut self,
        cancel: &CancellationToken,
    ) -> Option<ProviderStreamEvent> {
        loop {
            if self.pending_native_steer.is_none() {
                if cancel.is_cancelled() {
                    return None;
                }

                if !self.fallback_to_next_prompt {
                    if let Some((entry_id, message)) = self.steering_queue.peek_next_ready().await {
                        self.start_native_steer(entry_id, message);
                    }
                }
            }

            tokio::select! {
                biased;

                result = async {
                    self.pending_native_steer
                        .as_mut()
                        .expect("native steer future must exist")
                        .await
                }, if self.pending_native_steer.is_some() => {
                    self.pending_native_steer = None;
                    let (entry_id, result) = result;
                    match result {
                        Ok(true) => {
                            let message = self
                                .steering_queue
                                .remove_next_ready(entry_id)
                                .await
                                .expect("delivered steer must still be the next ready entry");
                            return Some(ProviderStreamEvent::NativeSteerDelivered(message));
                        }
                        Ok(false) => self.fallback_to_next_prompt = true,
                        Err(error) => {
                            warn!(%error, "Native steering failed; retaining steer for the next prompt");
                            self.fallback_to_next_prompt = true;
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    self.pending_native_steer = None;
                    return None;
                }
                output = self.stream.next() => {
                    match output {
                        Some(Ok(output)) => {
                            return Some(ProviderStreamEvent::ProviderOutput(Ok(output)));
                        }
                        Some(Err(error)) => {
                            self.pending_native_steer = None;
                            return Some(ProviderStreamEvent::ProviderOutput(Err(error)));
                        }
                        None => {
                            self.pending_native_steer = None;
                            return None;
                        }
                    }
                }
                _ = self.steering_queue.wait_for_next_ready(),
                    if self.pending_native_steer.is_none() && !self.fallback_to_next_prompt => {}
            }
        }
    }

    fn start_native_steer(&mut self, entry_id: u64, message: Message) {
        let provider = Arc::clone(&self.provider);
        let session_id = self.session_id.clone();
        self.pending_native_steer = Some(Box::pin(async move {
            let result = provider.steer_natively(&session_id, &message).await;
            (entry_id, result)
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use futures::{stream, StreamExt};
    use goose_providers::model::ModelConfig;
    use rmcp::model::Tool;
    use tokio::sync::{Mutex as AsyncMutex, Notify};
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    use super::{
        MessageStream, Provider, ProviderError, ProviderStreamCoordinator, ProviderStreamEvent,
    };
    use crate::agents::steering::SteeringQueue;
    use crate::conversation::message::Message;

    const TEST_TIMEOUT: Duration = Duration::from_secs(2);

    enum NativeSteerBehavior {
        Immediate(Result<bool, ProviderError>),
        Blocked {
            started: Arc<Notify>,
            release: Arc<Notify>,
        },
    }

    struct TestProvider {
        behaviors: AsyncMutex<VecDeque<NativeSteerBehavior>>,
        calls: AtomicUsize,
        messages: Mutex<Vec<String>>,
    }

    impl TestProvider {
        fn new(behaviors: impl IntoIterator<Item = NativeSteerBehavior>) -> Arc<Self> {
            Arc::new(Self {
                behaviors: AsyncMutex::new(behaviors.into_iter().collect()),
                calls: AtomicUsize::new(0),
                messages: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl Provider for TestProvider {
        fn get_name(&self) -> &str {
            "test"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            unreachable!("coordinator tests provide the stream directly")
        }

        async fn steer_natively(
            &self,
            _session_id: &str,
            message: &Message,
        ) -> Result<bool, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.messages
                .lock()
                .expect("message lock")
                .push(message.as_concat_text());

            match self
                .behaviors
                .lock()
                .await
                .pop_front()
                .expect("test behavior")
            {
                NativeSteerBehavior::Immediate(result) => result,
                NativeSteerBehavior::Blocked { started, release } => {
                    started.notify_one();
                    release.notified().await;
                    Ok(true)
                }
            }
        }
    }

    async fn enqueue_ready(queue: &SteeringQueue, text: &str) -> u64 {
        let entry_id = queue.enqueue(Message::user().with_text(text)).await;
        assert!(queue.mark_hook_complete(entry_id).await);
        entry_id
    }

    fn pending_stream() -> MessageStream {
        Box::pin(stream::pending())
    }

    fn output_stream(texts: &[&str]) -> MessageStream {
        let outputs = texts
            .iter()
            .map(|text| Ok((Some(Message::assistant().with_text(*text)), None)))
            .collect::<Vec<_>>();
        Box::pin(stream::iter(outputs).chain(stream::pending()))
    }

    fn delivered_message(event: Option<ProviderStreamEvent>) -> Message {
        match event {
            Some(ProviderStreamEvent::NativeSteerDelivered(message)) => message,
            _ => panic!("expected delivered steer"),
        }
    }

    fn provider_output(event: Option<ProviderStreamEvent>) -> Message {
        match event {
            Some(ProviderStreamEvent::ProviderOutput(Ok((Some(message), None)))) => message,
            _ => panic!("expected provider output"),
        }
    }

    #[tokio::test]
    async fn ready_steer_wakes_the_stream_and_is_committed_after_delivery() {
        let queue = Arc::new(SteeringQueue::default());
        let provider = TestProvider::new([NativeSteerBehavior::Immediate(Ok(true))]);
        let mut stream = ProviderStreamCoordinator::new(
            pending_stream(),
            provider.clone(),
            Arc::clone(&queue),
            "session",
        );

        let cancel = CancellationToken::new();
        let next_event = stream.next_event(&cancel);
        let enqueue = async {
            tokio::task::yield_now().await;
            enqueue_ready(&queue, "steer").await;
        };
        let (event, _) = timeout(TEST_TIMEOUT, async { tokio::join!(next_event, enqueue) })
            .await
            .expect("steer should wake the stream");

        let message = delivered_message(event);
        assert_eq!(message.as_concat_text(), "steer");
        assert!(message.metadata.steer);
        assert!(!queue.has_pending().await);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn delivered_steers_preserve_fifo_order() {
        let queue = Arc::new(SteeringQueue::default());
        enqueue_ready(&queue, "first").await;
        enqueue_ready(&queue, "second").await;
        let provider = TestProvider::new([
            NativeSteerBehavior::Immediate(Ok(true)),
            NativeSteerBehavior::Immediate(Ok(true)),
        ]);
        let mut stream = ProviderStreamCoordinator::new(
            pending_stream(),
            provider.clone(),
            Arc::clone(&queue),
            "session",
        );

        let first = delivered_message(stream.next_event(&CancellationToken::new()).await);
        let second = delivered_message(stream.next_event(&CancellationToken::new()).await);

        assert_eq!(first.as_concat_text(), "first");
        assert_eq!(second.as_concat_text(), "second");
        assert_eq!(
            *provider.messages.lock().expect("message lock"),
            ["first", "second"]
        );
    }

    #[tokio::test]
    async fn provider_output_continues_while_native_steering_is_pending() {
        let queue = Arc::new(SteeringQueue::default());
        enqueue_ready(&queue, "steer").await;
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::clone(&started),
            release: Arc::clone(&release),
        }]);
        let mut stream = ProviderStreamCoordinator::new(
            output_stream(&["one", "two"]),
            provider,
            Arc::clone(&queue),
            "session",
        );

        assert_eq!(
            provider_output(stream.next_event(&CancellationToken::new()).await).as_concat_text(),
            "one"
        );
        timeout(TEST_TIMEOUT, started.notified())
            .await
            .expect("native steering should start");
        assert_eq!(
            provider_output(stream.next_event(&CancellationToken::new()).await).as_concat_text(),
            "two"
        );

        release.notify_one();
        let message = timeout(TEST_TIMEOUT, stream.next_event(&CancellationToken::new()))
            .await
            .expect("native steering should finish");
        assert_eq!(delivered_message(message).as_concat_text(), "steer");
        assert!(!queue.has_pending().await);
    }

    #[tokio::test]
    async fn unsuccessful_native_steering_falls_back_without_retrying() {
        for result in [
            Ok(false),
            Err(ProviderError::ExecutionError("failed".to_string())),
        ] {
            let queue = Arc::new(SteeringQueue::default());
            enqueue_ready(&queue, "first").await;
            enqueue_ready(&queue, "second").await;
            let provider = TestProvider::new([NativeSteerBehavior::Immediate(result)]);
            let provider_stream: MessageStream = Box::pin(stream::iter([Ok((
                Some(Message::assistant().with_text("output")),
                None,
            ))]));
            let mut stream = ProviderStreamCoordinator::new(
                provider_stream,
                provider.clone(),
                Arc::clone(&queue),
                "session",
            );

            assert_eq!(
                provider_output(stream.next_event(&CancellationToken::new()).await)
                    .as_concat_text(),
                "output"
            );
            assert!(stream.next_event(&CancellationToken::new()).await.is_none());
            assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                queue
                    .drain_available()
                    .await
                    .iter()
                    .map(Message::as_concat_text)
                    .collect::<Vec<_>>(),
                ["first", "second"]
            );
        }
    }

    #[tokio::test]
    async fn cancellation_drops_pending_native_steering_and_retains_the_queue_entry() {
        let queue = Arc::new(SteeringQueue::default());
        let entry_id = enqueue_ready(&queue, "steer").await;
        let started = Arc::new(Notify::new());
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::clone(&started),
            release: Arc::new(Notify::new()),
        }]);
        let mut stream = ProviderStreamCoordinator::new(
            pending_stream(),
            provider,
            Arc::clone(&queue),
            "session",
        );
        let cancel = CancellationToken::new();

        let event = timeout(TEST_TIMEOUT, async {
            tokio::join!(stream.next_event(&cancel), async {
                started.notified().await;
                cancel.cancel();
            })
            .0
        })
        .await
        .expect("cancellation should stop the stream");

        assert!(event.is_none());
        assert_eq!(queue.peek_next_ready().await.unwrap().0, entry_id);
    }

    #[tokio::test]
    async fn terminal_provider_stream_drops_pending_steering_and_retains_the_queue_entry() {
        let queue = Arc::new(SteeringQueue::default());
        let entry_id = enqueue_ready(&queue, "steer").await;
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }]);
        let provider_stream: MessageStream = Box::pin(stream::empty());
        let mut stream = ProviderStreamCoordinator::new(
            provider_stream,
            provider,
            Arc::clone(&queue),
            "session",
        );

        assert!(stream.next_event(&CancellationToken::new()).await.is_none());
        assert_eq!(queue.peek_next_ready().await.unwrap().0, entry_id);
    }

    #[tokio::test]
    async fn provider_stream_error_drops_pending_steering_and_retains_the_queue_entry() {
        let queue = Arc::new(SteeringQueue::default());
        let entry_id = enqueue_ready(&queue, "steer").await;
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }]);
        let provider_stream: MessageStream = Box::pin(stream::iter([Err(
            ProviderError::ExecutionError("stream failed".to_string()),
        )]));
        let mut stream = ProviderStreamCoordinator::new(
            provider_stream,
            provider,
            Arc::clone(&queue),
            "session",
        );

        match stream.next_event(&CancellationToken::new()).await {
            Some(ProviderStreamEvent::ProviderOutput(Err(error))) => {
                assert_eq!(error.to_string(), "Execution error: stream failed");
            }
            _ => panic!("expected provider stream error"),
        }
        assert_eq!(queue.peek_next_ready().await.unwrap().0, entry_id);
    }

    #[tokio::test]
    async fn completed_native_steering_wins_over_simultaneous_cancellation() {
        let queue = Arc::new(SteeringQueue::default());
        enqueue_ready(&queue, "steer").await;
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::clone(&started),
            release: Arc::clone(&release),
        }]);
        let mut stream = ProviderStreamCoordinator::new(
            pending_stream(),
            provider,
            Arc::clone(&queue),
            "session",
        );
        let cancel = CancellationToken::new();

        let event = timeout(TEST_TIMEOUT, async {
            tokio::join!(stream.next_event(&cancel), async {
                started.notified().await;
                release.notify_one();
                cancel.cancel();
            })
            .0
        })
        .await
        .expect("completed steering should settle");

        assert_eq!(delivered_message(event).as_concat_text(), "steer");
        assert!(!queue.has_pending().await);
    }

    #[tokio::test]
    async fn dropping_the_coordinator_retains_a_pending_steer() {
        let queue = Arc::new(SteeringQueue::default());
        let entry_id = enqueue_ready(&queue, "steer").await;
        let provider = TestProvider::new([NativeSteerBehavior::Blocked {
            started: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }]);
        let mut stream = ProviderStreamCoordinator::new(
            output_stream(&["output"]),
            provider,
            Arc::clone(&queue),
            "session",
        );

        provider_output(stream.next_event(&CancellationToken::new()).await);
        drop(stream);

        assert_eq!(queue.peek_next_ready().await.unwrap().0, entry_id);
    }
}
