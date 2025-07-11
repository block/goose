mod config;
mod datadog_metrics;
mod events;
mod manager;
mod providers;
mod user;

pub use {
    config::{TelemetryConfig, TelemetryProvider, UsageType},
    events::{
        CommandExecution, CommandResult, CommandType, ErrorDetails, RecipeExecution, RecipeResult,
        SessionExecution, SessionResult, SessionType, TelemetryEvent, TokenUsage, ToolUsage,
    },
    manager::{
        global_telemetry, init_global_telemetry, shutdown_global_telemetry, RecipeExecutionBuilder,
        TelemetryManager,
    },
    user::{detect_usage_type, UserIdentity},
};
