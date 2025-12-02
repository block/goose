use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::{Logger, LoggerProvider};
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry_sdk::{runtime, Resource};
use std::time::Duration;
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry>;
pub type OtlpLogsLayer = OpenTelemetryTracingBridge<LoggerProvider, Logger>;
pub type OtlpLayers = (OtlpTracingLayer, OtlpMetricsLayer, OtlpLogsLayer);
pub type OtlpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
    pub fn from_config() -> Option<Self> {
        let config = crate::config::Config::global();

        // Try to get the endpoint from config (checks OTEL_EXPORTER_OTLP_ENDPOINT env var first)
        let endpoint = config
            .get_param::<String>("otel_exporter_otlp_endpoint")
            .ok()?;

        let mut otlp_config = Self {
            endpoint,
            timeout: Duration::from_secs(10),
        };

        // Try to get timeout from config (checks OTEL_EXPORTER_OTLP_TIMEOUT env var first)
        if let Ok(timeout_ms) = config.get_param::<u64>("otel_exporter_otlp_timeout") {
            otlp_config.timeout = Duration::from_millis(timeout_ms);
        }

        Some(otlp_config)
    }
}

pub fn init_otlp_tracing(config: &OtlpConfig) -> OtlpResult<()> {
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

pub fn init_otlp_metrics(config: &OtlpConfig) -> OtlpResult<()> {
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
                .with_interval(Duration::from_secs(3))
                .build(),
        )
        .build();

    global::set_meter_provider(meter_provider);

    Ok(())
}

pub fn create_otlp_tracing_layer() -> OtlpResult<OtlpTracingLayer> {
    let config = OtlpConfig::from_config().ok_or("OTEL_EXPORTER_OTLP_ENDPOINT not configured")?;

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
        .with_max_events_per_span(2048)
        .with_max_attributes_per_span(512)
        .with_max_links_per_span(512)
        .with_resource(resource)
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::TraceIdRatioBased(0.1))
        .build();

    let tracer = tracer_provider.tracer("goose");
    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn create_otlp_metrics_layer() -> OtlpResult<OtlpMetricsLayer> {
    let config = OtlpConfig::from_config().ok_or("OTEL_EXPORTER_OTLP_ENDPOINT not configured")?;

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
                .with_interval(Duration::from_millis(2000))
                .build(),
        )
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(tracing_opentelemetry::MetricsLayer::new(meter_provider))
}

pub fn create_otlp_logs_layer() -> OtlpResult<OpenTelemetryTracingBridge<LoggerProvider, Logger>> {
    let config = OtlpConfig::from_config().ok_or("OTEL_EXPORTER_OTLP_ENDPOINT not configured")?;

    let resource = Resource::new(vec![
        KeyValue::new("service.name", "goose"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "goose"),
    ]);

    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(&config.endpoint)
        .with_timeout(config.timeout)
        .build()?;

    let logger_provider = LoggerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(resource)
        .build();

    Ok(OpenTelemetryTracingBridge::new(&logger_provider))
}

pub fn init_otlp() -> OtlpResult<OtlpLayers> {
    let tracing_layer = create_otlp_tracing_layer()?;
    let metrics_layer = create_otlp_metrics_layer()?;
    let logs_layer = create_otlp_logs_layer()?;
    Ok((tracing_layer, metrics_layer, logs_layer))
}

pub fn init_otlp_tracing_only() -> OtlpResult<OtlpTracingLayer> {
    create_otlp_tracing_layer()
}

/// Creates a custom filter for OTLP tracing that captures:
/// - All spans at INFO level and above
/// - Specific spans marked with "otel.trace" field
/// - Events from specific modules related to telemetry
pub fn create_otlp_tracing_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata: &Metadata<'_>| {
        if metadata.level() <= &Level::INFO {
            return true;
        }

        if metadata.level() == &Level::DEBUG {
            let target = metadata.target();
            if target.starts_with("goose::")
                || target.starts_with("opentelemetry")
                || target.starts_with("tracing_opentelemetry")
            {
                return true;
            }
        }

        false
    })
}

/// Creates a custom filter for OTLP metrics that captures:
/// - All events at INFO level and above
/// - Specific events marked with "otel.metric" field
/// - Events that should be converted to metrics
pub fn create_otlp_metrics_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata: &Metadata<'_>| {
        if metadata.level() <= &Level::INFO {
            return true;
        }

        if metadata.level() == &Level::DEBUG {
            let target = metadata.target();
            if target.starts_with("goose::telemetry")
                || target.starts_with("goose::metrics")
                || target.contains("metric")
            {
                return true;
            }
        }

        false
    })
}

/// Creates a custom filter for OTLP metrics that captures:
/// - All events at WARN level and above
pub fn create_otlp_logs_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata: &Metadata<'_>| {
        if metadata.level() <= &Level::WARN {
            return true;
        }

        false
    })
}

/// Shutdown OTLP providers gracefully
pub fn shutdown_otlp() {
    // Shutdown the tracer provider and flush any pending spans
    global::shutdown_tracer_provider();

    // Force flush of metrics by waiting a bit
    // The meter provider doesn't have a direct shutdown method in the current SDK,
    // but we can give it time to export any pending metrics
    std::thread::sleep(std::time::Duration::from_millis(500));
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
    fn test_otlp_config_from_config() {
        use tempfile::NamedTempFile;

        // Save original env vars
        let original_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        let original_timeout = env::var("OTEL_EXPORTER_OTLP_TIMEOUT").ok();

        // Clear env vars to ensure we're testing config file
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        env::remove_var("OTEL_EXPORTER_OTLP_TIMEOUT");

        // Create a test config file
        let temp_file = NamedTempFile::new().unwrap();
        let test_config = crate::config::Config::new(temp_file.path(), "test-otlp").unwrap();

        // Set values in config
        test_config
            .set_param("otel_exporter_otlp_endpoint", "http://config:4318")
            .unwrap();
        test_config
            .set_param("otel_exporter_otlp_timeout", 3000)
            .unwrap();

        // Test that from_config reads from the config file
        // Note: We can't easily test from_config() directly since it uses Config::global()
        // But we can test that the config system works with our keys
        let endpoint: String = test_config
            .get_param("otel_exporter_otlp_endpoint")
            .unwrap();
        assert_eq!(endpoint, "http://config:4318");

        let timeout: u64 = test_config.get_param("otel_exporter_otlp_timeout").unwrap();
        assert_eq!(timeout, 3000);

        // Test env var override still works
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://env:4317");
        let endpoint: String = test_config
            .get_param("otel_exporter_otlp_endpoint")
            .unwrap();
        assert_eq!(endpoint, "http://env:4317");

        // Restore original env vars
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
