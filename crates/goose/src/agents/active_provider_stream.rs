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

            if matches!(
                self.pending_steer_delivery_strategy,
                PendingSteerDeliveryStrategy::NativeSteering
            ) && self.queue_scan_needed
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

            if matches!(
                self.pending_steer_delivery_strategy,
                PendingSteerDeliveryStrategy::NextPrompt
            ) {
                return self
                    .stream
                    .next()
                    .await
                    .map(ActiveProviderStreamEvent::ProviderOutput);
            }

            tokio::select! {
                biased;

                _ = self.steer_notification_waiter.as_mut() => {
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
