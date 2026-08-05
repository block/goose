use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use crate::conversation::message::Message;

#[derive(Default)]
pub(super) struct PendingSteers {
    by_session: Mutex<HashMap<String, PendingSteerState>>,
}

#[derive(Default)]
struct PendingSteerState {
    messages: VecDeque<Message>,
    notify: Arc<Notify>,
}

impl PendingSteers {
    pub(super) async fn enqueue(&self, session_id: &str, message: Message) {
        let notify = {
            let mut by_session = self.by_session.lock().await;
            let state = by_session.entry(session_id.to_string()).or_default();
            state.messages.push_back(message);
            Arc::clone(&state.notify)
        };
        notify.notify_one();
    }

    pub(super) async fn notifier(&self, session_id: &str) -> Arc<Notify> {
        let mut by_session = self.by_session.lock().await;
        Arc::clone(&by_session.entry(session_id.to_string()).or_default().notify)
    }

    pub(super) async fn discard(&self, session_id: &str) {
        self.by_session.lock().await.remove(session_id);
    }

    pub(super) async fn has_pending(&self, session_id: &str) -> bool {
        self.by_session
            .lock()
            .await
            .get(session_id)
            .is_some_and(|state| !state.messages.is_empty())
    }

    pub(super) async fn drain(&self, session_id: &str) -> Vec<Message> {
        let messages = self
            .by_session
            .lock()
            .await
            .get_mut(session_id)
            .map(|state| std::mem::take(&mut state.messages))
            .unwrap_or_default();

        messages.into_iter().map(Message::with_steer).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::PendingSteers;
    use crate::conversation::message::Message;

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);

    #[tokio::test]
    async fn notifier_remains_stable_across_drains() {
        let pending_steers = PendingSteers::default();
        let session_id = "stable-notifier";
        let notifier = pending_steers.notifier(session_id).await;

        for text in ["first steer", "second steer"] {
            pending_steers
                .enqueue(session_id, Message::user().with_text(text))
                .await;

            timeout(TEST_TIMEOUT, notifier.notified())
                .await
                .expect("the original notifier should be signalled");

            let messages = pending_steers.drain(session_id).await;
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].as_concat_text(), text);
            assert!(messages[0].metadata.steer);
        }
    }
}
