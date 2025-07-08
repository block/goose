use crate::telemetry::{
    config::TelemetryConfig,
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
use opentelemetry_semantic_conventions as semconv;
use std::time::Duration;

pub struct OtlpProvider {
    tracer_provider: Option<TracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
    recipe_counter: Option<Counter<u64>>,
    recipe_duration: Option<Histogram<f64>>,
    token_counter: Option<Counter<u64>>,
    tool_counter: Option<Counter<u64>>,
    initialized: bool,
}

impl OtlpProvider {
    pub fn new() -> Self {
        Self {
            tracer_provider: None,
            meter_provider: None,
            recipe_counter: None,
            recipe_duration: None,
            token_counter: None,
            tool_counter: None,
            initialized: false,
        }
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
impl super::TelemetryBackend for OtlpProvider {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            return Ok(());
        }

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

        self.init_metrics().await?;
        self.initialized = true;

        let auth_status = if config.api_key.is_some() { "with authentication" } else { "without authentication" };
        tracing::info!("OTLP telemetry provider initialized with endpoint: {} ({})", endpoint, auth_status);
        Ok(())
    }

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            return Err("OTLP provider not initialized".into());
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
            tracing::info!("OTLP telemetry provider shutdown successfully");
        }
        Ok(())
    }
}
