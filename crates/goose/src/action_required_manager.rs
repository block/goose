use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

use crate::conversation::message::{Message, MessageContent};

struct PendingRequest {
    session_id: String,
    response_tx: Option<tokio::sync::oneshot::Sender<Value>>,
}

pub struct ActionRequiredManager {
    pending: Arc<RwLock<HashMap<String, Arc<Mutex<PendingRequest>>>>>,
    queued_requests: Mutex<HashMap<String, VecDeque<Message>>>,
}

impl ActionRequiredManager {
    fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            queued_requests: Mutex::new(HashMap::new()),
        }
    }

    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::Lazy<ActionRequiredManager> =
            once_cell::sync::Lazy::new(ActionRequiredManager::new);
        &INSTANCE
    }

    pub async fn request_and_wait(
        &self,
        session_id: String,
        message: String,
        schema: Value,
        timeout_duration: Duration,
    ) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pending_request = PendingRequest {
            session_id: session_id.clone(),
            response_tx: Some(tx),
        };

        self.pending
            .write()
            .await
            .insert(id.clone(), Arc::new(Mutex::new(pending_request)));

        let action_required_message = Message::assistant().with_content(
            MessageContent::action_required_elicitation(id.clone(), message, schema),
        );

        self.queued_requests
            .lock()
            .await
            .entry(session_id)
            .or_default()
            .push_back(action_required_message);

        let result = match timeout(timeout_duration, rx).await {
            Ok(Ok(user_data)) => Ok(user_data),
            Ok(Err(_)) => {
                warn!("Response channel closed for request: {}", id);
                Err(anyhow::anyhow!("Response channel closed"))
            }
            Err(_) => {
                warn!("Timeout waiting for response: {}", id);
                Err(anyhow::anyhow!("Timeout waiting for user response"))
            }
        };

        self.pending.write().await.remove(&id);

        result
    }

    pub async fn submit_response(
        &self,
        session_id: &str,
        request_id: String,
        user_data: Value,
    ) -> Result<()> {
        let pending_arc = {
            let pending = self.pending.read().await;
            pending
                .get(&request_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Request not found: {}", request_id))?
        };

        let mut pending = pending_arc.lock().await;
        if pending.session_id != session_id {
            return Err(anyhow::anyhow!(
                "Request {} belongs to session {}, not {}",
                request_id,
                pending.session_id,
                session_id
            ));
        }

        if let Some(tx) = pending.response_tx.take() {
            if tx.send(user_data).is_err() {
                warn!("Failed to send response through oneshot channel");
            }
        }

        Ok(())
    }

    pub async fn drain_requests_for_session(&self, session_id: &str) -> Vec<Message> {
        self.queued_requests
            .lock()
            .await
            .remove(session_id)
            .map(|queue| queue.into_iter().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::ActionRequiredData;
    use serde_json::json;

    fn elicitation_id(message: &Message) -> String {
        match &message.content[0] {
            MessageContent::ActionRequired(action_required) => match &action_required.data {
                ActionRequiredData::Elicitation { id, .. } => id.clone(),
                _ => panic!("expected elicitation action-required message"),
            },
            _ => panic!("expected action-required message"),
        }
    }

    async fn wait_for_elicitation_messages(
        manager: &ActionRequiredManager,
        session_id: &str,
    ) -> Vec<Message> {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let messages = manager.drain_requests_for_session(session_id).await;
                if !messages.is_empty() {
                    return messages;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for elicitation message for {session_id}"))
    }

    #[tokio::test]
    async fn wrong_session_does_not_consume_pending_response() {
        let manager = Arc::new(ActionRequiredManager::new());
        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager
                    .request_and_wait(
                        "session-a".to_string(),
                        "Need input".to_string(),
                        json!({ "type": "object" }),
                        Duration::from_secs(5),
                    )
                    .await
            })
        };

        let messages = wait_for_elicitation_messages(&manager, "session-a").await;
        assert_eq!(messages.len(), 1);
        let request_id = elicitation_id(&messages[0]);

        let err = manager
            .submit_response(
                "session-b",
                request_id.clone(),
                json!({ "answer": "wrong" }),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("belongs to session session-a"));

        manager
            .submit_response("session-a", request_id, json!({ "answer": "right" }))
            .await
            .unwrap();

        let response = waiter.await.unwrap().unwrap();
        assert_eq!(response, json!({ "answer": "right" }));
    }

    #[tokio::test]
    async fn drains_only_requested_session() {
        let manager = Arc::new(ActionRequiredManager::new());
        let waiter_a = {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager
                    .request_and_wait(
                        "session-a".to_string(),
                        "Need input A".to_string(),
                        json!({ "type": "object" }),
                        Duration::from_secs(5),
                    )
                    .await
            })
        };
        let waiter_b = {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager
                    .request_and_wait(
                        "session-b".to_string(),
                        "Need input B".to_string(),
                        json!({ "type": "object" }),
                        Duration::from_secs(5),
                    )
                    .await
            })
        };

        let session_a_messages = wait_for_elicitation_messages(&manager, "session-a").await;
        assert_eq!(session_a_messages.len(), 1);
        let request_id_a = elicitation_id(&session_a_messages[0]);

        let empty_messages = manager.drain_requests_for_session("session-a").await;
        assert!(empty_messages.is_empty());

        let session_b_messages = wait_for_elicitation_messages(&manager, "session-b").await;
        assert_eq!(session_b_messages.len(), 1);
        let request_id_b = elicitation_id(&session_b_messages[0]);

        manager
            .submit_response("session-a", request_id_a, json!({ "answer": "a" }))
            .await
            .unwrap();
        manager
            .submit_response("session-b", request_id_b, json!({ "answer": "b" }))
            .await
            .unwrap();

        assert_eq!(waiter_a.await.unwrap().unwrap(), json!({ "answer": "a" }));
        assert_eq!(waiter_b.await.unwrap().unwrap(), json!({ "answer": "b" }));
    }
}
