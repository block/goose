use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use goose::config::ConfigError;
use goose::model::ConfigError as ModelConfigError;
use goose::providers::errors::ProviderError;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ErrorResponse {
    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub(crate) fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "message": self.message,
        }));

        (self.status, body).into_response()
    }
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(err: anyhow::Error) -> Self {
        Self::internal(err.to_string())
    }
}

impl From<ConfigError> for ErrorResponse {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::NotFound(key) => Self::not_found(format!("Config key not found: {}", key)),
            _ => Self::internal(err.to_string()),
        }
    }
}

impl From<ModelConfigError> for ErrorResponse {
    fn from(err: ModelConfigError) -> Self {
        Self::internal(format!("Model configuration error: {}", err))
    }
}

impl From<std::io::Error> for ErrorResponse {
    fn from(err: std::io::Error) -> Self {
        Self::internal(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for ErrorResponse {
    fn from(err: serde_json::Error) -> Self {
        Self::internal(format!("JSON serialization error: {}", err))
    }
}

impl From<serde_yaml::Error> for ErrorResponse {
    fn from(err: serde_yaml::Error) -> Self {
        Self::unprocessable(format!("YAML parsing error: {}", err))
    }
}

impl From<ProviderError> for ErrorResponse {
    fn from(err: ProviderError) -> Self {
        let (status, message) = match err {
            ProviderError::Authentication(_) => (
                StatusCode::BAD_REQUEST,
                format!("Authentication failed: {}", err),
            ),
            ProviderError::UsageError(_) => (
                StatusCode::BAD_REQUEST,
                format!("Usage error: {}", err),
            ),
            ProviderError::RateLimitExceeded { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded: {}", err),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Provider error: {}", err),
            ),
        };

        Self {
            message,
            status,
        }
    }
}
