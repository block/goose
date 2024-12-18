use thiserror::Error;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<RouterError> for mcp_core::protocol::ErrorData {
    fn from(err: RouterError) -> Self {
        use mcp_core::protocol::*;
        match err {
            RouterError::MethodNotFound(_) => ErrorData {
                code: METHOD_NOT_FOUND,
                message: err.to_string(),
                data: None,
            },
            RouterError::UnknownTool(_) => ErrorData {
                code: INVALID_REQUEST,
                message: err.to_string(),
                data: None,
            },
            RouterError::InvalidParams(_) => ErrorData {
                code: INVALID_PARAMS,
                message: err.to_string(),
                data: None,
            },
            RouterError::Internal(_) => ErrorData {
                code: INTERNAL_ERROR,
                message: err.to_string(),
                data: None,
            },
        }
    }
}