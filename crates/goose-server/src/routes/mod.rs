pub mod a2a;
pub mod acp_discovery;
pub mod acp_ide;
pub mod action_required;
pub mod agent;
pub mod agent_card;
pub mod agent_management;
pub mod analytics;
pub mod config_management;
pub mod dictation;
pub mod errors;
pub mod extension_routes;
pub mod mcp_app_proxy;
pub mod mcp_ui_proxy;
pub mod observatory;
pub mod pipeline;
pub mod prompts;
pub mod recipe;
pub mod recipe_utils;
pub mod registry;
pub mod reply;
pub mod runs;
pub mod schedule;
pub mod session;
pub mod setup;
pub mod status;
pub mod telemetry;
pub mod tunnel;
pub mod utils;

use std::sync::Arc;

use axum::Router;

pub fn configure(state: Arc<crate::state::AppState>, secret_key: String) -> Router {
    Router::new()
        .merge(a2a::routes(state.clone()))
        .merge(acp_discovery::routes(state.clone()))
        .merge(status::routes(state.clone()))
        .merge(reply::routes(state.clone()))
        .merge(action_required::routes(state.clone()))
        .merge(agent::routes(state.clone()))
        .merge(dictation::routes(state.clone()))
        .merge(config_management::routes(state.clone()))
        .merge(prompts::routes())
        .merge(registry::routes())
        .merge(agent_card::routes(state.clone()))
        .merge(agent_management::routes(state.clone()))
        .merge(recipe::routes(state.clone()))
        .merge(pipeline::routes(state.clone()))
        .merge(session::routes(state.clone()))
        .merge(schedule::routes(state.clone()))
        .merge(setup::routes(state.clone()))
        .merge(telemetry::routes(state.clone()))
        .merge(tunnel::routes(state.clone()))
        .merge(runs::routes(state.clone()))
        .merge(acp_ide::routes(state.clone()))
        .merge(analytics::routes(state.clone()))
        .merge(observatory::routes(state.clone()))
        .merge(extension_routes::routes(state.clone()))
        .merge(mcp_ui_proxy::routes(secret_key.clone()))
        .merge(mcp_app_proxy::routes(secret_key))
}
