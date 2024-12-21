// Export route modules
pub mod reply;
pub mod transcribe;

use axum::Router;

// Function to configure all routes
pub fn configure(state: crate::state::AppState) -> Router {
    Router::new()
        .merge(reply::routes(state.clone()))
        .merge(transcribe::routes())
}
