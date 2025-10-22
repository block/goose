//! A unified interface for requesting user approval
//! for various operations like tool calls, MCP sampling, etc

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, OnceCell, RwLock};
use utoipa::ToSchema;
use uuid::Uuid;

static GLOBAL_APPROVAL_STATE: OnceCell<Arc<ApprovalState>> = OnceCell::const_new();

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ApprovalType {
    #[serde(rename_all = "camelCase")]
    ToolCall {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        principal_type: String,
    },
    #[serde(rename_all = "camelCase")]
    Sampling {
        extension_name: String,
        #[schema(value_type = Vec<Object>)]
        messages: Vec<rmcp::model::SamplingMessage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system_prompt: Option<String>,
        max_tokens: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    AllowOnce,
    AlwaysAllow,
    Deny,
}

impl ApprovalAction {
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            ApprovalAction::AllowOnce | ApprovalAction::AlwaysAllow
        )
    }
}

#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    async fn request_approval(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub request_id: String,
    pub session_id: String,
    #[serde(flatten)]
    pub approval_type: ApprovalType,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResponse {
    pub request_id: String,
    pub action: ApprovalAction,
}

#[derive(Clone)]
pub struct ApprovalState {
    /// Channel for broadcasting approval requests to connected UI clients
    broadcast_tx: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
    /// Pending approval requests awaiting user response
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalAction>>>>,
}

impl ApprovalState {
    pub fn new() -> Self {
        Self {
            broadcast_tx: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create the global approval state instance
    pub async fn global() -> Arc<Self> {
        GLOBAL_APPROVAL_STATE
            .get_or_init(|| async { Arc::new(Self::new()) })
            .await
            .clone()
    }

    /// Submit a response to an approval request
    pub async fn submit_response(
        &self,
        request_id: String,
        action: ApprovalAction,
    ) -> Result<(), String> {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(&request_id) {
            tx.send(action)
                .map_err(|_| "Failed to send response".to_string())?;
            Ok(())
        } else {
            Err("Request not found or already responded".to_string())
        }
    }

    /// Add a new SSE client connection
    pub async fn add_client(&self, tx: mpsc::Sender<String>) {
        let mut senders = self.broadcast_tx.write().await;
        senders.push(tx);
    }
}

impl Default for ApprovalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the ApprovalHandler trait from goose crate
#[async_trait::async_trait]
impl ApprovalHandler for ApprovalState {
    async fn request_approval(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // store: request id -> channel so the channel can be used to continue the task when requests come back in
        let mut pending = self.pending_requests.write().await;
        pending.insert(request_id.clone(), tx);

        let request = ApprovalRequest {
            request_id: request_id.clone(),
            session_id,
            approval_type,
        };

        // Send the approval request to the user
        let message = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let sse_message = format!("data: {}\n\n", message);
        let senders = self.broadcast_tx.read().await;
        for sender in senders.iter() {
            let _ = sender.send(sse_message.clone()).await;
        }

        // timeout and expire state in 5 minutes if user has not responded
        match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(action)) => Ok(action),
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                // Timeout - clean up pending request
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err("Approval request timed out".to_string())
            }
        }
    }
}
