//! A2A error types with JSON-RPC error codes per spec section 5.4 / 9.5.

use crate::jsonrpc::JsonRpcError;

/// A2A protocol errors with associated JSON-RPC error codes.
///
/// Standard JSON-RPC codes: -32700, -32600..-32603
/// A2A-specific codes: -32001..-32009 per spec section 5.4.
#[derive(Debug, Clone, thiserror::Error)]
pub enum A2AError {
    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Method not found: {method}")]
    MethodNotFound { method: String },

    #[error("Invalid params: {message}")]
    InvalidParams { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },

    // A2A-specific errors (-32001 .. -32009)
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: String },

    #[error("Task not cancelable: {task_id}")]
    TaskNotCancelable { task_id: String },

    #[error("Push notification not supported")]
    PushNotificationNotSupported,

    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("Content type not supported: {content_type}")]
    ContentTypeNotSupported { content_type: String },

    #[error("Invalid agent response: {message}")]
    InvalidAgentResponse { message: String },

    #[error("Extended agent card not configured")]
    AuthenticatedExtendedCardNotConfigured,

    #[error("Extension support required: {uri}")]
    ExtensionSupportRequired { uri: String },

    #[error("Version not supported: {version}")]
    VersionNotSupported { version: String },
}

impl A2AError {
    pub fn code(&self) -> i32 {
        match self {
            Self::ParseError { .. } => -32700,
            Self::InvalidRequest { .. } => -32600,
            Self::MethodNotFound { .. } => -32601,
            Self::InvalidParams { .. } => -32602,
            Self::InternalError { .. } => -32603,
            Self::TaskNotFound { .. } => -32001,
            Self::TaskNotCancelable { .. } => -32002,
            Self::PushNotificationNotSupported => -32003,
            Self::UnsupportedOperation { .. } => -32004,
            Self::ContentTypeNotSupported { .. } => -32005,
            Self::InvalidAgentResponse { .. } => -32006,
            Self::AuthenticatedExtendedCardNotConfigured => -32007,
            Self::ExtensionSupportRequired { .. } => -32008,
            Self::VersionNotSupported { .. } => -32009,
        }
    }

    pub fn to_jsonrpc_error(&self) -> JsonRpcError {
        JsonRpcError {
            code: self.code(),
            message: self.to_string(),
            data: None,
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::MethodNotFound {
            method: method.into(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams {
            message: message.into(),
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    pub fn task_not_found(task_id: impl Into<String>) -> Self {
        Self::TaskNotFound {
            task_id: task_id.into(),
        }
    }

    pub fn task_not_cancelable(task_id: impl Into<String>) -> Self {
        Self::TaskNotCancelable {
            task_id: task_id.into(),
        }
    }

    pub fn unsupported_operation(operation: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
        }
    }

    pub fn content_type_not_supported(content_type: impl Into<String>) -> Self {
        Self::ContentTypeNotSupported {
            content_type: content_type.into(),
        }
    }

    pub fn invalid_agent_response(message: impl Into<String>) -> Self {
        Self::InvalidAgentResponse {
            message: message.into(),
        }
    }

    pub fn extension_support_required(uri: impl Into<String>) -> Self {
        Self::ExtensionSupportRequired { uri: uri.into() }
    }

    pub fn version_not_supported(version: impl Into<String>) -> Self {
        Self::VersionNotSupported {
            version: version.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(A2AError::parse_error("bad json").code(), -32700);
        assert_eq!(A2AError::invalid_request("missing field").code(), -32600);
        assert_eq!(A2AError::method_not_found("foo/bar").code(), -32601);
        assert_eq!(A2AError::invalid_params("bad type").code(), -32602);
        assert_eq!(A2AError::internal_error("oops").code(), -32603);
        assert_eq!(A2AError::task_not_found("t1").code(), -32001);
        assert_eq!(A2AError::task_not_cancelable("t1").code(), -32002);
        assert_eq!(A2AError::PushNotificationNotSupported.code(), -32003);
        assert_eq!(A2AError::unsupported_operation("list").code(), -32004);
        assert_eq!(
            A2AError::content_type_not_supported("text/xml").code(),
            -32005
        );
        assert_eq!(
            A2AError::invalid_agent_response("bad format").code(),
            -32006
        );
        assert_eq!(
            A2AError::AuthenticatedExtendedCardNotConfigured.code(),
            -32007
        );
        assert_eq!(
            A2AError::extension_support_required("https://example.com/ext").code(),
            -32008
        );
        assert_eq!(A2AError::version_not_supported("0.1").code(), -32009);
    }

    #[test]
    fn test_error_codes_complete_range() {
        // Verify all A2A-specific error codes -32001 through -32009 are covered
        let errors: Vec<Box<dyn Fn() -> A2AError>> = vec![
            Box::new(|| A2AError::task_not_found("t")),
            Box::new(|| A2AError::task_not_cancelable("t")),
            Box::new(|| A2AError::PushNotificationNotSupported),
            Box::new(|| A2AError::unsupported_operation("op")),
            Box::new(|| A2AError::content_type_not_supported("ct")),
            Box::new(|| A2AError::invalid_agent_response("msg")),
            Box::new(|| A2AError::AuthenticatedExtendedCardNotConfigured),
            Box::new(|| A2AError::extension_support_required("uri")),
            Box::new(|| A2AError::version_not_supported("v")),
        ];
        let codes: Vec<i32> = errors.iter().map(|f| f().code()).collect();
        assert_eq!(
            codes,
            vec![-32001, -32002, -32003, -32004, -32005, -32006, -32007, -32008, -32009]
        );
    }

    #[test]
    fn test_to_jsonrpc_error() {
        let err = A2AError::task_not_found("task-123");
        let rpc_err = err.to_jsonrpc_error();
        assert_eq!(rpc_err.code, -32001);
        assert!(rpc_err.message.contains("task-123"));
        assert!(rpc_err.data.is_none());
    }

    #[test]
    fn test_error_display() {
        let err = A2AError::internal_error("something went wrong");
        assert_eq!(err.to_string(), "Internal error: something went wrong");
    }
}
