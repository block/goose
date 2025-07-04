use crate::telemetry::{
    config::{TelemetryConfig, TelemetryProvider},
    events::{RecipeExecution, TelemetryEvent},
};
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, MeterProvider},
    trace::{Span, Tracer, TracerProvider as OtelTracerProvider},
    KeyValue,
};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
    trace::TracerProvider,
    Resource,
};
use opentelemetry_datadog::{new_pipeline, DatadogPropagator};
use opentelemetry_semantic_conventions as semconv;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

#[async_trait::async_trait]
pub trait TelemetryBackend: Send + Sync {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct OpenTelemetryProvider {
    tracer_provider: Option<TracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
    recipe_counter: Option<Counter<u64>>,
    recipe_duration: Option<Histogram<f64>>,
    token_counter: Option<Counter<u64>>,
    tool_counter: Option<Counter<u64>>,
    datadog_metrics: Option<crate::telemetry::datadog_metrics::DatadogMetricsExporter>,
    initialized: bool,
}

impl OpenTelemetryProvider {
    pub fn new() -> Self {
        Self {
            tracer_provider: None,
            meter_provider: None,
            recipe_counter: None,
            recipe_duration: None,
            token_counter: None,
            tool_counter: None,
            datadog_metrics: None,
            initialized: false,
        }
    }

    async fn init_opentelemetry(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resource = Resource::new(vec![
            KeyValue::new(semconv::resource::SERVICE_NAME, config.service_name.clone()),
            KeyValue::new(
                semconv::resource::SERVICE_VERSION,
                config.service_version.clone(),
            ),
            KeyValue::new(
                "goose.usage_type",
                format!(
                    "{:?}",
                    config
                        .usage_type
                        .as_ref()
                        .unwrap_or(&crate::telemetry::config::UsageType::Human)
                ),
            ),
        ]);

        match config.provider {
            TelemetryProvider::Console => {
                self.init_console(config, resource).await?;
            }
            TelemetryProvider::Datadog => {
                self.init_datadog(config, resource).await?;
            }
            TelemetryProvider::Otlp => {
                self.init_otlp(config, resource).await?;
            }
            _ => {
                return Err(format!("Provider {:?} is not yet implemented", config.provider).into());
            }
        }

        self.init_metrics().await?;

        self.initialized = true;
        Ok(())
    }

    async fn init_console(
        &mut self,
        _config: &TelemetryConfig,
        resource: Resource,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build();

        self.tracer_provider = Some(tracer_provider);

        let meter_provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(
                PeriodicReader::builder(
                    opentelemetry_stdout::MetricExporter::default(),
                    runtime::Tokio,
                )
                .with_interval(Duration::from_secs(30))
                .build(),
            )
            .build();

        self.meter_provider = Some(meter_provider);

        Ok(())
    }

    async fn init_datadog(
        &mut self,
        config: &TelemetryConfig,
        resource: Resource,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("DD_API_KEY").ok())
            .ok_or("Datadog API key is required (set GOOSE_TELEMETRY_API_KEY or DD_API_KEY)")?;

        let endpoint = config.get_endpoint()
            .ok_or("Datadog provider requires GOOSE_TELEMETRY_ENDPOINT to be set")?;

        opentelemetry::global::set_text_map_propagator(DatadogPropagator::new());

        let datadog_exporter = new_pipeline()
            .with_service_name(config.service_name.clone())
            .with_version(config.service_version.clone())
            .with_agent_endpoint(endpoint.clone())
            .build_exporter()?;

        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(datadog_exporter, runtime::Tokio)
            .build();

        self.tracer_provider = Some(tracer_provider);

        let datadog_metrics_exporter = crate::telemetry::datadog_metrics::DatadogMetricsExporter::new(
            api_key,
            endpoint.clone(),
        ).with_tags(vec![
            format!("service:{}", config.service_name),
            format!("version:{}", config.service_version),
            format!("usage_type:{:?}", config.usage_type.as_ref().unwrap_or(&crate::telemetry::config::UsageType::Human)),
        ]);

        self.datadog_metrics = Some(datadog_metrics_exporter);

        // For Datadog, we don't need OpenTelemetry metrics since we send directly via HTTP
        // Set meter_provider to None to avoid duplicate stdout metrics
        self.meter_provider = None;

        tracing::info!("Datadog telemetry provider initialized with endpoint: {} (traces via agent, metrics via HTTP API)", endpoint);
        Ok(())
    }

    async fn init_otlp(
        &mut self,
        config: &TelemetryConfig,
        resource: Resource,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
        use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;

        let endpoint = config.get_endpoint()
            .ok_or("OTLP provider requires GOOSE_TELEMETRY_ENDPOINT or OTEL_EXPORTER_OTLP_ENDPOINT")?;

        let mut otlp_exporter_builder = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint.clone());

        if let Some(api_key) = &config.api_key {
            use tonic::metadata::{MetadataMap, MetadataValue};
            
            let mut metadata = MetadataMap::new();
            
            let header_name = std::env::var("GOOSE_TELEMETRY_AUTH_HEADER")
                .unwrap_or_else(|_| "x-api-key".to_string());
            
            let metadata_value = MetadataValue::try_from(api_key.as_str())
                .map_err(|e| format!("Invalid API key format: {}", e))?;
            
            match header_name.as_str() {
                "x-api-key" => metadata.insert("x-api-key", metadata_value),
                "authorization" => metadata.insert("authorization", metadata_value),
                "x-honeycomb-team" => metadata.insert("x-honeycomb-team", metadata_value),
                "x-otlp-api-key" => metadata.insert("x-otlp-api-key", metadata_value),
                _ => {
                    let key = tonic::metadata::MetadataKey::from_bytes(header_name.as_bytes())
                        .map_err(|e| format!("Invalid header name '{}': {}", header_name, e))?;
                    metadata.insert(key, metadata_value)
                }
            };
            
            otlp_exporter_builder = otlp_exporter_builder.with_metadata(metadata);
            
            tracing::info!("OTLP provider configured with API key authentication");
        }

        let otlp_trace_exporter = otlp_exporter_builder.build()?;

        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(otlp_trace_exporter, runtime::Tokio)
            .build();

        self.tracer_provider = Some(tracer_provider);

        let mut otlp_metrics_builder = opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint.clone());

        if let Some(api_key) = &config.api_key {
            use tonic::metadata::{MetadataMap, MetadataValue};
            
            let mut metadata = MetadataMap::new();
            
            let header_name = std::env::var("GOOSE_TELEMETRY_AUTH_HEADER")
                .unwrap_or_else(|_| "x-api-key".to_string());
            
            let metadata_value = MetadataValue::try_from(api_key.as_str())
                .map_err(|e| format!("Invalid API key format: {}", e))?;
            
            match header_name.as_str() {
                "x-api-key" => metadata.insert("x-api-key", metadata_value),
                "authorization" => metadata.insert("authorization", metadata_value),
                "x-honeycomb-team" => metadata.insert("x-honeycomb-team", metadata_value),
                "x-otlp-api-key" => metadata.insert("x-otlp-api-key", metadata_value),
                _ => {
                    let key = tonic::metadata::MetadataKey::from_bytes(header_name.as_bytes())
                        .map_err(|e| format!("Invalid header name '{}': {}", header_name, e))?;
                    metadata.insert(key, metadata_value)
                }
            };
            
            otlp_metrics_builder = otlp_metrics_builder.with_metadata(metadata);
        }

        let otlp_metrics_exporter = otlp_metrics_builder.build()?;

        let meter_provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(
                PeriodicReader::builder(otlp_metrics_exporter, runtime::Tokio)
                    .with_interval(Duration::from_secs(30))
                    .build(),
            )
            .build();

        self.meter_provider = Some(meter_provider);

        let auth_status = if config.api_key.is_some() { "with authentication" } else { "without authentication" };
        tracing::info!("OTLP telemetry provider initialized with endpoint: {} ({})", endpoint, auth_status);
        Ok(())
    }

    async fn init_metrics(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(meter_provider) = &self.meter_provider {
            let meter = meter_provider.meter("goose");

            self.recipe_counter = Some(
                meter
                    .u64_counter("goose.recipe.executions")
                    .with_description("Number of recipe executions")
                    .build(),
            );

            self.recipe_duration = Some(
                meter
                    .f64_histogram("goose.recipe.duration")
                    .with_description("Recipe execution duration in seconds")
                    .build(),
            );

            self.token_counter = Some(
                meter
                    .u64_counter("goose.tokens.used")
                    .with_description("Number of tokens used")
                    .build(),
            );

            self.tool_counter = Some(
                meter
                    .u64_counter("goose.tool.calls")
                    .with_description("Number of tool calls")
                    .build(),
            );
        }

        Ok(())
    }

    fn record_recipe_execution(&self, execution: &RecipeExecution) {
        let attributes = vec![
            KeyValue::new("recipe.name", execution.recipe_name.clone()),
            KeyValue::new("recipe.version", execution.recipe_version.clone()),
            KeyValue::new("usage.type", format!("{:?}", execution.usage_type)),
            KeyValue::new(
                "result",
                format!(
                    "{:?}",
                    execution
                        .result
                        .as_ref()
                        .unwrap_or(&crate::telemetry::events::RecipeResult::Success)
                ),
            ),
        ];

        if let Some(counter) = &self.recipe_counter {
            counter.add(1, &attributes);
        }

        if let (Some(histogram), Some(duration_ms)) = (&self.recipe_duration, execution.duration_ms)
        {
            histogram.record(duration_ms as f64 / 1000.0, &attributes);
        }

        if let (Some(counter), Some(token_usage)) = (&self.token_counter, &execution.token_usage) {
            let token_attributes = [
                attributes.clone(),
                vec![
                    KeyValue::new("token.type", "input"),
                    KeyValue::new("model", token_usage.model.clone().unwrap_or_default()),
                    KeyValue::new("provider", token_usage.provider.clone().unwrap_or_default()),
                ],
            ]
            .concat();

            counter.add(token_usage.input_tokens, &token_attributes);

            let output_attributes = [
                attributes.clone(),
                vec![
                    KeyValue::new("token.type", "output"),
                    KeyValue::new("model", token_usage.model.clone().unwrap_or_default()),
                    KeyValue::new("provider", token_usage.provider.clone().unwrap_or_default()),
                ],
            ]
            .concat();

            counter.add(token_usage.output_tokens, &output_attributes);
        }

        if let Some(counter) = &self.tool_counter {
            for tool_usage in &execution.tool_usage {
                let tool_attributes = [
                    attributes.clone(),
                    vec![
                        KeyValue::new("tool.name", tool_usage.tool_name.clone()),
                        KeyValue::new("tool.result", "success"),
                    ],
                ]
                .concat();

                counter.add(tool_usage.success_count, &tool_attributes);

                if tool_usage.error_count > 0 {
                    let error_attributes = [
                        attributes.clone(),
                        vec![
                            KeyValue::new("tool.name", tool_usage.tool_name.clone()),
                            KeyValue::new("tool.result", "error"),
                        ],
                    ]
                    .concat();

                    counter.add(tool_usage.error_count, &error_attributes);
                }
            }
        }

        if let Some(datadog_metrics) = &self.datadog_metrics {
            let execution_clone = execution.clone();
            
            let datadog_exporter = datadog_metrics.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::send_datadog_metrics(&datadog_exporter, &execution_clone).await {
                    tracing::error!("Failed to send Datadog metrics: {}", e);
                }
            });
        }
    }

    async fn send_datadog_metrics(
        datadog_exporter: &crate::telemetry::datadog_metrics::DatadogMetricsExporter,
        execution: &RecipeExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let base_tags = vec![
            format!("recipe_name:{}", execution.recipe_name),
            format!("recipe_version:{}", execution.recipe_version),
            format!("usage_type:{:?}", execution.usage_type),
            format!("result:{:?}", execution.result.as_ref().unwrap_or(&crate::telemetry::events::RecipeResult::Success)),
        ];

        datadog_exporter.send_counter("recipe.executions", 1, base_tags.clone()).await?;

        if let Some(duration_ms) = execution.duration_ms {
            datadog_exporter.send_histogram(
                "recipe.duration",
                1,
                duration_ms as f64 / 1000.0,
                base_tags.clone(),
            ).await?;
        }

        if let Some(token_usage) = &execution.token_usage {
            let token_tags = [
                base_tags.clone(),
                vec![
                    format!("model:{}", token_usage.model.as_ref().unwrap_or(&"unknown".to_string())),
                    format!("provider:{}", token_usage.provider.as_ref().unwrap_or(&"unknown".to_string())),
                ],
            ].concat();

            datadog_exporter.send_counter("tokens.input", token_usage.input_tokens, token_tags.clone()).await?;
            datadog_exporter.send_counter("tokens.output", token_usage.output_tokens, token_tags.clone()).await?;
            datadog_exporter.send_counter("tokens.total", token_usage.input_tokens + token_usage.output_tokens, token_tags).await?;
        }

        for tool_usage in &execution.tool_usage {
            let tool_tags = [
                base_tags.clone(),
                vec![format!("tool_name:{}", tool_usage.tool_name)],
            ].concat();

            if tool_usage.success_count > 0 {
                let success_tags = [tool_tags.clone(), vec!["result:success".to_string()]].concat();
                datadog_exporter.send_counter("tool.calls", tool_usage.success_count, success_tags).await?;
            }

            if tool_usage.error_count > 0 {
                let error_tags = [tool_tags.clone(), vec!["result:error".to_string()]].concat();
                datadog_exporter.send_counter("tool.calls", tool_usage.error_count, error_tags).await?;
            }

            if tool_usage.avg_duration_ms > 0 {
                datadog_exporter.send_gauge(
                    "tool.duration.avg",
                    tool_usage.avg_duration_ms as f64 / 1000.0,
                    tool_tags,
                ).await?;
            }
        }

        Ok(())
    }

    fn create_recipe_span(&self, execution: &RecipeExecution) {
        if let Some(tracer_provider) = &self.tracer_provider {
            let tracer = tracer_provider.tracer("goose");
            let mut span = tracer
                .span_builder(format!("recipe.{}", execution.recipe_name))
                .with_attributes(vec![
                    KeyValue::new("recipe.name", execution.recipe_name.clone()),
                    KeyValue::new("recipe.version", execution.recipe_version.clone()),
                    KeyValue::new("usage.type", format!("{:?}", execution.usage_type)),
                    KeyValue::new("user.id", execution.user_id.clone()),
                ])
                .start(&tracer);

            if let Some(result) = &execution.result {
                span.set_attribute(KeyValue::new("result", format!("{:?}", result)));
            }

            if let Some(duration_ms) = execution.duration_ms {
                span.set_attribute(KeyValue::new("duration.ms", duration_ms as i64));
            }

            if let Some(error_details) = &execution.error_details {
                span.set_attribute(KeyValue::new(
                    "error.type",
                    error_details.error_type.clone(),
                ));
                span.set_attribute(KeyValue::new(
                    "error.message",
                    error_details.error_message.clone(),
                ));
                span.record_error(&std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error_details.error_message.clone(),
                ));
            }

            span.end();
        }
    }
}

#[async_trait::async_trait]
impl TelemetryBackend for OpenTelemetryProvider {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            return Ok(());
        }

        self.init_opentelemetry(config).await?;
        Ok(())
    }

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            return Err("Telemetry provider not initialized".into());
        }

        match event {
            TelemetryEvent::RecipeExecution(execution) => {
                self.record_recipe_execution(execution);

                self.create_recipe_span(execution);
            }
            TelemetryEvent::SystemMetrics(_metrics) => {}
            TelemetryEvent::UserSession(_session) => {}
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            global::shutdown_tracer_provider();
        }
        Ok(())
    }
}

/// File-based telemetry provider that writes JSON events to a file. mostly for debugging
pub struct FileProvider {
    file_path: Option<String>,
    file_handle: Option<Mutex<std::fs::File>>,
    initialized: bool,
}

impl FileProvider {
    pub fn new() -> Self {
        Self {
            file_path: None,
            file_handle: None,
            initialized: false,
        }
    }

    fn write_event_to_file(&self, event: &TelemetryEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_handle) = &self.file_handle {
            let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;

            let event_record = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": match event {
                    TelemetryEvent::RecipeExecution(_) => "recipe_execution",
                    TelemetryEvent::SystemMetrics(_) => "system_metrics", 
                    TelemetryEvent::UserSession(_) => "user_session",
                },
                "data": event
            });

            writeln!(file, "{}", serde_json::to_string(&event_record)?)?;
            file.flush()?;
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl TelemetryBackend for FileProvider {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            return Ok(());
        }

        let file_path = config.get_endpoint()
            .ok_or("File provider requires a file path in GOOSE_TELEMETRY_ENDPOINT")?;

        if let Some(parent) = Path::new(&file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        self.file_path = Some(file_path.clone());
        self.file_handle = Some(Mutex::new(file));
        self.initialized = true;

        let init_record = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "event_type": "telemetry_init",
            "data": {
                "service_name": config.service_name,
                "service_version": config.service_version,
                "provider": "file",
                "file_path": file_path
            }
        });

        if let Some(file_handle) = &self.file_handle {
            let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;
            writeln!(file, "{}", serde_json::to_string(&init_record)?)?;
            file.flush()?;
        }

        tracing::info!("File telemetry provider initialized: {}", file_path);
        Ok(())
    }

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            return Err("File provider not initialized".into());
        }

        self.write_event_to_file(event)?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            let shutdown_record = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "telemetry_shutdown",
                "data": {}
            });

            if let Some(file_handle) = &self.file_handle {
                let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;
                writeln!(file, "{}", serde_json::to_string(&shutdown_record)?)?;
                file.flush()?;
            }

            if let Some(file_path) = &self.file_path {
                tracing::info!("File telemetry provider shutdown: {}", file_path);
            }
        }
        Ok(())
    }
}

pub fn create_backend(config: &TelemetryConfig) -> Box<dyn TelemetryBackend> {
    match config.provider {
        TelemetryProvider::File => Box::new(FileProvider::new()),
        _ => Box::new(OpenTelemetryProvider::new()),
    }
}
