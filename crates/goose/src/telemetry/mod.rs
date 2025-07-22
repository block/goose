mod config;
mod datadog_metrics;
mod environment;
mod events;
mod macros;
mod manager;
mod providers;
mod user;
mod utils;

pub use {
    config::{TelemetryConfig, TelemetryProvider, UsageType},
    environment::detect_environment,
    events::{
        CommandExecution, CommandResult, CommandType, ErrorDetails, RecipeExecution,
        SessionExecution, SessionMetadataSupport, SessionResult, SessionType, TelemetryEvent,
        TelemetryExecution, TokenUsage, ToolUsage,
    },
    manager::{
        global_telemetry, init_global_telemetry, shutdown_global_telemetry, RecipeExecutionBuilder,
        TelemetryManager,
    },
    user::{detect_usage_type, UserIdentity},
    utils::extract_tool_usage_from_messages,
};
