use std::collections::VecDeque;

use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;

const NATIVE_STEER_OPERATION: &str = "llm";
const NATIVE_STEER_DELIVERED: &str = "native_steer_delivered";
const STEERING_CANCELLED_TOOL_RESPONSE: &str =
    "Tool call was cancelled because steering arrived before execution";

pub(crate) fn mark_native_steer_delivered(message: &mut Message) {
    message.metadata.steer = true;
    message.metadata.set_operation_note(
        NATIVE_STEER_OPERATION,
        NATIVE_STEER_DELIVERED,
        true.into(),
    );
}

pub(crate) fn was_native_steer_delivered(message: &Message) -> bool {
    message
        .metadata
        .operation_note(NATIVE_STEER_OPERATION, NATIVE_STEER_DELIVERED)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn tool_cancellation_response_for_steering(messages: &Conversation) -> Option<Message> {
    let answered = messages
        .iter()
        .flat_map(Message::get_tool_response_ids)
        .collect::<std::collections::HashSet<_>>();
    let mut response = Message::user();
    for content in messages.iter().flat_map(|message| &message.content) {
        if let MessageContent::ToolRequest(request) = content {
            if request.was_executed_externally() || answered.contains(request.id.as_str()) {
                continue;
            }
            response.add_tool_response_with_metadata(
                request.id.clone(),
                Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(STEERING_CANCELLED_TOOL_RESPONSE),
                ])),
                request.metadata.as_ref(),
            );
        }
    }
    (!response.get_tool_response_ids().is_empty()).then_some(response)
}

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

    pub(crate) async fn peek_next_ready(&self) -> Option<(u64, Message)> {
        self.state
            .lock()
            .await
            .items
            .front()
            .filter(|entry| entry.hook_complete)
            .map(|entry| (entry.entry_id, entry.message.clone()))
    }

    pub(crate) async fn remove_next_ready(&self, entry_id: u64) -> Option<Message> {
        let mut state = self.state.lock().await;
        let matches = state
            .items
            .front()
            .is_some_and(|entry| entry.hook_complete && entry.entry_id == entry_id);

        if !matches {
            return None;
        }

        state
            .items
            .pop_front()
            .map(|entry| entry.message.with_steer())
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

    pub(crate) async fn wait_for_next_ready_or_cancelled(&self, cancel: &CancellationToken) {
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

    pub(crate) async fn wait_for_next_ready(&self) {
        loop {
            let notified = self.hook_complete_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            if self
                .state
                .lock()
                .await
                .items
                .front()
                .is_some_and(|entry| entry.hook_complete)
            {
                return;
            }

            notified.as_mut().await;
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
        assert!(queue.peek_next_ready().await.is_none());
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
    async fn next_ready_entry_is_retained_until_the_matching_entry_is_removed() {
        let queue = SteeringQueue::default();
        let entry_id = queue.enqueue(Message::user().with_text("steer")).await;
        assert!(queue.mark_hook_complete(entry_id).await);

        let (peeked_id, message) = queue.peek_next_ready().await.expect("ready steer");
        assert_eq!(peeked_id, entry_id);
        assert_eq!(message.as_concat_text(), "steer");
        assert!(!message.metadata.steer);
        assert!(queue.has_pending().await);

        assert!(queue.remove_next_ready(entry_id + 1).await.is_none());
        assert!(queue.has_pending().await);

        let message = queue
            .remove_next_ready(entry_id)
            .await
            .expect("matching steer");
        assert_eq!(message.as_concat_text(), "steer");
        assert!(message.metadata.steer);
        assert!(!queue.has_pending().await);
    }

    #[tokio::test]
    async fn next_ready_wait_observes_existing_and_new_readiness() {
        let queue = SteeringQueue::default();
        let first = queue.enqueue(Message::user().with_text("first")).await;
        assert!(queue.mark_hook_complete(first).await);
        queue.wait_for_next_ready().await;
        assert!(queue.remove_next_ready(first).await.is_some());

        let second = queue.enqueue(Message::user().with_text("second")).await;
        let mut wait = Box::pin(queue.wait_for_next_ready());
        assert!(futures::poll!(wait.as_mut()).is_pending());
        assert!(queue.mark_hook_complete(second).await);
        wait.await;
        assert!(queue.peek_next_ready().await.is_some());

        assert!(queue.remove_next_ready(second).await.is_some());
        let third = queue.enqueue(Message::user().with_text("third")).await;
        let mut abandoned_wait = Box::pin(queue.wait_for_next_ready());
        assert!(futures::poll!(abandoned_wait.as_mut()).is_pending());
        drop(abandoned_wait);
        assert!(queue.mark_hook_complete(third).await);
        assert_eq!(queue.peek_next_ready().await.unwrap().0, third);
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
            .wait_for_next_ready_or_cancelled(&CancellationToken::new())
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
            async move { queue.wait_for_next_ready_or_cancelled(&cancel).await }
        });

        cancel.cancel();
        wait.await.expect("steering waiter should finish");
        assert!(queue.has_pending().await);
        assert!(queue.drain_available().await.is_empty());
    }
}
