// Export route modules
pub mod agent;
pub mod audio;
pub mod config_management;
pub mod context;
pub mod extension;
pub mod health;
pub mod project;
pub mod recipe;
pub mod reply;
pub mod schedule;
pub mod session;
pub mod setup;
pub mod utils;
use std::sync::Arc;

use axum::Router;

// Function to configure all routes
pub fn configure(state: Arc<crate::state::AppState>) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(reply::routes(state.clone()))
        .merge(agent::routes(state.clone()))
        .merge(audio::routes(state.clone()))
        .merge(context::routes(state.clone()))
        .merge(extension::routes(state.clone()))
        .merge(config_management::routes(state.clone()))
        .merge(recipe::routes(state.clone()))
        .merge(session::routes(state.clone()))
        .merge(schedule::routes(state.clone()))
        .merge(project::routes(state.clone()))
        .merge(setup::routes(state.clone()))
}
