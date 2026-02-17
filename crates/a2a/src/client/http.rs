//! HTTP-based A2A client using reqwest, following the JS client.ts patterns.

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::error::A2AError;
use crate::jsonrpc::{self, JsonRpcRequest, JsonRpcResponse};
use crate::types::agent_card::AgentCard;
use crate::types::config::TaskPushNotificationConfig;
use crate::types::core::Task;
use crate::types::requests::*;
use crate::types::responses::*;

const WELL_KNOWN_PATH: &str = "/.well-known/agent-card.json";

/// A2A HTTP client for communicating with remote A2A-compliant agents.
pub struct A2AClient {
    http: Client,
    agent_card: Option<AgentCard>,
    base_url: String,
    rpc_url: Option<String>,
    next_id: std::sync::atomic::AtomicU64,
}

impl A2AClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            agent_card: None,
            base_url: base_url.into(),
            rpc_url: None,
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    pub fn with_http_client(mut self, client: Client) -> Self {
        self.http = client;
        self
    }

    pub fn with_agent_card(mut self, card: AgentCard) -> Self {
        if let Some(iface) = card.supported_interfaces.first() {
            self.rpc_url = Some(iface.url.clone());
        }
        self.agent_card = Some(card);
        self
    }

    fn next_id(&self) -> serde_json::Value {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        serde_json::Value::Number(serde_json::Number::from(id))
    }

    fn rpc_url(&self) -> Result<&str, A2AError> {
        self.rpc_url
            .as_deref()
            .ok_or_else(|| A2AError::internal_error("No RPC endpoint. Fetch agent card first."))
    }

    /// Fetch the agent card from the well-known URL.
    pub async fn fetch_agent_card(&mut self) -> Result<AgentCard, A2AError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), WELL_KNOWN_PATH);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| A2AError::internal_error(e.to_string()))?;

        let card: AgentCard = resp
            .json()
            .await
            .map_err(|e| A2AError::parse_error(e.to_string()))?;

        if let Some(iface) = card.supported_interfaces.first() {
            self.rpc_url = Some(iface.url.clone());
        }
        self.agent_card = Some(card.clone());
        Ok(card)
    }

    pub fn agent_card(&self) -> Option<&AgentCard> {
        self.agent_card.as_ref()
    }

    async fn invoke_rpc<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, A2AError> {
        let url = self.rpc_url()?;
        let request = JsonRpcRequest::new(method, self.next_id(), Some(params));

        let resp = self
            .http
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| A2AError::internal_error(e.to_string()))?;

        let rpc_resp: JsonRpcResponse = resp
            .json()
            .await
            .map_err(|e| A2AError::parse_error(e.to_string()))?;

        if let Some(err) = rpc_resp.error {
            return Err(match err.code {
                -32700 => A2AError::parse_error(err.message),
                -32600 => A2AError::invalid_request(err.message),
                -32601 => A2AError::method_not_found(err.message),
                -32602 => A2AError::invalid_params(err.message),
                -32001 => A2AError::task_not_found(err.message),
                -32002 => A2AError::task_not_cancelable(err.message),
                -32003 => A2AError::PushNotificationNotSupported,
                -32004 => A2AError::unsupported_operation(err.message),
                -32007 => A2AError::AuthenticatedExtendedCardNotConfigured,
                _ => A2AError::internal_error(err.message),
            });
        }

        let result = rpc_resp
            .result
            .ok_or_else(|| A2AError::internal_error("Missing result in response"))?;

        serde_json::from_value(result).map_err(|e| A2AError::parse_error(e.to_string()))
    }

    /// Send a message to the remote agent (blocking or non-blocking).
    pub async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::SEND_MESSAGE, params)
            .await
    }

    /// Get a task by ID.
    pub async fn get_task(&self, request: GetTaskRequest) -> Result<Task, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::GET_TASK, params).await
    }

    /// List tasks with pagination.
    pub async fn list_tasks(
        &self,
        request: ListTasksRequest,
    ) -> Result<ListTasksResponse, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::LIST_TASKS, params).await
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, request: CancelTaskRequest) -> Result<Task, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::CANCEL_TASK, params).await
    }

    /// Create or update push notification config.
    pub async fn set_push_notification_config(
        &self,
        request: CreateTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::SET_PUSH_CONFIG, params)
            .await
    }

    /// Get push notification config.
    pub async fn get_push_notification_config(
        &self,
        request: GetTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::GET_PUSH_CONFIG, params)
            .await
    }

    /// Delete push notification config.
    pub async fn delete_push_notification_config(
        &self,
        request: DeleteTaskPushNotificationConfigRequest,
    ) -> Result<(), A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc::<serde_json::Value>(jsonrpc::methods::DELETE_PUSH_CONFIG, params)
            .await?;
        Ok(())
    }

    /// Get extended agent card with authentication.
    pub async fn get_authenticated_extended_card(
        &self,
        request: GetExtendedAgentCardRequest,
    ) -> Result<AgentCard, A2AError> {
        let params =
            serde_json::to_value(&request).map_err(|e| A2AError::internal_error(e.to_string()))?;
        self.invoke_rpc(jsonrpc::methods::GET_EXTENDED_CARD, params)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = A2AClient::new("https://example.com");
        assert!(client.agent_card().is_none());
    }

    #[test]
    fn test_client_with_agent_card() {
        let card = AgentCard {
            name: "TestAgent".to_string(),
            description: "Test".to_string(),
            supported_interfaces: vec![crate::types::agent_card::AgentInterface {
                url: "https://example.com/a2a".to_string(),
                protocol_binding: Some("JSONRPC".to_string()),
                tenant: None,
                protocol_version: None,
            }],
            provider: None,
            version: None,
            protocol_version: None,
            capabilities: None,
            security_schemes: serde_json::Value::Null,
            security: vec![],
            default_input_modes: vec![],
            default_output_modes: vec![],
            skills: vec![],
            documentation_url: None,
            icon_url: None,
            signatures: vec![],
        };

        let client = A2AClient::new("https://example.com").with_agent_card(card);
        assert!(client.agent_card().is_some());
        assert_eq!(client.rpc_url.as_deref(), Some("https://example.com/a2a"));
    }
}
