use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry_sdk::{runtime, Resource};
use std::env;
use std::time::Duration;
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry>;
pub type OtlpLayers = (OtlpTracingLayer, OtlpMetricsLayer);
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
    pub fn from_env() -> Option<Self> {
        if let Ok(endpoint) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            let mut config = Self {
                endpoint,
                timeout: Duration::from_secs(10),
            };

            if let Ok(timeout_str) = env::var("OTEL_EXPORTER_OTLP_TIMEOUT") {
                if let Ok(timeout_ms) = timeout_str.parse::<u64>() {
                    config.timeout = Duration::from_millis(timeout_ms);
                }
            }

            Some(config)
        } else {
            None
        }
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
    let config =
        OtlpConfig::from_env().ok_or("OTEL_EXPORTER_OTLP_ENDPOINT environment variable not set")?;

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
    let config =
        OtlpConfig::from_env().ok_or("OTEL_EXPORTER_OTLP_ENDPOINT environment variable not set")?;

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

pub fn init_otlp() -> OtlpResult<OtlpLayers> {
    let tracing_layer = create_otlp_tracing_layer()?;
    let metrics_layer = create_otlp_metrics_layer()?;
    Ok((tracing_layer, metrics_layer))
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
    fn test_otlp_config_from_env() {
        let original_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        let original_timeout = env::var("OTEL_EXPORTER_OTLP_TIMEOUT").ok();

        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        assert!(OtlpConfig::from_env().is_none());

        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://test:4317");
        env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", "5000");

        let config = OtlpConfig::from_env().unwrap();
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
