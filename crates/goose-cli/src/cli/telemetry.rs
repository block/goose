use anyhow::Result;
use std::collections::HashMap;

use goose::message::MessageContent;
use goose::telemetry::{
    detect_environment, CommandExecution, CommandResult, CommandType, SessionExecution,
    SessionResult, SessionType, ToolUsage, TelemetryExecution, SessionMetadataSupport,
};

pub fn log_if_telemetry_disabled(tracking_type: &str) {
    if goose::telemetry::global_telemetry().is_none() {
        tracing::debug!(
            "Telemetry is disabled or not initialized for {} tracking",
            tracking_type
        );
    }
}

pub fn extract_tool_usage_from_session(session: &crate::Session) -> Vec<ToolUsage> {
    let messages = session.message_history();
    let mut tool_usage_map: HashMap<String, ToolUsage> = HashMap::new();
    let mut tool_call_times: HashMap<String, i64> = HashMap::new();
    let mut tool_id_to_name: HashMap<String, String> = HashMap::new();

    for message in &messages {
        for content in &message.content {
            match content {
                MessageContent::ToolRequest(tool_request) => {
                    if let Ok(tool_call) = &tool_request.tool_call {
                        let tool_name = &tool_call.name;
                        let tool_id = &tool_request.id;

                        tool_id_to_name.insert(tool_id.clone(), tool_name.clone());
                        tool_call_times.insert(tool_id.clone(), message.created);

                        tool_usage_map
                            .entry(tool_name.clone())
                            .or_insert_with(|| ToolUsage::new(tool_name));
                    }
                }
                MessageContent::ToolResponse(tool_response) => {
                    let tool_id = &tool_response.id;

                    if let Some(tool_name) = tool_id_to_name.get(tool_id) {
                        if let Some(entry) = tool_usage_map.get_mut(tool_name) {
                            let duration = if let Some(start_time) = tool_call_times.get(tool_id) {
                                let duration_ms = (message.created - start_time).max(0) as u64;
                                std::time::Duration::from_millis(duration_ms)
                            } else {
                                std::time::Duration::from_millis(0)
                            };

                            let success = tool_response.tool_result.is_ok();
                            entry.add_call(duration, success);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    tool_usage_map.into_values().collect()
}

pub async fn track_session_execution<F, Fut, T>(
    session_id: &str,
    session_type: SessionType,
    execution_fn: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(T, crate::Session)>>,
{
    let start_time = std::time::Instant::now();

    let telemetry_execution = goose::telemetry::global_telemetry().map(|_manager| {
        SessionExecution::new(session_id, session_type).with_metadata("interface", "cli")
    });

    log_if_telemetry_disabled("session");

    let result = execution_fn().await;

    if let Some(mut execution) = telemetry_execution {
        let duration = start_time.elapsed();

        match &result {
            Ok((_, session)) => {
                if let Ok(session_metadata) = session.get_metadata() {
                    execution = execution.with_session_metadata(&session_metadata);
                }

                let tool_usage = extract_tool_usage_from_session(session);
                for tool in tool_usage {
                    execution.add_tool_usage(tool);
                }

                if let Some(env) = detect_environment() {
                    execution = execution.with_environment(&env);
                }

                let messages = session.message_history();
                execution = execution
                    .with_turn_count(messages.len() as u64)
                    .with_result(SessionResult::Success)
                    .with_duration(duration);
            }
            Err(e) => {
                execution = execution
                    .with_result(SessionResult::Error(e.to_string()))
                    .with_duration(duration);
            }
        }

        if let Some(manager) = goose::telemetry::global_telemetry() {
            if let Err(e) = manager.track_session_execution(execution).await {
                tracing::warn!("Failed to track session execution: {}", e);
            }
        }
    }

    result.map(|(result, _)| result)
}

pub async fn track_command_execution<F, Fut, T>(
    command_name: &str,
    command_type: CommandType,
    execution_fn: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let start_time = std::time::Instant::now();

    let telemetry_execution = goose::telemetry::global_telemetry()
        .map(|_manager| CommandExecution::new(command_name, command_type));

    log_if_telemetry_disabled("command");

    let result = execution_fn().await;

    if let Some(mut execution) = telemetry_execution {
        let duration = start_time.elapsed();

        match &result {
            Ok(_) => {
                execution = execution
                    .with_result(CommandResult::Success)
                    .with_duration(duration);
            }
            Err(e) => {
                execution = execution
                    .with_result(CommandResult::Error(e.to_string()))
                    .with_duration(duration);
            }
        }

        if let Some(env) = detect_environment() {
            execution = execution.with_environment(&env);
        }

        if let Some(manager) = goose::telemetry::global_telemetry() {
            if let Err(e) = manager.track_command_execution(execution).await {
                tracing::warn!("Failed to track command execution: {}", e);
            }
        }
    }

    result
}

pub async fn track_recipe_execution<F, Fut, T>(
    recipe_name: &str,
    recipe_version: &str,
    execution_fn: F,
    params: Vec<(String, String)>,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(T, crate::Session)>>,
{
    let start_time = std::time::Instant::now();

    let telemetry_execution = goose::telemetry::global_telemetry()
        .map(|manager| manager.recipe_execution(recipe_name, recipe_version));

    log_if_telemetry_disabled("recipe");

    let result = execution_fn().await;

    if let Some(execution_builder) = telemetry_execution {
        let duration = start_time.elapsed();
        let execution = match &result {
            Ok((_, session)) => {
                let mut builder = execution_builder
                    .with_result(goose::telemetry::SessionResult::Success)
                    .with_duration(duration);

                if let Ok(session_metadata) = session.get_metadata() {
                    builder = builder.with_session_metadata(&session_metadata);
                }

                let tool_usage = extract_tool_usage_from_session(session);
                for tool in tool_usage {
                    builder = builder.add_tool_usage(tool);
                }

                for (key, value) in &params {
                    builder = builder.with_metadata(key, value);
                }

                if let Some(env) = detect_environment() {
                    builder = builder.with_environment(&env);
                }

                let messages = session.message_history();
                builder = builder.with_turn_count(messages.len() as u64);

                builder.build()
            }
            Err(e) => {
                let mut builder = execution_builder
                    .with_result(goose::telemetry::SessionResult::Error(e.to_string()))
                    .with_duration(duration);

                for (key, value) in &params {
                    builder = builder.with_metadata(key, value);
                }

                builder.build()
            }
        };

        if let Some(manager) = goose::telemetry::global_telemetry() {
            if let Err(e) = manager.track_recipe_execution(execution).await {
                tracing::warn!("Failed to track recipe execution: {}", e);
            }
        }
    }

    result.map(|(result, _)| result)
}
