use std::pin::Pin;
use std::sync::Arc;

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

pub(super) enum ActiveProviderStreamEvent {
    ProviderOutput(ProviderStreamItem),
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
        }
    }

    pub(super) async fn next_event(
        &mut self,
        cancellation_token: &Option<CancellationToken>,
    ) -> Option<ActiveProviderStreamEvent> {
        loop {
            if is_token_cancelled(cancellation_token) {
                return None;
            }

            let native_steering_enabled = matches!(
                self.pending_steer_delivery_strategy,
                PendingSteerDeliveryStrategy::NativeSteering
            );

            if native_steering_enabled && self.queue_scan_needed {
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

                match self
                    .provider
                    .steer_natively(self.session_id, &message)
                    .await
                {
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

                continue;
            }

            tokio::select! {
                biased;

                _ = self.steer_notification_waiter.as_mut(), if native_steering_enabled => {
                    self.steer_notification_waiter
                        .as_mut()
                        .set(Arc::clone(&self.steer_notifier).notified_owned());
                    self.queue_scan_needed = true;
                }

                next = self.stream.next() => {
                    return next.map(ActiveProviderStreamEvent::ProviderOutput);
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
            Ok(true)
        }
    }

    #[tokio::test]
    async fn steer_notification_wakes_active_provider_stream() {
        let pending_steers = PendingSteers::default();
        let provider = Arc::new(NativeSteeringProvider {
            steer_calls: AtomicUsize::new(0),
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
}
