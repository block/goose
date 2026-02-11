use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::{EnvResourceDetector, TelemetryResourceDetector};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::sync::Mutex;
use std::time::Duration;
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;

use super::otel_config::{ExporterType, OtelConfig};

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry, SdkMeterProvider>;
pub type OtlpLogsLayer = OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>;
pub type OtlpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

static TRACER_PROVIDER: Mutex<Option<SdkTracerProvider>> = Mutex::new(None);
static METER_PROVIDER: Mutex<Option<SdkMeterProvider>> = Mutex::new(None);
static LOGGER_PROVIDER: Mutex<Option<SdkLoggerProvider>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct OtlpConfig {
    pub endpoint: Option<String>,
    pub timeout: Option<Duration>,
}

impl OtlpConfig {
    pub fn from_config() -> Option<Self> {
        let config = crate::config::Config::global();

        let endpoint = config
            .get_param::<String>("otel_exporter_otlp_endpoint")
            .ok();

        let timeout = config
            .get_param::<u64>("otel_exporter_otlp_timeout")
            .ok()
            .map(Duration::from_millis);

        if endpoint.is_none() && timeout.is_none() {
            return None;
        }

        Some(Self { endpoint, timeout })
    }
}

fn create_resource() -> Resource {
    let mut builder = Resource::builder_empty()
        .with_attributes([
            KeyValue::new("service.name", "goose"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("service.namespace", "goose"),
        ])
        .with_detector(Box::new(EnvResourceDetector::new()))
        .with_detector(Box::new(TelemetryResourceDetector));

    // OTEL_SERVICE_NAME takes highest priority (skip SdkProvidedResourceDetector
    // which would fall back to "unknown_service" when unset)
    if let Ok(name) = std::env::var("OTEL_SERVICE_NAME") {
        if !name.is_empty() {
            builder = builder.with_service_name(name);
        }
    }
    builder.build()
}

pub fn create_otlp_tracing_layer() -> OtlpResult<OtlpTracingLayer> {
    let otel_config = OtelConfig::detect().ok_or("OTel not configured")?;
    let signal = otel_config.traces.ok_or("Traces not enabled")?;
    let resource = create_resource();
    let config_file = OtlpConfig::from_config();

    let tracer_provider = match signal.exporter {
        ExporterType::Otlp => {
            let mut builder = opentelemetry_otlp::SpanExporter::builder().with_http();
            if let Some(ref cfg) = config_file {
                if let Some(ref endpoint) = cfg.endpoint {
                    builder = builder.with_endpoint(endpoint);
                }
                if let Some(timeout) = cfg.timeout {
                    builder = builder.with_timeout(timeout);
                }
            }
            let exporter = builder.build()?;
            SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::Console => {
            let exporter = opentelemetry_stdout::SpanExporter::default();
            SdkTracerProvider::builder()
                .with_simple_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::None => return Err("Traces exporter set to none".into()),
    };

    let tracer = tracer_provider.tracer("goose");
    *TRACER_PROVIDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tracer_provider);

    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn create_otlp_metrics_layer() -> OtlpResult<OtlpMetricsLayer> {
    let otel_config = OtelConfig::detect().ok_or("OTel not configured")?;
    let signal = otel_config.metrics.ok_or("Metrics not enabled")?;
    let resource = create_resource();
    let config_file = OtlpConfig::from_config();

    let meter_provider = match signal.exporter {
        ExporterType::Otlp => {
            let mut builder = opentelemetry_otlp::MetricExporter::builder().with_http();
            if let Some(ref cfg) = config_file {
                if let Some(ref endpoint) = cfg.endpoint {
                    builder = builder.with_endpoint(endpoint);
                }
                if let Some(timeout) = cfg.timeout {
                    builder = builder.with_timeout(timeout);
                }
            }
            let exporter = builder.build()?;
            SdkMeterProvider::builder()
                .with_resource(resource)
                .with_periodic_exporter(exporter)
                .build()
        }
        ExporterType::Console => {
            let exporter = opentelemetry_stdout::MetricExporter::default();
            SdkMeterProvider::builder()
                .with_resource(resource)
                .with_periodic_exporter(exporter)
                .build()
        }
        ExporterType::None => return Err("Metrics exporter set to none".into()),
    };

    global::set_meter_provider(meter_provider.clone());
    *METER_PROVIDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(meter_provider.clone());

    Ok(MetricsLayer::new(meter_provider))
}

pub fn create_otlp_logs_layer() -> OtlpResult<OtlpLogsLayer> {
    let otel_config = OtelConfig::detect().ok_or("OTel not configured")?;
    let signal = otel_config.logs.ok_or("Logs not enabled")?;
    let resource = create_resource();
    let config_file = OtlpConfig::from_config();

    let logger_provider = match signal.exporter {
        ExporterType::Otlp => {
            let mut builder = opentelemetry_otlp::LogExporter::builder().with_http();
            if let Some(ref cfg) = config_file {
                if let Some(ref endpoint) = cfg.endpoint {
                    builder = builder.with_endpoint(endpoint);
                }
                if let Some(timeout) = cfg.timeout {
                    builder = builder.with_timeout(timeout);
                }
            }
            let exporter = builder.build()?;
            SdkLoggerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::Console => {
            let exporter = opentelemetry_stdout::LogExporter::default();
            SdkLoggerProvider::builder()
                .with_simple_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::None => return Err("Logs exporter set to none".into()),
    };

    let bridge = OpenTelemetryTracingBridge::new(&logger_provider);
    *LOGGER_PROVIDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(logger_provider);

    Ok(bridge)
}

pub fn init_otel_propagation() {
    global::set_text_map_propagator(TraceContextPropagator::new());
}

pub fn is_otlp_initialized() -> bool {
    TRACER_PROVIDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
        || METER_PROVIDER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        || LOGGER_PROVIDER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
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

/// Creates a custom filter for OTLP logs that captures:
/// - All events at WARN level and above
pub fn create_otlp_logs_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata: &Metadata<'_>| metadata.level() <= &Level::WARN)
}

/// Shutdown OTLP providers gracefully
pub fn shutdown_otlp() {
    if let Some(provider) = TRACER_PROVIDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        let _ = provider.shutdown();
    }
    if let Some(provider) = METER_PROVIDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        let _ = provider.shutdown();
    }
    if let Some(provider) = LOGGER_PROVIDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        let _ = provider.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    #[test]
    fn test_otlp_config_from_config() {
        let _guard = env_lock::lock_env([
            ("OTEL_EXPORTER_OTLP_ENDPOINT", None::<&str>),
            ("OTEL_EXPORTER_OTLP_TIMEOUT", None::<&str>),
        ]);

        let temp_file = NamedTempFile::new().unwrap();
        let test_config = crate::config::Config::new(temp_file.path(), "test-otlp").unwrap();

        test_config
            .set_param("otel_exporter_otlp_endpoint", "http://config:4318")
            .unwrap();
        test_config
            .set_param("otel_exporter_otlp_timeout", 3000)
            .unwrap();

        let endpoint: String = test_config
            .get_param("otel_exporter_otlp_endpoint")
            .unwrap();
        assert_eq!(endpoint, "http://config:4318");

        let timeout: u64 = test_config.get_param("otel_exporter_otlp_timeout").unwrap();
        assert_eq!(timeout, 3000);
    }

    #[test_case(Some("true"), None, None, None, None; "sdk disabled")]
    #[test_case(None, Some("none"), Some("none"), Some("none"), None; "all none")]
    fn test_all_layers_err(
        sdk_disabled: Option<&str>,
        traces: Option<&str>,
        metrics: Option<&str>,
        logs: Option<&str>,
        _endpoint: Option<&str>,
    ) {
        let _guard = env_lock::lock_env([
            ("OTEL_SDK_DISABLED", sdk_disabled),
            ("OTEL_TRACES_EXPORTER", traces),
            ("OTEL_METRICS_EXPORTER", metrics),
            ("OTEL_LOGS_EXPORTER", logs),
            ("OTEL_EXPORTER_OTLP_ENDPOINT", _endpoint),
        ]);
        assert!(create_otlp_tracing_layer().is_err());
        assert!(create_otlp_metrics_layer().is_err());
        assert!(create_otlp_logs_layer().is_err());
    }

    #[test_case("console"; "console")]
    #[test_case("otlp"; "otlp")]
    fn test_all_layers_ok(exporter: &str) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let _env = env_lock::lock_env([
            ("OTEL_SDK_DISABLED", None::<&str>),
            ("OTEL_TRACES_EXPORTER", Some(exporter)),
            ("OTEL_METRICS_EXPORTER", Some(exporter)),
            ("OTEL_LOGS_EXPORTER", Some(exporter)),
            ("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://localhost:4318")),
        ]);
        assert!(create_otlp_tracing_layer().is_ok());
        assert!(create_otlp_metrics_layer().is_ok());
        assert!(create_otlp_logs_layer().is_ok());
        shutdown_otlp();
    }

    #[test_case(
        vec![("OTEL_SERVICE_NAME", None), ("OTEL_RESOURCE_ATTRIBUTES", None)],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .build();
        "no env vars uses goose defaults"
    )]
    #[test_case(
        vec![("OTEL_SERVICE_NAME", Some("custom")), ("OTEL_RESOURCE_ATTRIBUTES", None)],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_service_name("custom")
            .build();
        "OTEL_SERVICE_NAME overrides service.name"
    )]
    #[test_case(
        vec![("OTEL_SERVICE_NAME", None), ("OTEL_RESOURCE_ATTRIBUTES", Some("deployment.environment=prod"))],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_attribute(KeyValue::new("deployment.environment", "prod"))
            .build();
        "OTEL_RESOURCE_ATTRIBUTES adds custom attributes"
    )]
    #[test_case(
        vec![("OTEL_SERVICE_NAME", Some("custom")), ("OTEL_RESOURCE_ATTRIBUTES", Some("deployment.environment=prod"))],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_service_name("custom")
            .with_attribute(KeyValue::new("deployment.environment", "prod"))
            .build();
        "OTEL_SERVICE_NAME and OTEL_RESOURCE_ATTRIBUTES combine"
    )]
    fn test_create_resource(env: Vec<(&str, Option<&str>)>, expected: Resource) {
        let _guard = env_lock::lock_env(env);
        assert_eq!(create_resource(), expected);
    }
}
