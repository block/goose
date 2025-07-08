use crate::telemetry::{
    config::TelemetryConfig,
    events::{RecipeExecution, TelemetryEvent},
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
        tracing::info!("Sending Datadog metrics for recipe: {}", execution.recipe_name);
        
        let base_tags = vec![
            format!("recipe_name:{}", execution.recipe_name),
            format!("recipe_version:{}", execution.recipe_version),
            format!("usage_type:{:?}", execution.usage_type),
            format!("result:{:?}", execution.result.as_ref().unwrap_or(&crate::telemetry::events::RecipeResult::Success)),
        ];

        // Send recipe execution counter
        if let Err(e) = datadog_exporter.send_counter("recipe.executions", 1, base_tags.clone()).await {
            tracing::error!("Failed to send recipe.executions metric: {}", e);
            return Err(e);
        }
        tracing::info!("Successfully sent recipe.executions metric");

        if let Some(duration_ms) = execution.duration_ms {
            if let Err(e) = datadog_exporter.send_histogram(
                "recipe.duration",
                1,
                duration_ms as f64 / 1000.0,
                base_tags.clone(),
            ).await {
                tracing::error!("Failed to send recipe.duration metric: {}", e);
                return Err(e);
            }
            tracing::info!("Successfully sent recipe.duration metric");
        }

        if let Some(token_usage) = &execution.token_usage {
            let token_tags = [
                base_tags.clone(),
                vec![
                    format!("model:{}", token_usage.model.as_ref().unwrap_or(&"unknown".to_string())),
                    format!("provider:{}", token_usage.provider.as_ref().unwrap_or(&"unknown".to_string())),
                ],
            ].concat();

            if let Err(e) = datadog_exporter.send_counter("tokens.input", token_usage.input_tokens, token_tags.clone()).await {
                tracing::error!("Failed to send tokens.input metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter.send_counter("tokens.output", token_usage.output_tokens, token_tags.clone()).await {
                tracing::error!("Failed to send tokens.output metric: {}", e);
                return Err(e);
            }
            if let Err(e) = datadog_exporter.send_counter("tokens.total", token_usage.input_tokens + token_usage.output_tokens, token_tags).await {
                tracing::error!("Failed to send tokens.total metric: {}", e);
                return Err(e);
            }
            tracing::info!("Successfully sent token usage metrics");
        }

        for tool_usage in &execution.tool_usage {
            let tool_tags = [
                base_tags.clone(),
                vec![format!("tool_name:{}", tool_usage.tool_name)],
            ].concat();

            if tool_usage.success_count > 0 {
                let success_tags = [tool_tags.clone(), vec!["result:success".to_string()]].concat();
                if let Err(e) = datadog_exporter.send_counter("tool.calls", tool_usage.success_count, success_tags).await {
                    tracing::error!("Failed to send tool.calls success metric: {}", e);
                    return Err(e);
                }
            }

            if tool_usage.error_count > 0 {
                let error_tags = [tool_tags.clone(), vec!["result:error".to_string()]].concat();
                if let Err(e) = datadog_exporter.send_counter("tool.calls", tool_usage.error_count, error_tags).await {
                    tracing::error!("Failed to send tool.calls error metric: {}", e);
                    return Err(e);
                }
            }

            if tool_usage.avg_duration_ms > 0 {
                if let Err(e) = datadog_exporter.send_gauge(
                    "tool.duration.avg",
                    tool_usage.avg_duration_ms as f64 / 1000.0,
                    tool_tags,
                ).await {
                    tracing::error!("Failed to send tool.duration.avg metric: {}", e);
                    return Err(e);
                }
            }
        }

        tracing::info!("Successfully sent all Datadog metrics for recipe: {}", execution.recipe_name);
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

        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("DD_API_KEY").ok())
            .ok_or("Datadog API key is required (set GOOSE_TELEMETRY_API_KEY or DD_API_KEY)")?;

        let endpoint = config.get_endpoint()
            .ok_or("Datadog provider requires GOOSE_TELEMETRY_ENDPOINT to be set")?;

        // Create HTTP-based metrics exporter
        let datadog_metrics_exporter = crate::telemetry::datadog_metrics::DatadogMetricsExporter::new(
            api_key.clone(),
            endpoint.clone(),
        ).with_tags(vec![
            format!("service:{}", config.service_name),
            format!("version:{}", config.service_version),
            format!("usage_type:{:?}", config.usage_type.as_ref().unwrap_or(&crate::telemetry::config::UsageType::Human)),
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
                        tracing::error!("Failed to send Datadog metrics: {}", e);
                    }
                }

                // Datadog traces are not implemented - focusing on comprehensive metrics
                // which already provide the key insights we need for Goose telemetry
            }
            TelemetryEvent::SystemMetrics(_metrics) => {}
            TelemetryEvent::UserSession(_session) => {}
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
