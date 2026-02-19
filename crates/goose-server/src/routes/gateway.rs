use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use goose::gateway::manager::GatewayStatus;
use goose::gateway::GatewayConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct StartGatewayRequest {
    pub gateway_type: String,
    pub platform_config: serde_json::Value,
    #[serde(default)]
    pub max_sessions: usize,
}

#[derive(Deserialize, ToSchema)]
pub struct StopGatewayRequest {
    pub gateway_type: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RestartGatewayRequest {
    pub gateway_type: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RemoveGatewayRequest {
    pub gateway_type: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePairingRequest {
    pub gateway_type: String,
}

#[derive(Serialize, ToSchema)]
pub struct PairingCodeResponse {
    pub code: String,
    pub expires_at: i64,
}

#[utoipa::path(
    post,
    path = "/gateway/start",
    request_body = StartGatewayRequest,
    responses(
        (status = 200, description = "Gateway started"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn start_gateway(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StartGatewayRequest>,
) -> Response {
    let mut config = GatewayConfig {
        gateway_type: request.gateway_type,
        platform_config: request.platform_config,
        max_sessions: request.max_sessions,
    };

    let gw = match goose::gateway::create_gateway(&mut config) {
        Ok(gw) => gw,
        Err(e) => return ErrorResponse::bad_request(e.to_string()).into_response(),
    };

    match state.gateway_manager.start_gateway(config, gw).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ErrorResponse::bad_request(e.to_string()).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/gateway/stop",
    request_body = StopGatewayRequest,
    responses(
        (status = 200, description = "Gateway stopped"),
        (status = 404, description = "Gateway not found", body = ErrorResponse)
    )
)]
pub async fn stop_gateway(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StopGatewayRequest>,
) -> Response {
    match state
        .gateway_manager
        .stop_gateway(&request.gateway_type)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ErrorResponse::not_found(e.to_string()).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/gateway/restart",
    request_body = RestartGatewayRequest,
    responses(
        (status = 200, description = "Gateway restarted"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "No saved config", body = ErrorResponse)
    )
)]
pub async fn restart_gateway(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RestartGatewayRequest>,
) -> Response {
    match state
        .gateway_manager
        .restart_gateway(&request.gateway_type)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ErrorResponse::bad_request(e.to_string()).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/gateway/remove",
    request_body = RemoveGatewayRequest,
    responses(
        (status = 200, description = "Gateway removed"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn remove_gateway(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RemoveGatewayRequest>,
) -> Response {
    match state
        .gateway_manager
        .remove_gateway(&request.gateway_type)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ErrorResponse::internal(e.to_string()).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/gateway/status",
    responses(
        (status = 200, description = "Gateway statuses", body = Vec<GatewayStatus>)
    )
)]
pub async fn gateway_status(State(state): State<Arc<AppState>>) -> Json<Vec<GatewayStatus>> {
    Json(state.gateway_manager.status().await)
}

#[utoipa::path(
    post,
    path = "/gateway/pair",
    request_body = CreatePairingRequest,
    responses(
        (status = 200, description = "Pairing code generated", body = PairingCodeResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_pairing_code(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePairingRequest>,
) -> Response {
    match state
        .gateway_manager
        .generate_pairing_code(&request.gateway_type)
        .await
    {
        Ok((code, expires_at)) => (
            StatusCode::OK,
            Json(PairingCodeResponse { code, expires_at }),
        )
            .into_response(),
        Err(e) => ErrorResponse::internal(e.to_string()).into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/gateway/pair/{platform}/{user_id}",
    params(
        ("platform" = String, Path, description = "Platform name"),
        ("user_id" = String, Path, description = "Platform user ID")
    ),
    responses(
        (status = 200, description = "User unpaired"),
        (status = 404, description = "Pairing not found", body = ErrorResponse)
    )
)]
pub async fn unpair_user(
    State(state): State<Arc<AppState>>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Response {
    match state.gateway_manager.unpair_user(&platform, &user_id).await {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => {
            ErrorResponse::not_found(format!("No pairing found for {}/{}", platform, user_id))
                .into_response()
        }
        Err(e) => ErrorResponse::internal(e.to_string()).into_response(),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/gateway/start", post(start_gateway))
        .route("/gateway/stop", post(stop_gateway))
        .route("/gateway/restart", post(restart_gateway))
        .route("/gateway/remove", post(remove_gateway))
        .route("/gateway/status", get(gateway_status))
        .route("/gateway/pair", post(create_pairing_code))
        .route("/gateway/pair/{platform}/{user_id}", delete(unpair_user))
        .with_state(state)
}
