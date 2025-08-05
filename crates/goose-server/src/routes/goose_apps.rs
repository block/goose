use super::utils::verify_secret_key;
use std::sync::Arc;

use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GooseApp {
    pub name: String,
    pub description: Option<String>,
    pub js_implementation: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppListResponse {
    /// List of installed Goose apps
    pub apps: Vec<GooseApp>,
}

#[utoipa::path(
    get,
    path = "/apps",
    responses(
       (status = 200, description = "List of installed apps retrieved successfully", body = AppListResponse),
       (status = 401, description = "Unauthorized - Invalid or missing API key"),
       (status = 500, description = "Internal server error")
    ),
    security(
       ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn list_apps(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AppListResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let clock_app = GooseApp {
        name: "Clock".to_string(),
        description: Some("Digital clock".to_string()),
        js_implementation: r#"
class ClockWidget extends GooseWidget {
   getName() {
       return 'Clock';
   }

   render() {
       return `<div style="text-align: center; font-family: monospace; font-size: 2rem; padding: 20px;">
           ${new Date().toLocaleTimeString()}
       </div>`;
   }

   onMount() {
       setInterval(() => this.api.update(), 1000);
   }
}
"#.to_string(),
    };

    Ok(Json(AppListResponse {
        apps: vec![clock_app],
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/list_apps", get(list_apps))
        .with_state(state)
}
