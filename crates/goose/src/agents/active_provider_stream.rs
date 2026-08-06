use std::pin::Pin;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::StreamExt;
use tokio::sync::futures::OwnedNotified;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use super::pending_steers::PendingSteers;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider};
use crate::utils::is_token_cancelled;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;

pub(super) type ProviderStreamItem =
    Result<(Option<Message>, Option<ProviderUsage>), ProviderError>;
type NativeSteerResult = (Message, Result<bool, ProviderError>);

pub(super) enum ActiveProviderStreamEvent {
    ProviderOutput(Box<ProviderStreamItem>),
    NativeSteerDelivered(Message),
}

enum PendingSteerDeliveryStrategy {
    NativeSteering,
    NextPrompt,
}

pub(super) struct ActiveProviderStream<'a> {
    stream: MessageStream,
    provider: Arc<dyn Provider>,
    pending_steers: &'a PendingSteers,
    session_id: &'a str,
    steer_notifier: Arc<Notify>,
    steer_notification_waiter: Pin<Box<OwnedNotified>>,
    // Notifications can coalesce, so scan until the queue is empty.
    queue_scan_needed: bool,
    pending_steer_delivery_strategy: PendingSteerDeliveryStrategy,
    pending_native_steer: Option<BoxFuture<'static, NativeSteerResult>>,
}

impl<'a> ActiveProviderStream<'a> {
    pub(super) async fn new(
        stream: MessageStream,
        provider: Arc<dyn Provider>,
        pending_steers: &'a PendingSteers,
        session_id: &'a str,
    ) -> Self {
        let steer_notifier = pending_steers.notifier(session_id).await;
        let steer_notification_waiter = Box::pin(Arc::clone(&steer_notifier).notified_owned());

        Self {
            stream,
            provider,
            pending_steers,
            session_id,
            steer_notifier,
            steer_notification_waiter,
            queue_scan_needed: true,
            pending_steer_delivery_strategy: PendingSteerDeliveryStrategy::NativeSteering,
            pending_native_steer: None,
        }
    }

    fn start_native_steer(&mut self, message: Message) {
        let provider = Arc::clone(&self.provider);
        let session_id = self.session_id.to_string();
        let result = Box::pin(async move {
            let result = provider.steer_natively(&session_id, &message).await;
            (message, result)
        });
        self.pending_native_steer = Some(result);
    }

    async fn finish_native_steer(
        &mut self,
        (message, result): NativeSteerResult,
    ) -> Option<ActiveProviderStreamEvent> {
        match result {
            Ok(true) => {
                return Some(ActiveProviderStreamEvent::NativeSteerDelivered(message));
            }
            Ok(false) => {}
            Err(error) => {
                warn!(
                    "Native steering failed; sending the message with the next provider prompt: {error}"
                );
            }
        }

        self.pending_steers
            .restore_front(self.session_id, message)
            .await;
        self.pending_steer_delivery_strategy = PendingSteerDeliveryStrategy::NextPrompt;
        None
    }

    async fn await_pending_native_steer(&mut self) -> Option<ActiveProviderStreamEvent> {
        let result = self
            .pending_native_steer
            .as_mut()
            .expect("pending native steer required")
            .as_mut()
            .await;
        self.pending_native_steer = None;
        self.finish_native_steer(result).await
    }

    pub(super) async fn next_event(
        &mut self,
        cancellation_token: &Option<CancellationToken>,
    ) -> Option<ActiveProviderStreamEvent> {
        loop {
            if is_token_cancelled(cancellation_token) {
                if self.pending_native_steer.is_some() {
                    return self.await_pending_native_steer().await;
                }
                return None;
            }

            let native_steering_enabled = matches!(
                self.pending_steer_delivery_strategy,
                PendingSteerDeliveryStrategy::NativeSteering
            );

            if native_steering_enabled
                && self.pending_native_steer.is_none()
                && self.queue_scan_needed
            {
                let Some(message) = self.pending_steers.pop_front(self.session_id).await else {
                    self.queue_scan_needed = false;
                    continue;
                };

                if is_token_cancelled(cancellation_token) {
                    self.pending_steers
                        .restore_front(self.session_id, message)
                        .await;
                    return None;
                }

                self.start_native_steer(message);
                continue;
            }

            tokio::select! {
                biased;

                _ = async {
                    match cancellation_token {
                        Some(token) => token.cancelled().await,
                        None => futures::future::pending().await,
                    }
                } => {}

                _ = self.steer_notification_waiter.as_mut(), if native_steering_enabled && self.pending_native_steer.is_none() => {
                    self.steer_notification_waiter
                        .as_mut()
                        .set(Arc::clone(&self.steer_notifier).notified_owned());
                    self.queue_scan_needed = true;
                }

                next = self.stream.next() => {
                    match next {
                        Some(next) => {
                            return Some(ActiveProviderStreamEvent::ProviderOutput(Box::new(next)));
                        }
                        None => {
                            if self.pending_native_steer.is_some() {
                                return self.await_pending_native_steer().await;
                            }
                            return None;
                        }
                    }
                }

                result = async {
                    self.pending_native_steer
                        .as_mut()
                        .expect("select guard requires a pending native steer")
                        .as_mut()
                        .await
                }, if self.pending_native_steer.is_some() => {
                    self.pending_native_steer = None;
                    if let Some(event) = self.finish_native_steer(result).await {
                        return Some(event);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Poll;
    use std::time::Duration;

    use goose_providers::model::ModelConfig;
    use rmcp::model::Tool;
    use tokio::time::timeout;

    use super::*;

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);

    struct NativeSteeringProvider {
        steer_calls: AtomicUsize,
        behavior: NativeSteerBehavior,
    }

    enum NativeSteerBehavior {
        Immediate,
        Blocked {
            started: Arc<Notify>,
            release: Arc<Notify>,
        },
    }

    #[async_trait::async_trait]
    impl Provider for NativeSteeringProvider {
        fn get_name(&self) -> &str {
            "native-steering-test"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            unreachable!("the coordinator receives its provider stream directly")
        }

        async fn steer_natively(
            &self,
            _session_id: &str,
            _message: &Message,
        ) -> Result<bool, ProviderError> {
            self.steer_calls.fetch_add(1, Ordering::SeqCst);
            match &self.behavior {
                NativeSteerBehavior::Immediate => {}
                NativeSteerBehavior::Blocked { started, release } => {
                    started.notify_one();
                    release.notified().await;
                }
            }
            Ok(true)
        }
    }

    #[tokio::test]
    async fn steer_notification_wakes_active_provider_stream() {
        let pending_steers = PendingSteers::default();
        let provider = Arc::new(NativeSteeringProvider {
            steer_calls: AtomicUsize::new(0),
            behavior: NativeSteerBehavior::Immediate,
        });
        let stream_polled = Arc::new(Notify::new());
        let stream_polled_by_stream = Arc::clone(&stream_polled);
        let stream: MessageStream = Box::pin(futures::stream::poll_fn(move |_| {
            stream_polled_by_stream.notify_one();
            Poll::Pending
        }));
        let session_id = "notification-wakeup";
        let mut active_stream =
            ActiveProviderStream::new(stream, provider.clone(), &pending_steers, session_id).await;

        let event = timeout(TEST_TIMEOUT, async {
            let (event, ()) = tokio::join!(active_stream.next_event(&None), async {
                stream_polled.notified().await;
                pending_steers
                    .enqueue(session_id, Message::user().with_text("new steer"))
                    .await;
            });
            event
        })
        .await
        .expect("steer notification should wake the active provider stream");

        let Some(ActiveProviderStreamEvent::NativeSteerDelivered(message)) = event else {
            panic!("expected confirmed native steer delivery");
        };
        assert_eq!(message.as_concat_text(), "new steer");
        assert_eq!(provider.steer_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn provider_output_is_polled_while_native_steer_is_in_flight() {
        let pending_steers = PendingSteers::default();
        let steer_started = Arc::new(Notify::new());
        let steer_release = Arc::new(Notify::new());
        let provider = Arc::new(NativeSteeringProvider {
            steer_calls: AtomicUsize::new(0),
            behavior: NativeSteerBehavior::Blocked {
                started: Arc::clone(&steer_started),
                release: Arc::clone(&steer_release),
            },
        });
        let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
        let stream: MessageStream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(
            output_rx,
        ));
        let session_id = "output-during-native-steer";
        let mut active_stream =
            ActiveProviderStream::new(stream, provider.clone(), &pending_steers, session_id).await;

        let first_event = timeout(TEST_TIMEOUT, async {
            let (event, ()) = tokio::join!(active_stream.next_event(&None), async {
                pending_steers
                    .enqueue(session_id, Message::user().with_text("new steer"))
                    .await;
                steer_started.notified().await;
                output_tx
                    .send(Ok((
                        Some(Message::assistant().with_text("before steer")),
                        None,
                    )))
                    .expect("provider stream should remain open");
            });
            event
        })
        .await
        .expect("provider output should not wait for the steering response");

        let Some(ActiveProviderStreamEvent::ProviderOutput(output)) = first_event else {
            panic!("expected provider output before the steering response");
        };
        let Ok((Some(message), None)) = *output else {
            panic!("expected provider output before the steering response");
        };
        assert_eq!(message.as_concat_text(), "before steer");

        let second_event = timeout(TEST_TIMEOUT, async {
            let (event, ()) = tokio::join!(active_stream.next_event(&None), async {
                steer_release.notify_one();
            });
            event
        })
        .await
        .expect("steering response should be delivered after provider output");

        let Some(ActiveProviderStreamEvent::NativeSteerDelivered(message)) = second_event else {
            panic!("expected confirmed native steer delivery");
        };
        assert_eq!(message.as_concat_text(), "new steer");
        assert_eq!(provider.steer_calls.load(Ordering::SeqCst), 1);
    }
}
