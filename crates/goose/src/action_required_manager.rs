use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

use crate::conversation::message::{Message, MessageContent};

struct PendingRequest {
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

    pub async fn request_and_wait(
        &self,
        message: String,
        schema: Value,
        timeout_duration: Duration,
    ) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pending_request = PendingRequest {
            response_tx: Some(tx),
        };

        self.pending
            .write()
            .await
            .insert(id.clone(), Arc::new(Mutex::new(pending_request)));

        let action_required_message = Message::assistant().with_content(
            MessageContent::action_required_elicitation(id.clone(), message, schema),
        );

        if let Err(e) = self.request_tx.send(action_required_message) {
            warn!("Failed to send action required message: {}", e);
        }

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

    pub async fn submit_response(&self, request_id: String, user_data: Value) -> Result<()> {
        let pending_arc = {
            let pending = self.pending.read().await;
            pending
                .get(&request_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Request not found: {}", request_id))?
        };

        let mut pending = pending_arc.lock().await;
        if let Some(tx) = pending.response_tx.take() {
            if tx.send(user_data).is_err() {
                warn!("Failed to send response through oneshot channel");
            }
        }

        Ok(())
    }
}
