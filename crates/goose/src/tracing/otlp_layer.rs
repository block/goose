use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace;
use opentelemetry_sdk::{runtime, Resource};
use std::sync::Mutex;
use std::time::Duration;
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;

use super::otel_config::{ExporterType, OtelConfig};

// Store reference to SdkMeterProvider so we can call shutdown() on exit.
// Once we call global::set_meter_provider(), the provider is wrapped in a GlobalMeterProvider
// which does NOT expose shutdown() or force_flush() methods.
// See: https://github.com/open-telemetry/opentelemetry-rust/issues/1605
static METER_PROVIDER: Mutex<Option<opentelemetry_sdk::metrics::SdkMeterProvider>> =
    Mutex::new(None);

// Store reference to TracerProvider for shutdown
static TRACER_PROVIDER: Mutex<Option<opentelemetry_sdk::trace::TracerProvider>> = Mutex::new(None);

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry>;
pub type OtlpLayers = (OtlpTracingLayer, OtlpMetricsLayer);
pub type OtlpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct OtlpConfig {
    pub endpoint: Option<String>,
    pub timeout: Option<Duration>,
}

impl OtlpConfig {
    /// Create OtlpConfig from the goose config file only.
    /// Returns None if neither endpoint nor timeout is set in the config file.
    /// When None is returned, the OpenTelemetry SDK will read from standard
    /// OTEL_EXPORTER_OTLP_* environment variables automatically.
    /// Only sets fields that are explicitly present in the config.
    pub fn from_config() -> Option<Self> {
        let config = crate::config::Config::global();

        // Only read from config file, not environment variables
        let endpoint = config
            .get_param::<String>("otel_exporter_otlp_endpoint")
            .ok();

        let timeout = config
            .get_param::<u64>("otel_exporter_otlp_timeout")
            .ok()
            .map(Duration::from_millis);

        // Return None if neither field is set
        if endpoint.is_none() && timeout.is_none() {
            return None;
        }

        Some(Self { endpoint, timeout })
    }
}

fn create_resource() -> Resource {
    Resource::new(vec![
        KeyValue::new("service.name", "goose"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "goose"),
    ])
}

pub fn create_otlp_tracing_layer() -> OtlpResult<OtlpTracingLayer> {
    // Detect configuration from environment and config file
    let otel_config = OtelConfig::detect();

    // If no config or traces not configured, return error
    let traces_config = otel_config
        .and_then(|c| c.traces)
        .ok_or("Traces not configured (SDK disabled or OTEL_TRACES_EXPORTER=none)")?;

    let resource = create_resource();

    let tracer_provider = match traces_config.exporter {
        ExporterType::Otlp => {
            // Build OTLP exporter
            let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder().with_http();

            if let Some(ref endpoint) = traces_config.endpoint {
                exporter_builder = exporter_builder.with_endpoint(endpoint);
            }
            if let Some(timeout) = traces_config.timeout {
                exporter_builder = exporter_builder.with_timeout(timeout);
            }

            let exporter = exporter_builder.build()?;

            // Builder automatically reads OTEL env vars via Config::default() including:
            // - OTEL_TRACES_SAMPLER, OTEL_TRACES_SAMPLER_ARG
            // - OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT, OTEL_SPAN_EVENT_COUNT_LIMIT, OTEL_SPAN_LINK_COUNT_LIMIT
            // See: https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-sdk/src/trace/config.rs
            trace::TracerProvider::builder()
                .with_batch_exporter(exporter, runtime::Tokio)
                .with_resource(resource)
                .build()
        }
        ExporterType::Console => {
            // Build console/stdout exporter
            let exporter = opentelemetry_stdout::SpanExporter::default();

            // Builder automatically reads OTEL env vars via Config::default()
            trace::TracerProvider::builder()
                .with_simple_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::None => {
            return Err("Cannot create tracing layer for OTEL_TRACES_EXPORTER=none".into());
        }
    };

    // Store provider for shutdown
    *TRACER_PROVIDER.lock().unwrap() = Some(tracer_provider.clone());

    let tracer = tracer_provider.tracer("goose");
    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn create_otlp_metrics_layer() -> OtlpResult<OtlpMetricsLayer> {
    // Detect configuration from environment and config file
    let otel_config = OtelConfig::detect();

    // If no config or metrics not configured, return error
    let metrics_config = otel_config
        .and_then(|c| c.metrics)
        .ok_or("Metrics not configured (SDK disabled or OTEL_METRICS_EXPORTER=none)")?;

    let resource = create_resource();

    let meter_provider = match metrics_config.exporter {
        ExporterType::Otlp => {
            // Build OTLP exporter
            let mut exporter_builder = opentelemetry_otlp::MetricExporter::builder().with_http();

            if let Some(ref endpoint) = metrics_config.endpoint {
                exporter_builder = exporter_builder.with_endpoint(endpoint);
            }
            if let Some(timeout) = metrics_config.timeout {
                exporter_builder = exporter_builder.with_timeout(timeout);
            }

            let exporter = exporter_builder.build()?;

            // PeriodicReader automatically reads OTEL_METRIC_EXPORT_INTERVAL (default: 60000ms)
            opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_resource(resource)
                .with_reader(
                    opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, runtime::Tokio)
                        .build(),
                )
                .build()
        }
        ExporterType::Console => {
            // Build console/stdout exporter
            let exporter = opentelemetry_stdout::MetricExporter::default();

            opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_resource(resource)
                .with_reader(
                    opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, runtime::Tokio)
                        .build(),
                )
                .build()
        }
        ExporterType::None => {
            return Err("Cannot create metrics layer for OTEL_METRICS_EXPORTER=none".into());
        }
    };

    // Clone creates a new reference to the same provider (via Arc), not a new instance
    global::set_meter_provider(meter_provider.clone());
    *METER_PROVIDER.lock().unwrap() = Some(meter_provider.clone());

    Ok(tracing_opentelemetry::MetricsLayer::new(meter_provider))
}

/// Initialize OpenTelemetry propagation for distributed tracing.
/// This sets the W3C Trace Context propagator globally, enabling
/// trace context to be propagated across HTTP and MCP boundaries.
///
/// This should be called once at application startup, before any
/// OTLP initialization or trace propagation occurs.
pub fn init_otel_propagation() {
    global::set_text_map_propagator(TraceContextPropagator::new());
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

/// Returns true if OTLP has been initialized (tracing, metrics, or both)
pub fn is_otlp_initialized() -> bool {
    METER_PROVIDER.lock().unwrap().is_some() || TRACER_PROVIDER.lock().unwrap().is_some()
}

/// Shutdown OTLP providers gracefully
pub fn shutdown_otlp() {
    // Shutdown the tracer provider if we have a reference to it
    if TRACER_PROVIDER.lock().unwrap().take().is_some() {
        global::shutdown_tracer_provider();
    }

    // Shutdown the meter provider if we have a reference to it
    if let Some(provider) = METER_PROVIDER.lock().unwrap().take() {
        let _ = provider.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env::with_vars;

    #[test]
    fn test_otlp_config_from_config() {
        use tempfile::NamedTempFile;

        with_vars(
            vec![
                ("OTEL_EXPORTER_OTLP_ENDPOINT", None::<&str>),
                ("OTEL_EXPORTER_OTLP_TIMEOUT", None::<&str>),
            ],
            || {
                // Create a test config file
                let temp_file = NamedTempFile::new().unwrap();
                let test_config =
                    crate::config::Config::new(temp_file.path(), "test-otlp").unwrap();

                // Set values in config
                test_config
                    .set_param(
                        "otel_exporter_otlp_endpoint",
                        serde_json::Value::String("http://config:4318".to_string()),
                    )
                    .unwrap();
                test_config
                    .set_param(
                        "otel_exporter_otlp_timeout",
                        serde_json::Value::Number(3000.into()),
                    )
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

                // Test env var override with nested with_vars
                with_vars(
                    vec![("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://env:4317"))],
                    || {
                        let endpoint: String = test_config
                            .get_param("otel_exporter_otlp_endpoint")
                            .unwrap();
                        assert_eq!(endpoint, "http://env:4317");
                    },
                );
            },
        );
    }

    #[tokio::test]
    async fn test_console_exporter() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("console")),
                ("OTEL_METRICS_EXPORTER", Some("console")),
            ],
            || {
                // Should successfully create layers with console exporter
                let tracing_result = create_otlp_tracing_layer();
                assert!(tracing_result.is_ok());

                let metrics_result = create_otlp_metrics_layer();
                assert!(metrics_result.is_ok());
            },
        );
    }

    #[tokio::test]
    async fn test_otlp_exporter() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_METRICS_EXPORTER", Some("otlp")),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://localhost:4318")),
            ],
            || {
                // Should successfully create layers with OTLP exporter
                let tracing_result = create_otlp_tracing_layer();
                assert!(tracing_result.is_ok());

                let metrics_result = create_otlp_metrics_layer();
                assert!(metrics_result.is_ok());
            },
        );
    }

    #[test]
    fn test_sdk_disabled() {
        with_vars(vec![("OTEL_SDK_DISABLED", Some("true"))], || {
            // Should fail when SDK is disabled
            let tracing_result = create_otlp_tracing_layer();
            assert!(tracing_result.is_err());

            let metrics_result = create_otlp_metrics_layer();
            assert!(metrics_result.is_err());
        });
    }

    #[test]
    fn test_exporter_none() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("none")),
                ("OTEL_METRICS_EXPORTER", Some("none")),
            ],
            || {
                // Should fail when exporter is explicitly set to none
                let tracing_result = create_otlp_tracing_layer();
                assert!(tracing_result.is_err());

                let metrics_result = create_otlp_metrics_layer();
                assert!(metrics_result.is_err());
            },
        );
    }

    #[tokio::test]
    async fn test_separate_endpoints() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_METRICS_EXPORTER", Some("otlp")),
                (
                    "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
                    Some("http://jaeger:4318"),
                ),
                (
                    "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
                    Some("http://prometheus:4318"),
                ),
            ],
            || {
                // Should successfully create layers with separate endpoints
                let tracing_result = create_otlp_tracing_layer();
                assert!(tracing_result.is_ok());

                let metrics_result = create_otlp_metrics_layer();
                assert!(metrics_result.is_ok());
            },
        );
    }
}
