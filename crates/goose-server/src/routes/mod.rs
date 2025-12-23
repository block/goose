pub mod action_required;
pub mod agent;
pub mod audio;
pub mod config_management;
pub mod errors;
pub mod mcp_app_proxy;
pub mod mcp_ui_proxy;
pub mod recipe;
pub mod recipe_utils;
pub mod reply;
pub mod schedule;
pub mod session;
pub mod setup;
pub mod status;
pub mod telemetry;
pub mod tunnel;
pub mod utils;

use std::sync::Arc;

use axum::Router;

// Function to configure all routes
pub fn configure(state: Arc<crate::state::AppState>, secret_key: String) -> Router {
    Router::new()
        .merge(status::routes())
        .merge(reply::routes(state.clone()))
        .merge(action_required::routes(state.clone()))
        .merge(agent::routes(state.clone()))
        .merge(audio::routes(state.clone()))
        .merge(config_management::routes(state.clone()))
        .merge(recipe::routes(state.clone()))
        .merge(session::routes(state.clone()))
        .merge(schedule::routes(state.clone()))
        .merge(setup::routes(state.clone()))
        .merge(telemetry::routes(state.clone()))
        .merge(tunnel::routes(state.clone()))
        .merge(mcp_ui_proxy::routes(secret_key.clone()))
        .merge(mcp_app_proxy::routes(secret_key))
}
