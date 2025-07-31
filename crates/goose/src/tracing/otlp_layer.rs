use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry_sdk::{runtime, Resource};
use std::env;
use std::time::Duration;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

#[derive(Debug, Clone)]
pub struct OtlpConfig {
    pub endpoint: String,
    pub timeout: Duration,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4318".to_string(),
            timeout: Duration::from_secs(10),
        }
    }
}

impl OtlpConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(endpoint) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.endpoint = endpoint;
        }

        if let Ok(timeout_str) = env::var("OTEL_EXPORTER_OTLP_TIMEOUT") {
            if let Ok(timeout_ms) = timeout_str.parse::<u64>() {
                config.timeout = Duration::from_millis(timeout_ms);
            }
        }

        config
    }
}

pub fn init_otlp_tracing(
    config: &OtlpConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "goose"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "goose"),
    ]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(&config.endpoint)
        .with_timeout(config.timeout)
        .build()?;

    let tracer_provider = trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(resource.clone())
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::AlwaysOn)
        .build();

    global::set_tracer_provider(tracer_provider);

    Ok(())
}

pub fn init_otlp_metrics(
    config: &OtlpConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "goose"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "goose"),
    ]);

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(&config.endpoint)
        .with_timeout(config.timeout)
        .build()?;

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(
            opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, runtime::Tokio)
                .with_interval(Duration::from_secs(30))
                .build(),
        )
        .build();

    global::set_meter_provider(meter_provider);

    Ok(())
}

pub fn create_otlp_tracing_layer() -> Result<
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let config = OtlpConfig::from_env();

    let resource = Resource::new(vec![
        KeyValue::new("service.name", "goose"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "goose"),
    ]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(&config.endpoint)
        .with_timeout(config.timeout)
        .build()?;

    let tracer_provider = trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(resource)
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::AlwaysOn)
        .build();

    let tracer = tracer_provider.tracer("goose");
    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn create_otlp_metrics_layer(
) -> Result<MetricsLayer<tracing_subscriber::Registry>, Box<dyn std::error::Error + Send + Sync>> {
    Err("Metrics layer not supported yet".into())
}

pub fn init_otlp() -> Result<
    (
        OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>,
        MetricsLayer<tracing_subscriber::Registry>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    // For now, we'll skip metrics and just return the tracing layer
    Err("Full OTLP support with metrics not implemented yet".into())
}

pub fn init_otlp_tracing_only() -> Result<
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    create_otlp_tracing_layer()
}

pub fn shutdown_otlp() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_otlp_config_default() {
        let config = OtlpConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4318");
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_otlp_config_from_env() {
        let original_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        let original_timeout = env::var("OTEL_EXPORTER_OTLP_TIMEOUT").ok();

        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://test:4317");
        env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", "5000");

        let config = OtlpConfig::from_env();
        assert_eq!(config.endpoint, "http://test:4317");
        assert_eq!(config.timeout, Duration::from_millis(5000));

        match original_endpoint {
            Some(val) => env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", val),
            None => env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT"),
        }
        match original_timeout {
            Some(val) => env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", val),
            None => env::remove_var("OTEL_EXPORTER_OTLP_TIMEOUT"),
        }
    }
}
