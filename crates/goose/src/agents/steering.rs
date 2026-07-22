use std::collections::{HashMap, VecDeque};

use tokio::sync::Mutex;

use crate::conversation::message::Message;

#[derive(Default)]
pub(super) struct PendingSteers {
    messages: Mutex<HashMap<String, VecDeque<Message>>>,
}

impl PendingSteers {
    pub(super) async fn push(&self, session_id: &str, message: Message) {
        self.messages
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .push_back(message);
    }

    pub(super) async fn discard(&self, session_id: &str) {
        self.messages.lock().await.remove(session_id);
    }

    pub(super) async fn has_pending(&self, session_id: &str) -> bool {
        self.messages
            .lock()
            .await
            .get(session_id)
            .is_some_and(|messages| !messages.is_empty())
    }

    pub(super) async fn drain(&self, session_id: &str) -> Vec<Message> {
        self.messages
            .lock()
            .await
            .remove(session_id)
            .map(|messages| messages.into_iter().map(Message::with_steer).collect())
            .unwrap_or_default()
    }
}
