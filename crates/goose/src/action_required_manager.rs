use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::conversation::message::Message;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum RequestStatus {
    Pending,
    Completed(Value),
    TimedOut,
}

struct PendingRequest {
    status: RequestStatus,
    response_tx: Option<tokio::sync::oneshot::Sender<Value>>,
}

pub struct ActionRequiredManager {
    pending: Arc<RwLock<HashMap<String, Arc<Mutex<PendingRequest>>>>>,
    request_tx: mpsc::UnboundedSender<Message>,
    pub request_rx: Mutex<mpsc::UnboundedReceiver<Message>>,
}

impl ActionRequiredManager {
    fn new() -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            request_tx,
            request_rx: Mutex::new(request_rx),
        }
    }

    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::Lazy<ActionRequiredManager> =
            once_cell::sync::Lazy::new(ActionRequiredManager::new);
        &INSTANCE
    }

    pub async fn create_request(&self, message: String, schema: Value) -> String {
        let id = Uuid::new_v4().to_string();

        let (response_tx, _response_rx) = tokio::sync::oneshot::channel();

        let pending_request = PendingRequest {
            status: RequestStatus::Pending,
            response_tx: Some(response_tx),
        };

        self.pending
            .write()
            .await
            .insert(id.clone(), Arc::new(Mutex::new(pending_request)));

        use crate::conversation::message::MessageContent;

        let action_required_message = Message::assistant().with_content(
            MessageContent::action_required_elicitation(id.clone(), message, schema),
        );

        let _ = self.request_tx.send(action_required_message);

        id
    }

    pub async fn wait_for_response(
        &self,
        request_id: &str,
        timeout_duration: Duration,
    ) -> Result<Value> {
        let pending_arc = {
            let pending = self.pending.read().await;
            pending
                .get(request_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Request not found: {}", request_id))?
        };

        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut pending = pending_arc.lock().await;
            pending.response_tx = Some(tx);
        }

        match timeout(timeout_duration, rx).await {
            Ok(Ok(user_data)) => {
                debug!("Received response for request: {}", request_id);

                {
                    let mut pending = pending_arc.lock().await;
                    pending.status = RequestStatus::Completed(user_data.clone());
                }

                Ok(user_data)
            }
            Ok(Err(_)) => {
                warn!("Response channel closed for request: {}", request_id);
                Err(anyhow::anyhow!("Response channel closed"))
            }
            Err(_) => {
                warn!("Timeout waiting for response: {}", request_id);

                {
                    let mut pending = pending_arc.lock().await;
                    pending.status = RequestStatus::TimedOut;
                }

                Err(anyhow::anyhow!("Timeout waiting for user response"))
            }
        }
    }

    pub async fn submit_response(&self, request_id: String, user_data: Value) -> Result<()> {
        debug!("Submitting response for request: {}", request_id);

        let pending_arc = {
            let pending = self.pending.read().await;
            pending
                .get(&request_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Request not found: {}", request_id))?
        };

        {
            let mut pending = pending_arc.lock().await;
            if let Some(tx) = pending.response_tx.take() {
                if tx.send(user_data.clone()).is_err() {
                    warn!("Failed to send response through oneshot channel");
                }
            }
        }

        Ok(())
    }

    pub async fn remove_request(&self, request_id: &str) {
        self.pending.write().await.remove(request_id);
    }
}
