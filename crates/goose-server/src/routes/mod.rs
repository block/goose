// Export route modules
pub mod agent;
pub mod health;
pub mod reply;
pub mod system;

use axum::Router;

// Function to configure all routes
pub fn configure(state: crate::state::AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(reply::routes(state.clone()))
        .merge(agent::routes(state.clone()))
        .merge(system::routes(state))
}
