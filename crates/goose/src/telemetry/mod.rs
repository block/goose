mod config;
mod events;
mod manager;
mod providers;
mod user;

pub use {
    config::{TelemetryConfig, TelemetryProvider, UsageType},
    events::{RecipeExecution, RecipeResult, TelemetryEvent},
    manager::TelemetryManager,
    user::{detect_usage_type, UserIdentity},
};
