use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::futures::OwnedNotified;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
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

pub(super) enum ActiveProviderStreamEvent {
    ProviderOutput(Box<ProviderStreamItem>),
    NativeSteerDelivered(Message),
}

enum PendingSteerDeliveryStrategy {
    NativeSteering,
    NextPrompt,
}

struct PendingNativeSteer {
    message: Message,
    task: JoinHandle<Result<bool, ProviderError>>,
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
    pending_native_steer: Option<PendingNativeSteer>,
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
        let steer_message = message.clone();
        let task =
            tokio::spawn(async move { provider.steer_natively(&session_id, &steer_message).await });

        self.pending_native_steer = Some(PendingNativeSteer { message, task });
    }

    async fn finish_native_steer(
        &mut self,
        result: Result<Result<bool, ProviderError>, tokio::task::JoinError>,
    ) -> Option<ActiveProviderStreamEvent> {
        let pending = self
            .pending_native_steer
            .take()
            .expect("pending native steer required");

        match result {
            Ok(Ok(true)) => {
                return Some(ActiveProviderStreamEvent::NativeSteerDelivered(
                    pending.message,
                ));
            }
            Ok(Ok(false)) => {}
            Ok(Err(error)) => {
                warn!(
                    "Native steering failed; sending the message with the next provider prompt: {error}"
                );
            }
            Err(error) => {
                warn!(
                    "Native steering task failed; sending the message with the next provider prompt: {error}"
                );
            }
        }

        self.pending_steers
            .restore_front(self.session_id, pending.message)
            .await;
        self.pending_steer_delivery_strategy = PendingSteerDeliveryStrategy::NextPrompt;
        None
    }

    async fn abort_native_steer(&mut self) {
        let Some(pending) = self.pending_native_steer.take() else {
            return;
        };

        pending.task.abort();
        self.pending_steers
            .restore_front(self.session_id, pending.message)
            .await;
    }

    pub(super) async fn next_event(
        &mut self,
        cancellation_token: &Option<CancellationToken>,
    ) -> Option<ActiveProviderStreamEvent> {
        loop {
            if self.pending_native_steer.is_none() && is_token_cancelled(cancellation_token) {
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

                result = async {
                    (&mut self
                        .pending_native_steer
                        .as_mut()
                        .expect("select guard requires a pending native steer")
                        .task)
                        .await
                }, if self.pending_native_steer.is_some() => {
                    if let Some(event) = self.finish_native_steer(result).await {
                        return Some(event);
                    }
                }

                _ = async {
                    match cancellation_token {
                        Some(token) => token.cancelled().await,
                        None => futures::future::pending().await,
                    }
                } => {
                    self.abort_native_steer().await;
                    return None;
                },

                _ = self.steer_notification_waiter.as_mut(), if native_steering_enabled && self.pending_native_steer.is_none() => {
                    self.steer_notification_waiter
                        .as_mut()
                        .set(Arc::clone(&self.steer_notifier).notified_owned());
                    self.queue_scan_needed = true;
                }

                next = self.stream.next() => {
                    match next {
                        Some(next @ Err(_)) => {
                            self.abort_native_steer().await;
                            return Some(ActiveProviderStreamEvent::ProviderOutput(Box::new(next)));
                        }
                        Some(next) => {
                            return Some(ActiveProviderStreamEvent::ProviderOutput(Box::new(next)));
                        }
                        None => {
                            self.abort_native_steer().await;
                            return None;
                        }
                    }
                }
            }
        }
    }
}

impl Drop for ActiveProviderStream<'_> {
    fn drop(&mut self) {
        if let Some(pending) = self.pending_native_steer.take() {
            pending.task.abort();
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
        Rejected,
        Error,
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
                NativeSteerBehavior::Immediate => Ok(true),
                NativeSteerBehavior::Rejected => Ok(false),
                NativeSteerBehavior::Error => {
                    Err(ProviderError::ExecutionError("steering failed".into()))
                }
                NativeSteerBehavior::Blocked { started, release } => {
                    started.notify_one();
                    release.notified().await;
                    Ok(true)
                }
            }
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
        assert!(!pending_steers.has_pending(session_id).await);
    }

    #[tokio::test]
    async fn cancellation_does_not_wait_for_native_steering() {
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
        let stream: MessageStream = Box::pin(futures::stream::pending());
        let session_id = "cancel-native-steer";
        let cancellation_token = CancellationToken::new();
        let cancellation = Some(cancellation_token.clone());
        let mut active_stream =
            ActiveProviderStream::new(stream, provider.clone(), &pending_steers, session_id).await;

        let event = timeout(TEST_TIMEOUT, async {
            let (event, ()) = tokio::join!(active_stream.next_event(&cancellation), async {
                pending_steers
                    .enqueue(session_id, Message::user().with_text("new steer"))
                    .await;
                steer_started.notified().await;
                cancellation_token.cancel();
            });
            event
        })
        .await
        .expect("cancellation should not wait for the steering response");

        assert!(event.is_none());
        assert_eq!(provider.steer_calls.load(Ordering::SeqCst), 1);
        let restored = pending_steers.drain(session_id).await;
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].as_concat_text(), "new steer");
    }

    #[tokio::test]
    async fn provider_output_continues_while_native_steering_is_pending() {
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
        let stream: MessageStream = Box::pin(
            futures::stream::iter((0..65).map(|index| {
                Ok((
                    Some(Message::assistant().with_text(index.to_string())),
                    None,
                ))
            }))
            .chain(futures::stream::pending()),
        );
        let session_id = "drain-during-native-steer";
        pending_steers
            .enqueue(session_id, Message::user().with_text("new steer"))
            .await;
        let mut active_stream =
            ActiveProviderStream::new(stream, provider, &pending_steers, session_id).await;

        for _ in 0..65 {
            assert!(matches!(
                active_stream.next_event(&None).await,
                Some(ActiveProviderStreamEvent::ProviderOutput(_))
            ));
        }

        steer_started.notified().await;
        steer_release.notify_one();
        let event = timeout(TEST_TIMEOUT, active_stream.next_event(&None))
            .await
            .expect("native steer should complete after being released");
        assert!(matches!(
            event,
            Some(ActiveProviderStreamEvent::NativeSteerDelivered(_))
        ));
    }

    #[tokio::test]
    async fn terminal_provider_output_restores_pending_native_steer() {
        for provider_error in [true, false] {
            let pending_steers = PendingSteers::default();
            let provider = Arc::new(NativeSteeringProvider {
                steer_calls: AtomicUsize::new(0),
                behavior: NativeSteerBehavior::Blocked {
                    started: Arc::new(Notify::new()),
                    release: Arc::new(Notify::new()),
                },
            });
            let stream: MessageStream = if provider_error {
                Box::pin(futures::stream::once(async {
                    Err(ProviderError::ExecutionError("provider failed".into()))
                }))
            } else {
                Box::pin(futures::stream::empty())
            };
            let session_id = if provider_error {
                "provider-error-during-native-steer"
            } else {
                "stream-end-during-native-steer"
            };
            pending_steers
                .enqueue(session_id, Message::user().with_text("new steer"))
                .await;
            let mut active_stream =
                ActiveProviderStream::new(stream, provider, &pending_steers, session_id).await;

            let event = active_stream.next_event(&None).await;

            if provider_error {
                assert!(matches!(
                    event,
                    Some(ActiveProviderStreamEvent::ProviderOutput(next)) if next.is_err()
                ));
            } else {
                assert!(event.is_none());
            }
            let restored = pending_steers.drain(session_id).await;
            assert_eq!(restored.len(), 1);
            assert_eq!(restored[0].as_concat_text(), "new steer");
        }
    }

    #[tokio::test]
    async fn unsuccessful_native_steer_is_retained_for_the_next_prompt() {
        for (session_id, behavior) in [
            ("rejected-native-steer", NativeSteerBehavior::Rejected),
            ("failed-native-steer", NativeSteerBehavior::Error),
        ] {
            let pending_steers = PendingSteers::default();
            let provider = Arc::new(NativeSteeringProvider {
                steer_calls: AtomicUsize::new(0),
                behavior,
            });
            let stream: MessageStream = Box::pin(futures::stream::once(async {
                Ok((Some(Message::assistant().with_text("done")), None))
            }));
            pending_steers
                .enqueue(session_id, Message::user().with_text("new steer"))
                .await;
            let mut active_stream =
                ActiveProviderStream::new(stream, provider.clone(), &pending_steers, session_id)
                    .await;

            let event = timeout(TEST_TIMEOUT, active_stream.next_event(&None))
                .await
                .expect("provider output should continue after unsuccessful native steering");

            assert!(matches!(
                event,
                Some(ActiveProviderStreamEvent::ProviderOutput(_))
            ));
            let terminal_event = timeout(TEST_TIMEOUT, active_stream.next_event(&None))
                .await
                .expect("provider stream should terminate after its output");
            assert!(terminal_event.is_none());
            assert!(pending_steers.has_pending(session_id).await);
        }
    }
}
