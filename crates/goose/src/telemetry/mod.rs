mod config;
mod events;
mod manager;
mod providers;
mod user;

pub use {
    config::{TelemetryConfig, TelemetryProvider, UsageType},
    events::{RecipeExecution, RecipeResult, TelemetryEvent, TokenUsage, ToolUsage},
    manager::{TelemetryManager, RecipeExecutionBuilder, init_global_telemetry, global_telemetry, shutdown_global_telemetry},
    user::{detect_usage_type, UserIdentity},
};
