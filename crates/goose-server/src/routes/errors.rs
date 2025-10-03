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

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "message": self.message,
        }));

        (self.status, body).into_response()
    }
}
