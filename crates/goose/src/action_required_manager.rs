use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

use crate::conversation::message::{Message, MessageContent};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ElicitationOutcome {
    Accept(Value),
    Decline,
    Cancel,
}

struct PendingRequest {
    session_id: String,
    response_tx: Option<tokio::sync::oneshot::Sender<ElicitationOutcome>>,
}

pub(crate) struct PendingResponseClaim {
    request_id: String,
    pending: OwnedMutexGuard<PendingRequest>,
}

impl PendingResponseClaim {
    pub(crate) fn submit(mut self, response: ElicitationOutcome) -> Result<()> {
        let tx = self
            .pending
            .response_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("Request already completed: {}", self.request_id))?;
        drop(self.pending);

        if tx.send(response).is_err() {
            return Err(anyhow::anyhow!("Response channel closed"));
        }

        Ok(())
    }
}

pub(crate) struct ActionRequiredManager {
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

    pub(crate) fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::Lazy<ActionRequiredManager> =
            once_cell::sync::Lazy::new(ActionRequiredManager::new);
        &INSTANCE
    }

    pub(crate) async fn request_and_wait(
        &self,
        session_id: String,
        message: String,
        schema: Value,
        timeout_duration: Duration,
    ) -> Result<ElicitationOutcome> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pending_request = PendingRequest {
            session_id: session_id.clone(),
            response_tx: Some(tx),
        };
        let pending_request = Arc::new(Mutex::new(pending_request));

        self.pending
            .write()
            .await
            .insert(id.clone(), Arc::clone(&pending_request));

        let action_required_message = Message::assistant().with_content(
            MessageContent::action_required_elicitation(id.clone(), message, schema),
        );

        self.queued_requests
            .lock()
            .await
            .entry(session_id)
            .or_default()
            .push_back(action_required_message);

        let result = self
            .wait_for_response(&id, pending_request, rx, timeout_duration)
            .await;

        self.pending.write().await.remove(&id);

        result
    }

    pub(crate) async fn claim_response(
        &self,
        session_id: &str,
        request_id: &str,
    ) -> Result<PendingResponseClaim> {
        let pending_arc = self.pending_request(request_id).await?;
        let mut pending = pending_arc.lock_owned().await;

        if pending.session_id != session_id {
            return Err(anyhow::anyhow!(
                "Request {} belongs to session {}, not {}",
                request_id,
                pending.session_id,
                session_id
            ));
        }

        let tx = pending
            .response_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Request already completed: {}", request_id))?;
        if tx.is_closed() {
            pending.response_tx.take();
            return Err(anyhow::anyhow!("Response channel closed"));
        }

        Ok(PendingResponseClaim {
            request_id: request_id.to_string(),
            pending,
        })
    }

    async fn pending_request(&self, request_id: &str) -> Result<Arc<Mutex<PendingRequest>>> {
        let pending = self.pending.read().await;
        pending
            .get(request_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Request not found: {}", request_id))
    }

    async fn wait_for_response(
        &self,
        request_id: &str,
        pending_request: Arc<Mutex<PendingRequest>>,
        mut rx: tokio::sync::oneshot::Receiver<ElicitationOutcome>,
        timeout_duration: Duration,
    ) -> Result<ElicitationOutcome> {
        match timeout(timeout_duration, &mut rx).await {
            Ok(response) => Self::finish_waiting(request_id, response),
            Err(_) => {
                let mut pending = pending_request.lock().await;
                if pending.response_tx.is_some() {
                    pending.response_tx.take();
                    warn!("Timeout waiting for response: {}", request_id);
                    return Err(anyhow::anyhow!("Timeout waiting for user response"));
                }
                drop(pending);

                Self::finish_waiting(request_id, rx.await)
            }
        }
    }

    fn finish_waiting(
        request_id: &str,
        response: Result<ElicitationOutcome, tokio::sync::oneshot::error::RecvError>,
    ) -> Result<ElicitationOutcome> {
        match response {
            Ok(user_data) => Ok(user_data),
            Err(_) => {
                warn!("Response channel closed for request: {}", request_id);
                Err(anyhow::anyhow!("Response channel closed"))
            }
        }
    }

    pub(crate) async fn drain_requests_for_session(&self, session_id: &str) -> Vec<Message> {
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

        let err = match manager.claim_response("session-b", &request_id).await {
            Ok(_) => panic!("wrong session should not claim pending response"),
            Err(error) => error,
        };
        assert!(err.to_string().contains("belongs to session session-a"));

        manager
            .claim_response("session-a", &request_id)
            .await
            .unwrap()
            .submit(ElicitationOutcome::Accept(json!({ "answer": "right" })))
            .unwrap();

        let response = waiter.await.unwrap().unwrap();
        assert_eq!(
            response,
            ElicitationOutcome::Accept(json!({ "answer": "right" }))
        );
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
            .claim_response("session-a", &request_id_a)
            .await
            .unwrap()
            .submit(ElicitationOutcome::Accept(json!({ "answer": "a" })))
            .unwrap();
        manager
            .claim_response("session-b", &request_id_b)
            .await
            .unwrap()
            .submit(ElicitationOutcome::Accept(json!({ "answer": "b" })))
            .unwrap();

        assert_eq!(
            waiter_a.await.unwrap().unwrap(),
            ElicitationOutcome::Accept(json!({ "answer": "a" }))
        );
        assert_eq!(
            waiter_b.await.unwrap().unwrap(),
            ElicitationOutcome::Accept(json!({ "answer": "b" }))
        );
    }

    #[tokio::test]
    async fn claimed_response_can_complete_after_timeout_deadline() {
        let manager = Arc::new(ActionRequiredManager::new());
        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager
                    .request_and_wait(
                        "session-a".to_string(),
                        "Need input".to_string(),
                        json!({ "type": "object" }),
                        Duration::from_millis(25),
                    )
                    .await
            })
        };

        let messages = wait_for_elicitation_messages(&manager, "session-a").await;
        assert_eq!(messages.len(), 1);
        let request_id = elicitation_id(&messages[0]);

        let claim = manager
            .claim_response("session-a", &request_id)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        claim
            .submit(ElicitationOutcome::Accept(json!({ "answer": "late" })))
            .unwrap();

        assert_eq!(
            waiter.await.unwrap().unwrap(),
            ElicitationOutcome::Accept(json!({ "answer": "late" }))
        );
    }

    #[tokio::test]
    async fn request_and_wait_returns_decline_and_cancel_actions() {
        let manager = Arc::new(ActionRequiredManager::new());
        let decline_waiter = {
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
        let cancel_waiter = {
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

        let decline_messages = wait_for_elicitation_messages(&manager, "session-a").await;
        let decline_request_id = elicitation_id(&decline_messages[0]);
        let cancel_messages = wait_for_elicitation_messages(&manager, "session-b").await;
        let cancel_request_id = elicitation_id(&cancel_messages[0]);

        manager
            .claim_response("session-a", &decline_request_id)
            .await
            .unwrap()
            .submit(ElicitationOutcome::Decline)
            .unwrap();
        manager
            .claim_response("session-b", &cancel_request_id)
            .await
            .unwrap()
            .submit(ElicitationOutcome::Cancel)
            .unwrap();

        assert_eq!(
            decline_waiter.await.unwrap().unwrap(),
            ElicitationOutcome::Decline
        );
        assert_eq!(
            cancel_waiter.await.unwrap().unwrap(),
            ElicitationOutcome::Cancel
        );
    }
}
