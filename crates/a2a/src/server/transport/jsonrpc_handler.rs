//! JSON-RPC transport handler for A2A protocol.
//!
//! Routes incoming JSON-RPC requests to the appropriate request handler methods.

use std::sync::Arc;

use futures::stream::StreamExt;
use serde_json::Value;

use crate::error::A2AError;
use crate::jsonrpc::{methods, JsonRpcRequest, JsonRpcResponse};
use crate::server::executor::AgentExecutor;
use crate::server::request_handler::DefaultRequestHandler;
use crate::server::store::TaskStore;
use crate::types::requests::{
    CancelTaskRequest, GetTaskRequest, ListTasksRequest, SendMessageRequest,
};

/// JSON-RPC transport handler that dispatches to a `DefaultRequestHandler`.
pub struct JsonRpcHandler<S: TaskStore, E: AgentExecutor> {
    handler: Arc<DefaultRequestHandler<S, E>>,
}

impl<S: TaskStore + Clone + 'static, E: AgentExecutor + 'static> JsonRpcHandler<S, E> {
    pub fn new(handler: Arc<DefaultRequestHandler<S, E>>) -> Self {
        Self { handler }
    }

    /// Handle a single JSON-RPC request and return a response.
    pub async fn handle_request(&self, raw: &Value) -> JsonRpcResponse {
        let request: JsonRpcRequest = match serde_json::from_value(raw.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    Value::Null,
                    A2AError::parse_error(e.to_string()).to_jsonrpc_error(),
                );
            }
        };

        let id = request.id.clone();
        let params = request.params.unwrap_or(Value::Object(Default::default()));

        match request.method.as_str() {
            methods::SEND_MESSAGE => self.handle_send_message(&id, &params).await,
            methods::GET_TASK => self.handle_get_task(&id, &params).await,
            methods::LIST_TASKS => self.handle_list_tasks(&id, &params).await,
            methods::CANCEL_TASK => self.handle_cancel_task(&id, &params).await,
            methods::GET_EXTENDED_CARD => self.handle_get_agent_card(&id),
            _ => JsonRpcResponse::error(
                id,
                A2AError::method_not_found(request.method.clone()).to_jsonrpc_error(),
            ),
        }
    }

    /// Handle a streaming JSON-RPC request, returning SSE event chunks.
    pub async fn handle_stream_request(
        &self,
        raw: &Value,
    ) -> Result<impl futures::Stream<Item = Result<String, A2AError>>, JsonRpcResponse> {
        let request: JsonRpcRequest = match serde_json::from_value(raw.clone()) {
            Ok(r) => r,
            Err(e) => {
                return Err(JsonRpcResponse::error(
                    Value::Null,
                    A2AError::parse_error(e.to_string()).to_jsonrpc_error(),
                ));
            }
        };

        let id = request.id.clone();

        if request.method != methods::SEND_STREAM {
            return Err(JsonRpcResponse::error(
                id,
                A2AError::method_not_found(request.method.clone()).to_jsonrpc_error(),
            ));
        }

        let send_request: SendMessageRequest =
            serde_json::from_value(request.params.unwrap_or(Value::Object(Default::default())))
                .map_err(|e| {
                    JsonRpcResponse::error(
                        id.clone(),
                        A2AError::invalid_params(e.to_string()).to_jsonrpc_error(),
                    )
                })?;

        let stream = self
            .handler
            .send_message_stream(&send_request)
            .await
            .map_err(|e| JsonRpcResponse::error(id.clone(), e.to_jsonrpc_error()))?;

        let sse_stream = stream.map(move |result| match result {
            Ok(response) => {
                let rpc_response = JsonRpcResponse::success(
                    id.clone(),
                    serde_json::to_value(&response).unwrap_or_default(),
                );
                serde_json::to_string(&rpc_response)
                    .map_err(|e| A2AError::internal_error(e.to_string()))
            }
            Err(e) => Err(e),
        });

        Ok(sse_stream)
    }

    async fn handle_send_message(&self, id: &Value, params: &Value) -> JsonRpcResponse {
        let request: SendMessageRequest = match serde_json::from_value(params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id.clone(),
                    A2AError::invalid_params(e.to_string()).to_jsonrpc_error(),
                );
            }
        };

        match self.handler.send_message(&request).await {
            Ok(response) => JsonRpcResponse::success(
                id.clone(),
                serde_json::to_value(&response).unwrap_or_default(),
            ),
            Err(e) => JsonRpcResponse::error(id.clone(), e.to_jsonrpc_error()),
        }
    }

    async fn handle_get_task(&self, id: &Value, params: &Value) -> JsonRpcResponse {
        let request: GetTaskRequest = match serde_json::from_value(params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id.clone(),
                    A2AError::invalid_params(e.to_string()).to_jsonrpc_error(),
                );
            }
        };

        match self.handler.get_task(&request).await {
            Ok(task) => JsonRpcResponse::success(
                id.clone(),
                serde_json::to_value(&task).unwrap_or_default(),
            ),
            Err(e) => JsonRpcResponse::error(id.clone(), e.to_jsonrpc_error()),
        }
    }

    async fn handle_list_tasks(&self, id: &Value, params: &Value) -> JsonRpcResponse {
        let request: ListTasksRequest = match serde_json::from_value(params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id.clone(),
                    A2AError::invalid_params(e.to_string()).to_jsonrpc_error(),
                );
            }
        };

        match self.handler.list_tasks(&request).await {
            Ok(response) => JsonRpcResponse::success(
                id.clone(),
                serde_json::to_value(&response).unwrap_or_default(),
            ),
            Err(e) => JsonRpcResponse::error(id.clone(), e.to_jsonrpc_error()),
        }
    }

    async fn handle_cancel_task(&self, id: &Value, params: &Value) -> JsonRpcResponse {
        let request: CancelTaskRequest = match serde_json::from_value(params.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id.clone(),
                    A2AError::invalid_params(e.to_string()).to_jsonrpc_error(),
                );
            }
        };

        match self.handler.cancel_task(&request).await {
            Ok(task) => JsonRpcResponse::success(
                id.clone(),
                serde_json::to_value(&task).unwrap_or_default(),
            ),
            Err(e) => JsonRpcResponse::error(id.clone(), e.to_jsonrpc_error()),
        }
    }

    fn handle_get_agent_card(&self, id: &Value) -> JsonRpcResponse {
        let card = self.handler.get_agent_card();
        JsonRpcResponse::success(id.clone(), serde_json::to_value(&card).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::context::RequestContext;
    use crate::server::executor::AgentExecutor;
    use crate::server::store::InMemoryTaskStore;
    use crate::types::agent_card::AgentCard;
    use crate::types::core::{Artifact, TaskState, TaskStatus};
    use crate::types::events::{
        AgentExecutionEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent,
    };
    use tokio::sync::mpsc;

    struct EchoExecutor;

    #[async_trait::async_trait]
    impl AgentExecutor for EchoExecutor {
        async fn execute(
            &self,
            context: RequestContext,
            tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), A2AError> {
            let _ = tx
                .send(AgentExecutionEvent::ArtifactUpdate(
                    TaskArtifactUpdateEvent {
                        task_id: context.task_id.clone(),
                        context_id: context.context_id.clone(),
                        artifact: Artifact {
                            artifact_id: "art-1".to_string(),
                            name: Some("echo".to_string()),
                            description: None,
                            parts: context.user_message.parts.clone(),
                            metadata: None,
                            extensions: vec![],
                        },
                        append: false,
                        last_chunk: true,
                        metadata: None,
                    },
                ))
                .await;
            let _ = tx
                .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                    task_id: context.task_id,
                    context_id: context.context_id,
                    status: TaskStatus {
                        state: TaskState::Completed,
                        message: None,
                        timestamp: Some(chrono::Utc::now().to_rfc3339()),
                    },
                    metadata: None,
                }))
                .await;
            Ok(())
        }

        async fn cancel(
            &self,
            _task_id: &str,
            _tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), A2AError> {
            Ok(())
        }
    }

    fn test_handler() -> Arc<DefaultRequestHandler<InMemoryTaskStore, EchoExecutor>> {
        let card = AgentCard {
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            ..Default::default()
        };
        Arc::new(DefaultRequestHandler::new(
            card,
            InMemoryTaskStore::new(),
            EchoExecutor,
        ))
    }

    #[tokio::test]
    async fn test_jsonrpc_send_message() {
        let rpc = JsonRpcHandler::new(test_handler());
        let raw = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "message/send",
            "params": {
                "message": {
                    "messageId": "msg-1",
                    "role": "user",
                    "parts": [{"type": "text", "text": "Hello!"}]
                }
            }
        });

        let response = rpc.handle_request(&raw).await;
        assert!(
            response.error.is_none(),
            "Expected no error, got: {:?}",
            response.error
        );
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_jsonrpc_method_not_found() {
        let rpc = JsonRpcHandler::new(test_handler());
        let raw = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unknown/method",
            "params": {}
        });

        let response = rpc.handle_request(&raw).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_jsonrpc_invalid_params() {
        let rpc = JsonRpcHandler::new(test_handler());
        let raw = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/get",
            "params": {}
        });

        let response = rpc.handle_request(&raw).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_jsonrpc_parse_error() {
        let rpc = JsonRpcHandler::new(test_handler());
        let raw = serde_json::json!("not a valid request");

        let response = rpc.handle_request(&raw).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn test_jsonrpc_get_agent_card() {
        let rpc = JsonRpcHandler::new(test_handler());
        let raw = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "agent/authenticatedExtendedCard",
            "params": {}
        });

        let response = rpc.handle_request(&raw).await;
        assert!(response.result.is_some());
    }
}
