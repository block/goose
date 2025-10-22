use axum::{
    extract::State,
    http,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures::Stream;
use goose::agents::approval::ApprovalState;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// Re-export for OpenAPI
pub use goose::agents::approval::{ApprovalRequest, ApprovalResponse};

pub struct SseResponse {
    rx: ReceiverStream<String>,
}

impl SseResponse {
    fn new(rx: ReceiverStream<String>) -> Self {
        Self { rx }
    }
}

// TODO(alexhancock) - Dedupe into chat stream
impl Stream for SseResponse {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx)
            .poll_next(cx)
            .map(|opt| opt.map(|s| Ok(Bytes::from(s))))
    }
}

impl IntoResponse for SseResponse {
    fn into_response(self) -> axum::response::Response {
        let stream = self;
        let body = axum::body::Body::from_stream(stream);

        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }
}

#[utoipa::path(
    get,
    path = "/approval",
    responses(
        (status = 200, description = "SSE stream of approval requests", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn approval_stream(State(state): State<Arc<ApprovalState>>) -> SseResponse {
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    state.add_client(tx).await;

    SseResponse::new(stream)
}

#[utoipa::path(
    post,
    path = "/approval",
    request_body = ApprovalResponse,
    responses(
        (status = 200, description = "Approval response submitted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn submit_approval_response(
    State(state): State<Arc<ApprovalState>>,
    Json(response): Json<ApprovalResponse>,
) -> Result<impl IntoResponse, (http::StatusCode, String)> {
    state
        .submit_response(response.request_id, response.action)
        .await
        .map_err(|e| (http::StatusCode::BAD_REQUEST, e))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn routes(state: Arc<ApprovalState>) -> Router {
    Router::new()
        .route("/approval", get(approval_stream))
        .route("/approval", post(submit_approval_response))
        .with_state(state)
}
