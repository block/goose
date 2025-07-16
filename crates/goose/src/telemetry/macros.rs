#[macro_export]
macro_rules! track_telemetry {
    (session: ($session_id:expr, $session_type:expr) => $body:block) => {{
        use goose::telemetry::{TelemetryExecution, SessionMetadataSupport};

        let start_time = std::time::Instant::now();

        let telemetry_execution = goose::telemetry::global_telemetry().map(|_manager| {
            goose::telemetry::SessionExecution::new($session_id, $session_type)
                .with_metadata("interface", "cli")
        });

        if goose::telemetry::global_telemetry().is_none() {
            tracing::debug!("Telemetry is disabled or not initialized for session tracking");
        }

        let result = async move $body.await;

        if let Some(mut execution) = telemetry_execution {
            let duration = start_time.elapsed();

            match &result {
                Ok((_, session)) => {
                    if let Ok(session_metadata) = session.get_metadata() {
                        execution = execution.with_session_metadata(&session_metadata);
                    }

                    let tool_usage = $crate::cli::telemetry::extract_tool_usage_from_session(session);
                    for tool in tool_usage {
                        execution.add_tool_usage(tool);
                    }

                    if let Some(env) = goose::telemetry::detect_environment() {
                        execution = execution.with_environment(&env);
                    }

                    let messages = session.message_history();
                    execution = execution
                        .with_turn_count(messages.len() as u64)
                        .with_result(goose::telemetry::SessionResult::Success)
                        .with_duration(duration);
                }
                Err(e) => {
                    execution = execution
                        .with_result(goose::telemetry::SessionResult::Error(e.to_string()))
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
    }};

    (command: ($command_name:expr, $command_type:expr) => $body:block) => {{
        use goose::telemetry::TelemetryExecution;

        let start_time = std::time::Instant::now();

        let telemetry_execution = goose::telemetry::global_telemetry()
            .map(|_manager| goose::telemetry::CommandExecution::new($command_name, $command_type));

        if goose::telemetry::global_telemetry().is_none() {
            tracing::debug!("Telemetry is disabled or not initialized for command tracking");
        }

        let result = async move $body.await;

        if let Some(mut execution) = telemetry_execution {
            let duration = start_time.elapsed();

            match &result {
                Ok(_) => {
                    execution = execution
                        .with_result(goose::telemetry::CommandResult::Success)
                        .with_duration(duration);
                }
                Err(e) => {
                    execution = execution
                        .with_result(goose::telemetry::CommandResult::Error(e.to_string()))
                        .with_duration(duration);
                }
            }

            if let Some(env) = goose::telemetry::detect_environment() {
                execution = execution.with_environment(&env);
            }

            if let Some(manager) = goose::telemetry::global_telemetry() {
                if let Err(e) = manager.track_command_execution(execution).await {
                    tracing::warn!("Failed to track command execution: {}", e);
                }
            }
        }

        result
    }};

    (recipe: ($recipe_name:expr, $recipe_version:expr, $params:expr) => $body:block) => {{
        use goose::telemetry::SessionMetadataSupport;

        let start_time = std::time::Instant::now();

        let telemetry_execution = goose::telemetry::global_telemetry()
            .map(|manager| manager.recipe_execution($recipe_name, $recipe_version));

        if goose::telemetry::global_telemetry().is_none() {
            tracing::debug!("Telemetry is disabled or not initialized for recipe tracking");
        }

        let result = async move $body.await;

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

                    let tool_usage = $crate::cli::telemetry::extract_tool_usage_from_session(session);
                    for tool in tool_usage {
                        builder = builder.add_tool_usage(tool);
                    }

                    for (key, value) in &$params {
                        builder = builder.with_metadata(key, value);
                    }

                    if let Some(env) = goose::telemetry::detect_environment() {
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

                    for (key, value) in &$params {
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
    }};
}
