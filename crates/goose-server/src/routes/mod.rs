// Export route modules
pub mod agent;
pub mod config_management;
pub mod configs;
pub mod extension;
pub mod health;
pub mod recipe;
pub mod reply;
pub mod session;
pub mod utils;
use axum::Router;
use axum::extract::DefaultBodyLimit;

// Function to configure all routes
pub fn configure(state: crate::state::AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(reply::routes(state.clone()).layer(DefaultBodyLimit::max(20 * 1024 * 1024)))
        .merge(agent::routes(state.clone()))
        .merge(extension::routes(state.clone()))
        .merge(configs::routes(state.clone()))
        .merge(config_management::routes(state.clone()))
        .merge(recipe::routes(state.clone()))
        .merge(session::routes(state))
}
