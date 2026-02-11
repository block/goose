use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::{EnvResourceDetector, TelemetryResourceDetector};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::sync::Mutex;
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;

use super::otel_config::{signal_exporter, ExporterType};

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry, SdkMeterProvider>;
pub type OtlpLogsLayer = OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>;
pub type OtlpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

static TRACER_PROVIDER: Mutex<Option<SdkTracerProvider>> = Mutex::new(None);
static METER_PROVIDER: Mutex<Option<SdkMeterProvider>> = Mutex::new(None);
static LOGGER_PROVIDER: Mutex<Option<SdkLoggerProvider>> = Mutex::new(None);

/// Promotes goose config-file OTel settings to env vars so the SDK handles
/// endpoint resolution correctly (e.g. appending /v1/traces to the base URL).
///
/// This must be called before creating any exporters. The SDK reads env vars
/// during exporter build, so we set them here rather than using programmatic
/// `with_endpoint()` which bypasses the SDK's signal-path appending logic.
pub fn promote_config_to_env() {
    let config = crate::config::Config::global();
    let endpoint = config
        .get_param::<String>("otel_exporter_otlp_endpoint")
        .ok();
    let timeout = config
        .get_param::<u64>("otel_exporter_otlp_timeout")
        .ok()
        .map(|t| t.to_string());
    promote_to_env(endpoint.as_deref(), timeout.as_deref());
}

fn promote_to_env(endpoint: Option<&str>, timeout: Option<&str>) {
    if let Some(endpoint) = endpoint {
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
            std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint);
        }
    }
    if let Some(timeout) = timeout {
        if std::env::var("OTEL_EXPORTER_OTLP_TIMEOUT").is_err() {
            std::env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", timeout);
        }
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
    let exporter = signal_exporter("traces").ok_or("Traces not enabled")?;
    let resource = create_resource();

    let tracer_provider = match exporter {
        ExporterType::Otlp => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .build()?;
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

    global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer("goose");
    *TRACER_PROVIDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tracer_provider);

    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn create_otlp_metrics_layer() -> OtlpResult<OtlpMetricsLayer> {
    let exporter = signal_exporter("metrics").ok_or("Metrics not enabled")?;
    let resource = create_resource();

    let meter_provider = match exporter {
        ExporterType::Otlp => {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .build()?;
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
    let exporter = signal_exporter("logs").ok_or("Logs not enabled")?;
    let resource = create_resource();

    let logger_provider = match exporter {
        ExporterType::Otlp => {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_http()
                .build()?;
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
    use super::super::otel_config::clear_otel_env;
    use super::*;
    use test_case::test_case;

    #[test_case("console"; "console")]
    #[test_case("otlp"; "otlp")]
    fn test_all_layers_ok(exporter: &str) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let _env = clear_otel_env(&[
            ("OTEL_TRACES_EXPORTER", exporter),
            ("OTEL_METRICS_EXPORTER", exporter),
            ("OTEL_LOGS_EXPORTER", exporter),
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318"),
        ]);
        assert!(create_otlp_tracing_layer().is_ok());
        assert!(create_otlp_metrics_layer().is_ok());
        assert!(create_otlp_logs_layer().is_ok());
        shutdown_otlp();
    }

    #[test_case(
        &[],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .build();
        "no env vars uses goose defaults"
    )]
    #[test_case(
        &[("OTEL_SERVICE_NAME", "custom")],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_service_name("custom")
            .build();
        "OTEL_SERVICE_NAME overrides service.name"
    )]
    #[test_case(
        &[("OTEL_RESOURCE_ATTRIBUTES", "deployment.environment=prod")],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_attribute(KeyValue::new("deployment.environment", "prod"))
            .build();
        "OTEL_RESOURCE_ATTRIBUTES adds custom attributes"
    )]
    #[test_case(
        &[("OTEL_SERVICE_NAME", "custom"), ("OTEL_RESOURCE_ATTRIBUTES", "deployment.environment=prod")],
        Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "goose"), KeyValue::new("service.version", env!("CARGO_PKG_VERSION")), KeyValue::new("service.namespace", "goose")])
            .with_detector(Box::new(TelemetryResourceDetector))
            .with_service_name("custom")
            .with_attribute(KeyValue::new("deployment.environment", "prod"))
            .build();
        "OTEL_SERVICE_NAME and OTEL_RESOURCE_ATTRIBUTES combine"
    )]
    fn test_create_resource(env: &[(&str, &str)], expected: Resource) {
        let _guard = clear_otel_env(env);
        assert_eq!(create_resource(), expected);
    }

    #[test_case(
        &[],
        Some("http://config:4318"), Some("5000"),
        Some("http://config:4318"), Some("5000");
        "config promotes to env when unset"
    )]
    #[test_case(
        &[("OTEL_EXPORTER_OTLP_ENDPOINT", "http://env:4318"), ("OTEL_EXPORTER_OTLP_TIMEOUT", "3000")],
        Some("http://config:4318"), Some("5000"),
        Some("http://env:4318"), Some("3000");
        "env var takes precedence over config"
    )]
    #[test_case(
        &[],
        None, None,
        None, None;
        "no config leaves env unset"
    )]
    fn test_promote_to_env(
        env: &[(&str, &str)],
        cfg_endpoint: Option<&str>,
        cfg_timeout: Option<&str>,
        expect_endpoint: Option<&str>,
        expect_timeout: Option<&str>,
    ) {
        let _guard = clear_otel_env(env);
        promote_to_env(cfg_endpoint, cfg_timeout);
        assert_eq!(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok().as_deref(),
            expect_endpoint
        );
        assert_eq!(
            std::env::var("OTEL_EXPORTER_OTLP_TIMEOUT").ok().as_deref(),
            expect_timeout
        );
    }
}
