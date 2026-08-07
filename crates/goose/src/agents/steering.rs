use std::collections::VecDeque;

use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

use crate::conversation::message::Message;

struct QueuedSteer {
    entry_id: u64,
    message: Message,
    hook_complete: bool,
}

#[derive(Default)]
struct SteeringQueueState {
    items: VecDeque<QueuedSteer>,
    next_entry_id: u64,
}

#[derive(Default)]
pub(crate) struct SteeringQueue {
    state: Mutex<SteeringQueueState>,
    hook_complete_notify: Notify,
}

impl SteeringQueue {
    pub(crate) async fn enqueue(&self, message: Message) -> u64 {
        let mut state = self.state.lock().await;
        let entry_id = state.next_entry_id;
        state.next_entry_id = state
            .next_entry_id
            .checked_add(1)
            .expect("steering queue entry ID overflow");
        state.items.push_back(QueuedSteer {
            entry_id,
            message,
            hook_complete: false,
        });
        entry_id
    }

    pub(crate) async fn mark_hook_complete(&self, entry_id: u64) -> bool {
        {
            let mut state = self.state.lock().await;
            let Some(entry) = state
                .items
                .iter_mut()
                .find(|entry| entry.entry_id == entry_id)
            else {
                return false;
            };
            if entry.hook_complete {
                return false;
            }
            entry.hook_complete = true;
        }

        self.hook_complete_notify.notify_one();
        true
    }

    pub(crate) async fn has_pending(&self) -> bool {
        !self.state.lock().await.items.is_empty()
    }

    pub(crate) async fn drain_available(&self) -> Vec<Message> {
        let mut state = self.state.lock().await;
        let available = state
            .items
            .iter()
            .take_while(|entry| entry.hook_complete)
            .count();
        state
            .items
            .drain(..available)
            .map(|entry| entry.message.with_steer())
            .collect()
    }

    pub(crate) async fn wait_until_steer_can_be_used(&self, cancel: &CancellationToken) {
        loop {
            let notified = self.hook_complete_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            match self.state.lock().await.items.front() {
                Some(entry) if entry.hook_complete => return,
                None => return,
                Some(_) => {}
            }

            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = notified.as_mut() => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SteeringQueue;
    use crate::conversation::message::Message;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn available_entries_cannot_pass_an_earlier_incomplete_hook() {
        let queue = SteeringQueue::default();
        let first = queue.enqueue(Message::user().with_text("first")).await;
        let second = queue.enqueue(Message::user().with_text("second")).await;

        assert!(queue.mark_hook_complete(second).await);
        assert!(queue.drain_available().await.is_empty());

        assert!(queue.mark_hook_complete(first).await);
        let messages = queue.drain_available().await;
        assert_eq!(
            messages
                .iter()
                .map(Message::as_concat_text)
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert!(messages.iter().all(|message| message.metadata.steer));
    }

    #[tokio::test]
    async fn marking_a_hook_complete_is_idempotent() {
        let queue = SteeringQueue::default();
        let entry_id = queue.enqueue(Message::user().with_text("steer")).await;

        assert!(queue.mark_hook_complete(entry_id).await);
        assert!(!queue.mark_hook_complete(entry_id).await);
        assert_eq!(queue.drain_available().await.len(), 1);
    }

    #[tokio::test]
    async fn enqueue_does_not_notify_until_the_hook_is_complete() {
        let queue = SteeringQueue::default();
        let notified = queue.hook_complete_notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let entry_id = queue.enqueue(Message::user().with_text("steer")).await;
        assert!(futures::poll!(notified.as_mut()).is_pending());

        assert!(queue.mark_hook_complete(entry_id).await);
        notified.await;
    }

    #[tokio::test]
    async fn completed_hook_is_observed_without_waiting_for_another_notification() {
        let queue = SteeringQueue::default();
        let entry_id = queue.enqueue(Message::user().with_text("steer")).await;
        assert!(queue.mark_hook_complete(entry_id).await);

        queue
            .wait_until_steer_can_be_used(&CancellationToken::new())
            .await;
    }

    #[tokio::test]
    async fn cancellation_interrupts_waiting_for_a_steer_hook() {
        let queue = Arc::new(SteeringQueue::default());
        queue.enqueue(Message::user().with_text("steer")).await;
        let cancel = CancellationToken::new();
        let wait = tokio::spawn({
            let queue = Arc::clone(&queue);
            let cancel = cancel.clone();
            async move { queue.wait_until_steer_can_be_used(&cancel).await }
        });

        cancel.cancel();
        wait.await.expect("steering waiter should finish");
        assert!(queue.has_pending().await);
        assert!(queue.drain_available().await.is_empty());
    }
}
