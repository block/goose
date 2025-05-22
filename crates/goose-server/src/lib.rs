use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

pub mod configuration;
pub mod error;
pub mod logging;
pub mod openapi;
pub mod routes;
pub mod scheduler;
pub mod state;

// Re-export commonly used items
pub use openapi::*;
pub use state::*;
