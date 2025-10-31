use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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
