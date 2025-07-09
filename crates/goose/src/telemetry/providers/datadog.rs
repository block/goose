use crate::telemetry::{
    config::TelemetryConfig,
    events::{CommandExecution, RecipeExecution, SessionExecution, TelemetryEvent},
};

pub struct DatadogProvider {
    datadog_metrics: Option<crate::telemetry::datadog_metrics::DatadogMetricsExporter>,
    initialized: bool,
}

impl DatadogProvider {
    pub fn new() -> Self {
        Self {
            datadog_metrics: None,
            initialized: false,
        }
    }

    async fn send_datadog_metrics(
        datadog_exporter: &crate::telemetry::datadog_metrics::DatadogMetricsExporter,
        execution: &RecipeExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "Sending Datadog metrics for recipe: {}",
            execution.recipe_name
        );

        let base_tags = vec![
            format!("recipe_name:{}", execution.recipe_name),
            format!("recipe_version:{}", execution.recipe_version),
            format!("usage_type:{:?}", execution.usage_type),
            format!(
                "result:{:?}",
                execution
                    .result
                    .as_ref()
                    .unwrap_or(&crate::telemetry::events::RecipeResult::Success)
            ),
            "execution_type:recipe".to_string(),
        ];

        // Send recipe execution counter
        if let Err(e) = datadog_exporter
            .send_counter("goose.recipe.executions", 1, base_tags.clone())
            .await
        {
            tracing::error!("Failed to send goose.recipe.executions metric: {}", e);
            return Err(e);
        }
        tracing::info!("Successfully sent goose.recipe.executions metric");

        if let Some(duration_ms) = execution.duration_ms {
            if let Err(e) = datadog_exporter
                .send_histogram(
                    "goose.recipe.duration",
                    1,
                    duration_ms as f64 / 1000.0,
                    base_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.recipe.duration metric: {}", e);
                return Err(e);
            }
            tracing::info!("Successfully sent goose.recipe.duration metric");
        }

        if let Some(token_usage) = &execution.token_usage {
            let token_tags = [
                base_tags.clone(),
                vec![
                    format!(
                        "model:{}",
                        token_usage.model.as_ref().unwrap_or(&"unknown".to_string())
                    ),
                    format!(
                        "provider:{}",
                        token_usage
                            .provider
                            .as_ref()
                            .unwrap_or(&"unknown".to_string())
                    ),
                ],
            ]
            .concat();

            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.input",
                    token_usage.input_tokens,
                    token_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.input metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.output",
                    token_usage.output_tokens,
                    token_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.output metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.total",
                    token_usage.input_tokens + token_usage.output_tokens,
                    token_tags,
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.total metric: {}", e);
                return Err(e);
            }
            tracing::info!("Successfully sent token usage metrics");
        }

        for tool_usage in &execution.tool_usage {
            let tool_tags = [
                base_tags.clone(),
                vec![format!("tool_name:{}", tool_usage.tool_name)],
            ]
            .concat();

            if tool_usage.success_count > 0 {
                let success_tags = [tool_tags.clone(), vec!["result:success".to_string()]].concat();
                if let Err(e) = datadog_exporter
                    .send_counter("goose.tool.calls", tool_usage.success_count, success_tags)
                    .await
                {
                    tracing::error!("Failed to send goose.tool.calls success metric: {}", e);
                    return Err(e);
                }
            }

            if tool_usage.error_count > 0 {
                let error_tags = [tool_tags.clone(), vec!["result:error".to_string()]].concat();
                if let Err(e) = datadog_exporter
                    .send_counter("goose.tool.calls", tool_usage.error_count, error_tags)
                    .await
                {
                    tracing::error!("Failed to send goose.tool.calls error metric: {}", e);
                    return Err(e);
                }
            }

            // Send tool duration as histogram (count + sum) instead of just average
            if tool_usage.avg_duration_ms > 0 {
                let total_calls = tool_usage.success_count + tool_usage.error_count;
                let total_duration_seconds = (tool_usage.avg_duration_ms as f64 * total_calls as f64) / 1000.0;
                
                if let Err(e) = datadog_exporter
                    .send_histogram(
                        "goose.tool.duration",
                        total_calls,
                        total_duration_seconds,
                        tool_tags,
                    )
                    .await
                {
                    tracing::error!("Failed to send goose.tool.duration metric: {}", e);
                    return Err(e);
                }
            }
        }

        tracing::info!(
            "Successfully sent all Datadog metrics for recipe: {}",
            execution.recipe_name
        );
        Ok(())
    }

    async fn send_session_metrics(
        datadog_exporter: &crate::telemetry::datadog_metrics::DatadogMetricsExporter,
        execution: &SessionExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "Sending Datadog metrics for session: {}",
            execution.session_id
        );

        let base_tags = vec![
            format!("session_type:{:?}", execution.session_type),
            format!("usage_type:{:?}", execution.usage_type),
            format!(
                "result:{:?}",
                execution
                    .result
                    .as_ref()
                    .unwrap_or(&crate::telemetry::events::SessionResult::Success)
            ),
            "execution_type:session".to_string(),
        ];

        // Send session execution counter
        if let Err(e) = datadog_exporter
            .send_counter("goose.session.executions", 1, base_tags.clone())
            .await
        {
            tracing::error!("Failed to send goose.session.executions metric: {}", e);
            return Err(e);
        }

        if let Some(duration_ms) = execution.duration_ms {
            if let Err(e) = datadog_exporter
                .send_histogram(
                    "goose.session.duration",
                    1,
                    duration_ms as f64 / 1000.0,
                    base_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.session.duration metric: {}", e);
                return Err(e);
            }
        }

        if execution.message_count > 0 {
            if let Err(e) = datadog_exporter
                .send_gauge(
                    "goose.session.messages",
                    execution.message_count as f64,
                    base_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.session.messages metric: {}", e);
                return Err(e);
            }
        }

        if execution.turn_count > 0 {
            if let Err(e) = datadog_exporter
                .send_gauge(
                    "goose.session.turns",
                    execution.turn_count as f64,
                    base_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.session.turns metric: {}", e);
                return Err(e);
            }
        }

        // Handle token usage if present
        if let Some(token_usage) = &execution.token_usage {
            let token_tags = [
                base_tags.clone(),
                vec![
                    format!(
                        "model:{}",
                        token_usage.model.as_ref().unwrap_or(&"unknown".to_string())
                    ),
                    format!(
                        "provider:{}",
                        token_usage
                            .provider
                            .as_ref()
                            .unwrap_or(&"unknown".to_string())
                    ),
                ],
            ]
            .concat();

            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.input",
                    token_usage.input_tokens,
                    token_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.input metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.output",
                    token_usage.output_tokens,
                    token_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.output metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter
                .send_counter(
                    "goose.tokens.total",
                    token_usage.input_tokens + token_usage.output_tokens,
                    token_tags,
                )
                .await
            {
                tracing::error!("Failed to send goose.tokens.total metric: {}", e);
                return Err(e);
            }
        }

        // Handle tool usage
        for tool_usage in &execution.tool_usage {
            let tool_tags = [
                base_tags.clone(),
                vec![format!("tool_name:{}", tool_usage.tool_name)],
            ]
            .concat();

            if tool_usage.success_count > 0 {
                let success_tags = [tool_tags.clone(), vec!["result:success".to_string()]].concat();
                if let Err(e) = datadog_exporter
                    .send_counter("goose.tool.calls", tool_usage.success_count, success_tags)
                    .await
                {
                    tracing::error!("Failed to send goose.tool.calls success metric: {}", e);
                    return Err(e);
                }
            }

            if tool_usage.error_count > 0 {
                let error_tags = [tool_tags.clone(), vec!["result:error".to_string()]].concat();
                if let Err(e) = datadog_exporter
                    .send_counter("goose.tool.calls", tool_usage.error_count, error_tags)
                    .await
                {
                    tracing::error!("Failed to send goose.tool.calls error metric: {}", e);
                    return Err(e);
                }
            }

            // Send tool duration as histogram (count + sum) instead of just average
            if tool_usage.avg_duration_ms > 0 {
                let total_calls = tool_usage.success_count + tool_usage.error_count;
                let total_duration_seconds = (tool_usage.avg_duration_ms as f64 * total_calls as f64) / 1000.0;
                
                if let Err(e) = datadog_exporter
                    .send_histogram(
                        "goose.tool.duration",
                        total_calls,
                        total_duration_seconds,
                        tool_tags,
                    )
                    .await
                {
                    tracing::error!("Failed to send goose.tool.duration metric: {}", e);
                    return Err(e);
                }
            }
        }

        tracing::info!(
            "Successfully sent all Datadog metrics for session: {}",
            execution.session_id
        );
        Ok(())
    }

    async fn send_command_metrics(
        datadog_exporter: &crate::telemetry::datadog_metrics::DatadogMetricsExporter,
        execution: &CommandExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "Sending Datadog metrics for command: {}",
            execution.command_name
        );

        let base_tags = vec![
            format!("command_name:{}", execution.command_name),
            format!("command_type:{:?}", execution.command_type),
            format!("usage_type:{:?}", execution.usage_type),
            format!(
                "result:{:?}",
                execution
                    .result
                    .as_ref()
                    .unwrap_or(&crate::telemetry::events::CommandResult::Success)
            ),
            "execution_type:command".to_string(),
        ];

        // Send command execution counter
        if let Err(e) = datadog_exporter
            .send_counter("goose.command.executions", 1, base_tags.clone())
            .await
        {
            tracing::error!("Failed to send goose.command.executions metric: {}", e);
            return Err(e);
        }

        if let Some(duration_ms) = execution.duration_ms {
            if let Err(e) = datadog_exporter
                .send_histogram(
                    "goose.command.duration",
                    1,
                    duration_ms as f64 / 1000.0,
                    base_tags.clone(),
                )
                .await
            {
                tracing::error!("Failed to send goose.command.duration metric: {}", e);
                return Err(e);
            }
        }

        tracing::info!(
            "Successfully sent all Datadog metrics for command: {}",
            execution.command_name
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl super::TelemetryBackend for DatadogProvider {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            return Ok(());
        }

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("DD_API_KEY").ok())
            .ok_or("Datadog API key is required (set GOOSE_TELEMETRY_API_KEY or DD_API_KEY)")?;

        let endpoint = config
            .get_endpoint()
            .ok_or("Datadog provider requires GOOSE_TELEMETRY_ENDPOINT to be set")?;

        // Create HTTP-based metrics exporter
        let datadog_metrics_exporter =
            crate::telemetry::datadog_metrics::DatadogMetricsExporter::new(
                api_key.clone(),
                endpoint.clone(),
            )
            .with_tags(vec![
                format!("service:{}", config.service_name),
                format!("version:{}", config.service_version),
                format!(
                    "usage_type:{:?}",
                    config
                        .usage_type
                        .as_ref()
                        .unwrap_or(&crate::telemetry::config::UsageType::Human)
                ),
            ]);

        self.datadog_metrics = Some(datadog_metrics_exporter);

        // Note: Traces are not implemented for Datadog HTTP API
        // Datadog traces require either:
        // 1. Datadog Agent (not suitable for our use case)
        // 2. Custom subdomain endpoints (not available for traces)
        // 3. Complex msgpack format (adds dependency overhead)
        //
        // Our comprehensive metrics already provide the key insights:
        // - Recipe execution counts and timing
        // - Token usage and costs
        // - Tool usage patterns
        // - Error rates and success metrics
        //
        // This covers 95% of our telemetry needs without the complexity of traces.
        self.initialized = true;

        tracing::info!("Datadog telemetry provider initialized with endpoint: {} (metrics via HTTP API, no agent required)", endpoint);
        Ok(())
    }

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            return Err("Datadog provider not initialized".into());
        }

        match event {
            TelemetryEvent::RecipeExecution(execution) => {
                if let Some(datadog_metrics) = &self.datadog_metrics {
                    if let Err(e) = Self::send_datadog_metrics(datadog_metrics, execution).await {
                        tracing::error!("Failed to send Datadog recipe metrics: {}", e);
                    }
                }
            }
            TelemetryEvent::SessionExecution(execution) => {
                if let Some(datadog_metrics) = &self.datadog_metrics {
                    if let Err(e) = Self::send_session_metrics(datadog_metrics, execution).await {
                        tracing::error!("Failed to send Datadog session metrics: {}", e);
                    }
                }
            }
            TelemetryEvent::CommandExecution(execution) => {
                if let Some(datadog_metrics) = &self.datadog_metrics {
                    if let Err(e) = Self::send_command_metrics(datadog_metrics, execution).await {
                        tracing::error!("Failed to send Datadog command metrics: {}", e);
                    }
                }
            }
            TelemetryEvent::SystemMetrics(_metrics) => {
                // System metrics could be implemented in the future
            }
            TelemetryEvent::UserSession(_session) => {
                // User session metrics could be implemented in the future
            }
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            tracing::info!("Datadog telemetry provider shutdown successfully");
        }
        Ok(())
    }
}
