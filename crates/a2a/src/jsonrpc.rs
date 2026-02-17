//! JSON-RPC 2.0 envelope types and A2A method constants per spec section 9.4.

use serde::{Deserialize, Serialize};

pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 success/error response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(
        method: impl Into<String>,
        id: impl Into<serde_json::Value>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            id: id.into(),
            params,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// A2A JSON-RPC method name constants per spec section 9.4.
pub mod methods {
    pub const SEND_MESSAGE: &str = "message/send";
    pub const SEND_STREAM: &str = "message/stream";
    pub const GET_TASK: &str = "tasks/get";
    pub const LIST_TASKS: &str = "tasks/list";
    pub const CANCEL_TASK: &str = "tasks/cancel";
    pub const SUBSCRIBE_TASK: &str = "tasks/resubscribe";
    pub const SET_PUSH_CONFIG: &str = "tasks/pushNotificationConfig/set";
    pub const GET_PUSH_CONFIG: &str = "tasks/pushNotificationConfig/get";
    pub const LIST_PUSH_CONFIG: &str = "tasks/pushNotificationConfig/list";
    pub const DELETE_PUSH_CONFIG: &str = "tasks/pushNotificationConfig/delete";
    pub const GET_EXTENDED_CARD: &str = "agent/authenticatedExtendedCard";
}

/// A2A HTTP header constants per spec section 4.7.
pub mod headers {
    pub const A2A_VERSION: &str = "A2A-Version";
    pub const A2A_EXTENSIONS: &str = "A2A-Extensions";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serde() {
        let req = JsonRpcRequest::new(
            methods::SEND_MESSAGE,
            serde_json::json!(1),
            Some(serde_json::json!({"message": {"role": "user", "parts": []}})),
        );
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "message/send");
        assert_eq!(json["id"], 1);
    }

    #[test]
    fn test_success_response() {
        let resp =
            JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"id": "task-1"}));
        assert!(!resp.is_error());
        assert_eq!(resp.result.unwrap()["id"], "task-1");
    }

    #[test]
    fn test_error_response() {
        let resp = JsonRpcResponse::error(
            serde_json::json!(1),
            JsonRpcError {
                code: -32001,
                message: "Task not found".to_string(),
                data: None,
            },
        );
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, -32001);
    }
}
