use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;
pub mod commands;
pub mod logging;
pub mod session;

// Re-export commonly used types
pub use session::Session;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});
