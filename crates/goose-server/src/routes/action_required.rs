use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmToolActionRequest {
    id: String,
    #[serde(default = "default_principal_type")]
    principal_type: PrincipalType,
    action: Permission,
    session_id: String,
}

fn default_principal_type() -> PrincipalType {
    PrincipalType::Tool
}

/// Submit a tool execution decision (approve or deny)
///
/// Forwards the user's approve/deny decision for a tool that requires confirmation. When the agent wants to execute a tool that requires user approval, it emits an actionRequired event on the SSE stream. The UI should present the action to the user and send the user's decision back via this endpoint. Returns an empty JSON object as an acknowledgment — the actual tool execution result will appear on the SSE event stream.
#[utoipa::path(
    post,
    path = "/action-required/tool-confirmation",
    request_body = ConfirmToolActionRequest,
    responses(
        (status = 200, description = "Decision forwarded to the agent", body = Value),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn confirm_tool_action(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfirmToolActionRequest>,
) -> Result<Json<Value>, ErrorResponse> {
    let agent = state.get_agent_for_route(request.session_id).await?;

    agent
        .handle_confirmation(
            request.id.clone(),
            PermissionConfirmation {
                principal_type: request.principal_type,
                permission: request.action,
            },
        )
        .await;

    Ok(Json(Value::Object(serde_json::Map::new())))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/action-required/tool-confirmation",
            post(confirm_tool_action),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod integration_tests {
        use super::*;
        use axum::{body::Body, http::Request};
        use http::StatusCode;
        use tower::ServiceExt;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_tool_confirmation_endpoint() {
            let state = AppState::new(true).await.unwrap();

            let app = routes(state);

            let request = Request::builder()
                .uri("/action-required/tool-confirmation")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&ConfirmToolActionRequest {
                        id: "test-id".to_string(),
                        principal_type: PrincipalType::Tool,
                        action: Permission::AllowOnce,
                        session_id: "test-session".to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
