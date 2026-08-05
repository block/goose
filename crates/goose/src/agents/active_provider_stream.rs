use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::futures::OwnedNotified;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use super::pending_steers::PendingSteers;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider};
use crate::utils::is_token_cancelled;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;

pub(super) type ProviderStreamItem =
    Result<(Option<Message>, Option<ProviderUsage>), ProviderError>;

pub(super) enum ProviderStreamEvent {
    ProviderOutput(ProviderStreamItem),
    NativeSteer {
        message: Message,
        result: Result<(), ProviderError>,
    },
}

enum PendingSteerHandling {
    TryNativeDelivery,
    SendAsNextPrompt,
}

pub(super) struct ActiveProviderStream<'a> {
    stream: MessageStream,
    provider: Arc<dyn Provider>,
    pending_steers: &'a PendingSteers,
    session_id: &'a str,
    steer_notifier: Arc<Notify>,
    steer_notification_waiter: Pin<Box<OwnedNotified>>,
    // One notification can represent multiple queued steers.
    steers_may_remain_queued: bool,
    pending_steer_handling: PendingSteerHandling,
    native_steer_errored: bool,
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
            steers_may_remain_queued: true,
            pending_steer_handling: PendingSteerHandling::TryNativeDelivery,
            native_steer_errored: false,
        }
    }

    pub(super) async fn next_event(
        &mut self,
        cancellation_token: &Option<CancellationToken>,
    ) -> Option<ProviderStreamEvent> {
        loop {
            if self.native_steer_errored || is_token_cancelled(cancellation_token) {
                return None;
            }

            if matches!(
                self.pending_steer_handling,
                PendingSteerHandling::TryNativeDelivery
            ) && self.steers_may_remain_queued
            {
                let Some(message) = self.pending_steers.pop_front(self.session_id).await else {
                    self.steers_may_remain_queued = false;
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
                        return Some(ProviderStreamEvent::NativeSteer {
                            message,
                            result: Ok(()),
                        });
                    }
                    Ok(false) => {
                        self.pending_steers
                            .restore_front(self.session_id, message)
                            .await;
                        self.pending_steer_handling = PendingSteerHandling::SendAsNextPrompt;
                    }
                    Err(error) => {
                        self.native_steer_errored = true;
                        return Some(ProviderStreamEvent::NativeSteer {
                            message,
                            result: Err(error),
                        });
                    }
                }

                continue;
            }

            if matches!(
                self.pending_steer_handling,
                PendingSteerHandling::SendAsNextPrompt
            ) {
                return self
                    .stream
                    .next()
                    .await
                    .map(ProviderStreamEvent::ProviderOutput);
            }

            tokio::select! {
                biased;

                _ = self.steer_notification_waiter.as_mut() => {
                    self.steer_notification_waiter
                        .as_mut()
                        .set(Arc::clone(&self.steer_notifier).notified_owned());
                    self.steers_may_remain_queued = true;
                }

                next = self.stream.next() => {
                    return next.map(ProviderStreamEvent::ProviderOutput);
                }
            }
        }
    }

    pub(super) fn native_steer_errored(&self) -> bool {
        self.native_steer_errored
    }
}
