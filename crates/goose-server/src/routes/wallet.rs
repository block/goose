use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use futures::Stream;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;
use crate::wallet::{
    CreateInvoiceRequest, ParseInvoiceRequest, PayInvoiceRequest, PayInvoiceResponse,
    PaymentApprovalResponse, PaymentReceivedEvent, WalletStatusResponse,
};

// -- Error response helper --

fn error_response(status: axum::http::StatusCode, msg: String) -> axum::response::Response {
    let body = serde_json::json!({ "error": msg });
    (status, Json(body)).into_response()
}

// -- SSE stream wrapper --

struct WalletEventStream {
    inner: BroadcastStream<PaymentReceivedEvent>,
}

impl Stream for WalletEventStream {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let json = serde_json::to_string(&event).unwrap_or_default();
                let sse = format!("data: {json}\n\n");
                Poll::Ready(Some(Ok(Bytes::from(sse))))
            }
            Poll::Ready(Some(Err(_lagged))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl IntoResponse for WalletEventStream {
    fn into_response(self) -> axum::response::Response {
        let body = axum::body::Body::from_stream(self);
        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }
}

// -- Handlers --

#[utoipa::path(
    get,
    path = "/wallet/status",
    responses(
        (status = 200, description = "Wallet status", body = WalletStatusResponse),
    )
)]
async fn wallet_status(State(state): State<Arc<AppState>>) -> Json<WalletStatusResponse> {
    let wallet_state = state.wallet_manager.get_state().await;
    Json(WalletStatusResponse {
        state: wallet_state,
    })
}

#[utoipa::path(
    get,
    path = "/wallet/balance",
    responses(
        (status = 200, description = "Wallet balance", body = WalletBalance),
        (status = 500, description = "Wallet error"),
    )
)]
async fn wallet_balance(State(state): State<Arc<AppState>>) -> axum::response::Response {
    match state.wallet_manager.get_balance().await {
        Ok(balance) => Json(balance).into_response(),
        Err(e) => {
            tracing::error!("Failed to get wallet balance: {e:#}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{e:#}"),
            )
        }
    }
}

#[utoipa::path(
    post,
    path = "/wallet/invoice",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 200, description = "Created invoice", body = Invoice),
        (status = 500, description = "Invoice creation failed"),
    )
)]
async fn wallet_create_invoice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateInvoiceRequest>,
) -> axum::response::Response {
    match state.wallet_manager.create_invoice(req.amount_sats).await {
        Ok(invoice) => Json(invoice).into_response(),
        Err(e) => {
            tracing::error!("Failed to create invoice: {e:#}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{e:#}"),
            )
        }
    }
}

#[utoipa::path(
    post,
    path = "/wallet/parse-invoice",
    request_body = ParseInvoiceRequest,
    responses(
        (status = 200, description = "Parsed invoice details", body = ParsedInvoice),
        (status = 500, description = "Failed to parse invoice"),
    )
)]
async fn wallet_parse_invoice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ParseInvoiceRequest>,
) -> axum::response::Response {
    match state.wallet_manager.parse_invoice(&req.bolt11).await {
        Ok(parsed) => Json(parsed).into_response(),
        Err(e) => {
            tracing::error!("Failed to parse invoice: {e:#}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{e:#}"),
            )
        }
    }
}

#[utoipa::path(
    post,
    path = "/wallet/pay",
    request_body = PayInvoiceRequest,
    responses(
        (status = 200, description = "Payment initiated", body = PayInvoiceResponse),
        (status = 500, description = "Payment failed"),
    )
)]
async fn wallet_pay(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PayInvoiceRequest>,
) -> axum::response::Response {
    match state.wallet_manager.pay_invoice(&req.bolt11, req.amount_sats).await {
        Ok((amount_sats, preimage)) => Json(PayInvoiceResponse {
            success: true,
            amount_sats,
            preimage,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to pay invoice: {e:#}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{e:#}"),
            )
        }
    }
}

#[utoipa::path(
    get,
    path = "/wallet/events",
    responses(
        (status = 200, description = "SSE stream of payment events"),
    )
)]
async fn wallet_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.wallet_manager.subscribe_payments();
    WalletEventStream {
        inner: BroadcastStream::new(rx),
    }
}

// -- History handler --

#[utoipa::path(
    get,
    path = "/wallet/history",
    responses(
        (status = 200, description = "Payment history", body = Vec<PaymentRecord>),
        (status = 500, description = "Failed to fetch history"),
    )
)]
async fn wallet_history(State(state): State<Arc<AppState>>) -> axum::response::Response {
    match state.wallet_manager.get_history().await {
        Ok(history) => {
            tracing::debug!(count = history.len(), "Returning payment history");
            Json(history).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get payment history: {e:#}");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("{e:#}"),
            )
        }
    }
}

// -- Approval handlers --

#[utoipa::path(
    post,
    path = "/wallet/approve-payment",
    request_body = PaymentApprovalResponse,
    responses(
        (status = 200, description = "Approval response accepted"),
        (status = 400, description = "Invalid or expired approval request"),
    )
)]
async fn wallet_approve_payment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PaymentApprovalResponse>,
) -> axum::response::Response {
    match state.payment_approval.respond(&req.id, req.approved).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::warn!("Approval response failed: {e:#}");
            error_response(axum::http::StatusCode::BAD_REQUEST, format!("{e:#}"))
        }
    }
}

#[utoipa::path(
    get,
    path = "/wallet/pending-approvals",
    responses(
        (status = 200, description = "List of pending payment approval requests", body = Vec<PaymentApprovalRequest>),
    )
)]
async fn wallet_pending_approvals(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pending = state.payment_approval.get_pending().await;
    Json(pending)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/wallet/status", get(wallet_status))
        .route("/wallet/balance", get(wallet_balance))
        .route("/wallet/invoice", post(wallet_create_invoice))
        .route("/wallet/parse-invoice", post(wallet_parse_invoice))
        .route("/wallet/pay", post(wallet_pay))
        .route("/wallet/events", get(wallet_events))
        .route("/wallet/history", get(wallet_history))
        .route("/wallet/approve-payment", post(wallet_approve_payment))
        .route("/wallet/pending-approvals", get(wallet_pending_approvals))
        .with_state(state)
}
