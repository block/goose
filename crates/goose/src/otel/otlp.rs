use opentelemetry::logs::{AnyValue, LogRecord};
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, Key, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::logs::{LogProcessor, SdkLogRecord, SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::metrics::{SdkMeterProvider, Temporality};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::{EnvResourceDetector, TelemetryResourceDetector};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::cell::RefCell;
use std::env;
use std::sync::Mutex;
use tracing::field::{Field, Visit};
use tracing::{Level, Metadata};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer as _;

pub type OtlpTracingLayer =
    OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>;
pub type OtlpMetricsLayer = MetricsLayer<tracing_subscriber::Registry, SdkMeterProvider>;
pub type OtlpLogsLayer = OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>;
pub type OtlpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

static TRACER_PROVIDER: Mutex<Option<SdkTracerProvider>> = Mutex::new(None);
static METER_PROVIDER: Mutex<Option<SdkMeterProvider>> = Mutex::new(None);
static LOGGER_PROVIDER: Mutex<Option<SdkLoggerProvider>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq)]
pub enum ExporterType {
    Otlp,
    Console,
    None,
}

impl ExporterType {
    pub fn from_env_value(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "" | "otlp" => ExporterType::Otlp,
            "console" | "stdout" => ExporterType::Console,
            _ => ExporterType::None,
        }
    }
}

/// Returns the exporter type for a signal, or None if disabled.
///
/// Checks in order:
/// 1. OTEL_SDK_DISABLED — disables everything
/// 2. OTEL_{SIGNAL}_EXPORTER — explicit exporter selection ("none" disables)
/// 3. OTEL_EXPORTER_OTLP_{SIGNAL}_ENDPOINT or OTEL_EXPORTER_OTLP_ENDPOINT — enables OTLP
pub fn signal_exporter(signal: &str) -> Option<ExporterType> {
    if env::var("OTEL_SDK_DISABLED")
        .ok()
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
    {
        return None;
    }

    let exporter_var = format!("OTEL_{}_EXPORTER", signal.to_uppercase());
    if let Ok(val) = env::var(&exporter_var) {
        let typ = ExporterType::from_env_value(&val);
        return if matches!(typ, ExporterType::None) {
            None
        } else {
            Some(typ)
        };
    }

    let signal_endpoint = format!("OTEL_EXPORTER_OTLP_{}_ENDPOINT", signal.to_uppercase());
    let has_endpoint = env::var(&signal_endpoint)
        .ok()
        .is_some_and(|v| !v.is_empty())
        || env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .is_some_and(|v| !v.is_empty());

    if has_endpoint {
        Some(ExporterType::Otlp)
    } else {
        None
    }
}

/// Promotes goose config-file OTel settings to env vars before exporter build.
pub fn promote_config_to_env(config: &crate::config::Config) {
    if env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
        if let Ok(endpoint) = config.get_param::<String>("otel_exporter_otlp_endpoint") {
            env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint);
        }
    }
    if env::var("OTEL_EXPORTER_OTLP_TIMEOUT").is_err() {
        if let Ok(timeout) = config.get_param::<u64>("otel_exporter_otlp_timeout") {
            env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", timeout.to_string());
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

// Propagates session.id from tracing spans to OTel log records via a thread-local,
// similar to opentelemetry-appender-tracing's experimental_span_attributes feature.
// Remove once https://github.com/open-telemetry/opentelemetry-rust/pull/3387 is released.
// SessionIdBridge must be inner to the bridge layer so its on_event fires first.

thread_local! {
    static CURRENT_SESSION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct SessionId(String);

#[derive(Debug)]
pub struct SessionIdBridge;

impl<S> tracing_subscriber::Layer<S> for SessionIdBridge
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        struct V(Option<String>);
        impl Visit for V {
            fn record_str(&mut self, field: &Field, value: &str) {
                if field.name() == "session.id" {
                    self.0 = Some(value.to_string());
                }
            }
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "session.id" {
                    self.0 = Some(format!("{value:?}"));
                }
            }
        }
        let mut v = V(None);
        attrs.record(&mut v);
        if let Some(sid) = v.0 {
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SessionId(sid));
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let sid = ctx.event_scope(event).and_then(|scope| {
            scope
                .from_root()
                .find_map(|span| span.extensions().get::<SessionId>().map(|s| s.0.clone()))
        });
        CURRENT_SESSION_ID.with(|cell| *cell.borrow_mut() = sid);
    }
}

impl LogProcessor for SessionIdBridge {
    fn emit(&self, record: &mut SdkLogRecord, _: &opentelemetry::InstrumentationScope) {
        CURRENT_SESSION_ID.with(|cell| {
            if let Some(ref id) = *cell.borrow() {
                record.add_attribute(Key::new("session.id"), AnyValue::from(id.clone()));
            }
        });
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }
}

/// Initializes all OTLP signal layers (traces, metrics, logs) and propagation.
/// Returns boxed layers ready to add to a subscriber.
pub fn init_otlp_layers(
    config: &crate::config::Config,
) -> Vec<Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>> {
    promote_config_to_env(config);

    let mut layers: Vec<
        Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>,
    > = Vec::new();

    if let Ok(layer) = create_otlp_tracing_layer() {
        layers.push(layer.with_filter(create_otlp_tracing_filter()).boxed());
    }
    if let Ok(layer) = create_otlp_metrics_layer() {
        layers.push(layer.with_filter(create_otlp_metrics_filter()).boxed());
    }
    if let Ok(bridge) = create_otlp_logs_layer() {
        // SessionIdBridge must be first so on_event fires before bridge in Vec iteration
        layers.push(SessionIdBridge.boxed());
        layers.push(bridge.with_filter(create_otlp_logs_filter()).boxed());
    }

    if !layers.is_empty() {
        global::set_text_map_propagator(TraceContextPropagator::new());
    }

    layers
}

fn create_otlp_tracing_layer() -> OtlpResult<OtlpTracingLayer> {
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

// TODO: remove once https://github.com/open-telemetry/opentelemetry-rust/pull/3351 is released.
fn temporality_preference() -> Temporality {
    match env::var("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "delta" => Temporality::Delta,
        "lowmemory" => Temporality::LowMemory,
        _ => Temporality::Cumulative,
    }
}

fn create_otlp_metrics_layer() -> OtlpResult<OtlpMetricsLayer> {
    let exporter = signal_exporter("metrics").ok_or("Metrics not enabled")?;
    let resource = create_resource();

    let meter_provider = match exporter {
        ExporterType::Otlp => {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_temporality(temporality_preference())
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

fn create_otlp_logs_layer() -> OtlpResult<OtlpLogsLayer> {
    let exporter = signal_exporter("logs").ok_or("Logs not enabled")?;
    let resource = create_resource();

    let logger_provider = match exporter {
        ExporterType::Otlp => {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_http()
                .build()?;
            SdkLoggerProvider::builder()
                .with_log_processor(SessionIdBridge)
                .with_batch_exporter(exporter)
                .with_resource(resource)
                .build()
        }
        ExporterType::Console => {
            let exporter = opentelemetry_stdout::LogExporter::default();
            SdkLoggerProvider::builder()
                .with_log_processor(SessionIdBridge)
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
fn create_otlp_tracing_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
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
fn create_otlp_metrics_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
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

fn parse_level(s: &str) -> Option<Level> {
    match s.to_lowercase().as_str() {
        "trace" => Some(Level::TRACE),
        "debug" => Some(Level::DEBUG),
        "info" => Some(Level::INFO),
        "warn" => Some(Level::WARN),
        "error" => Some(Level::ERROR),
        _ => None,
    }
}

fn otel_logs_level() -> Level {
    env::var("RUST_LOG")
        .ok()
        .and_then(|s| parse_level(&s))
        .or_else(|| {
            env::var("OTEL_LOG_LEVEL")
                .ok()
                .and_then(|s| parse_level(&s))
        })
        .unwrap_or(Level::INFO)
}

/// Creates a custom filter for OTLP logs.
/// Level is resolved via RUST_LOG → OTEL_LOG_LEVEL → default INFO.
fn create_otlp_logs_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    let min_level = otel_logs_level();
    FilterFn::new(move |metadata: &Metadata<'_>| metadata.level() <= &min_level)
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
    use goose_test_support::otel::clear_otel_env;
    use opentelemetry_sdk::metrics::Temporality;
    use test_case::test_case;

    #[test]
    fn exporter_type_from_env_value() {
        assert_eq!(ExporterType::from_env_value("otlp"), ExporterType::Otlp);
        assert_eq!(ExporterType::from_env_value("OTLP"), ExporterType::Otlp);
        assert_eq!(ExporterType::from_env_value(""), ExporterType::Otlp);
        assert_eq!(
            ExporterType::from_env_value("console"),
            ExporterType::Console
        );
        assert_eq!(
            ExporterType::from_env_value("stdout"),
            ExporterType::Console
        );
        assert_eq!(ExporterType::from_env_value("none"), ExporterType::None);
        assert_eq!(ExporterType::from_env_value("NONE"), ExporterType::None);
        assert_eq!(ExporterType::from_env_value("unknown"), ExporterType::None);
    }

    #[test_case(&[("OTEL_SDK_DISABLED", "true")]; "OTEL_SDK_DISABLED disables all signals")]
    #[test_case(&[]; "no env vars returns None")]
    fn signal_exporter_disabled(env: &[(&'static str, &'static str)]) {
        let _guard = clear_otel_env(env);
        assert!(signal_exporter("traces").is_none());
        assert!(signal_exporter("metrics").is_none());
        assert!(signal_exporter("logs").is_none());
    }

    #[test_case("traces",  &[("OTEL_TRACES_EXPORTER", "console")], Some(ExporterType::Console); "OTEL_TRACES_EXPORTER=console")]
    #[test_case("traces",  &[("OTEL_TRACES_EXPORTER", "none")],    None;                        "OTEL_TRACES_EXPORTER=none")]
    #[test_case("traces",  &[("OTEL_TRACES_EXPORTER", "otlp")],    Some(ExporterType::Otlp);    "OTEL_TRACES_EXPORTER=otlp")]
    #[test_case("metrics", &[("OTEL_METRICS_EXPORTER", "console")], Some(ExporterType::Console); "OTEL_METRICS_EXPORTER=console")]
    #[test_case("logs",    &[("OTEL_LOGS_EXPORTER", "none")],       None;                        "OTEL_LOGS_EXPORTER=none")]
    fn signal_exporter_by_var(
        signal: &str,
        env: &[(&'static str, &'static str)],
        expected: Option<ExporterType>,
    ) {
        let _guard = clear_otel_env(env);
        assert_eq!(signal_exporter(signal), expected);
    }

    #[test_case("traces",  &[("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318")],        Some(ExporterType::Otlp); "generic endpoint enables traces")]
    #[test_case("traces",  &[("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", "http://localhost:4318")],  Some(ExporterType::Otlp); "signal-specific endpoint enables traces")]
    #[test_case("metrics", &[("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT", "http://localhost:4318")], Some(ExporterType::Otlp); "signal-specific endpoint enables metrics")]
    #[test_case("traces",  &[("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT", "http://localhost:4318")], None;                     "metrics endpoint does not enable traces")]
    #[test_case("traces",  &[("OTEL_TRACES_EXPORTER", "none"), ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318")], None; "OTEL_TRACES_EXPORTER=none overrides endpoint")]
    #[test_case("traces",  &[("OTEL_EXPORTER_OTLP_ENDPOINT", "")],                              None;                     "empty endpoint returns None")]
    fn signal_exporter_endpoints(
        signal: &str,
        env: &[(&'static str, &'static str)],
        expected: Option<ExporterType>,
    ) {
        let _guard = clear_otel_env(env);
        assert_eq!(signal_exporter(signal), expected);
    }

    #[test_case("console"; "console")]
    #[test_case("otlp"; "otlp")]
    fn test_all_layers_ok(exporter: &'static str) {
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
    fn test_create_resource(env: &[(&'static str, &'static str)], expected: Resource) {
        let _guard = clear_otel_env(env);
        assert_eq!(create_resource(), expected);
    }

    #[test_case(&[("RUST_LOG", "")], Level::INFO; "default is info")]
    #[test_case(&[("RUST_LOG", "debug")], Level::DEBUG; "RUST_LOG takes precedence")]
    #[test_case(&[("RUST_LOG", ""), ("OTEL_LOG_LEVEL", "error")], Level::ERROR; "OTEL_LOG_LEVEL fallback")]
    #[test_case(&[("RUST_LOG", "warn"), ("OTEL_LOG_LEVEL", "error")], Level::WARN; "RUST_LOG wins over OTEL_LOG_LEVEL")]
    #[test_case(&[("RUST_LOG", "goose=debug"), ("OTEL_LOG_LEVEL", "trace")], Level::TRACE; "directive RUST_LOG falls through to OTEL_LOG_LEVEL")]
    #[test_case(&[("RUST_LOG", "goose=debug")], Level::INFO; "directive RUST_LOG falls through to default")]
    #[test_case(&[("RUST_LOG", ""), ("OTEL_LOG_LEVEL", "INFO")], Level::INFO; "case insensitive")]
    #[test_case(&[("RUST_LOG", ""), ("OTEL_LOG_LEVEL", "bogus")], Level::INFO; "unknown defaults to info")]
    fn otel_logs_level_from_env(env: &[(&'static str, &'static str)], expected: Level) {
        let _guard = clear_otel_env(env);
        assert_eq!(otel_logs_level(), expected);
    }

    fn test_config(
        params: &[(&str, &str)],
    ) -> (
        crate::config::Config,
        tempfile::NamedTempFile,
        tempfile::NamedTempFile,
    ) {
        let config_file = tempfile::NamedTempFile::new().unwrap();
        let secrets_file = tempfile::NamedTempFile::new().unwrap();
        let yaml: String = params.iter().map(|(k, v)| format!("{k}: {v}\n")).collect();
        std::fs::write(config_file.path(), yaml).unwrap();
        let config =
            crate::config::Config::new_with_file_secrets(config_file.path(), secrets_file.path())
                .unwrap();
        (config, config_file, secrets_file)
    }

    #[test_case(
        &[],
        &[("otel_exporter_otlp_endpoint", "http://config:4318"), ("otel_exporter_otlp_timeout", "5000")],
        Some("http://config:4318"), Some("5000");
        "config promotes to env when unset"
    )]
    #[test_case(
        &[("OTEL_EXPORTER_OTLP_ENDPOINT", "http://env:4318"), ("OTEL_EXPORTER_OTLP_TIMEOUT", "3000")],
        &[("otel_exporter_otlp_endpoint", "http://config:4318"), ("otel_exporter_otlp_timeout", "5000")],
        Some("http://env:4318"), Some("3000");
        "env var takes precedence over config"
    )]
    #[test_case(
        &[],
        &[],
        None, None;
        "no config leaves env unset"
    )]
    fn test_promote_config_to_env(
        env_overrides: &[(&'static str, &'static str)],
        cfg: &[(&str, &str)],
        expect_endpoint: Option<&str>,
        expect_timeout: Option<&str>,
    ) {
        let _guard = clear_otel_env(env_overrides);
        let (config, _cf, _sf) = test_config(cfg);

        promote_config_to_env(&config);

        assert_eq!(
            env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok().as_deref(),
            expect_endpoint
        );
        assert_eq!(
            env::var("OTEL_EXPORTER_OTLP_TIMEOUT").ok().as_deref(),
            expect_timeout
        );
    }

    #[test_case(&[], Temporality::Cumulative; "default is cumulative")]
    #[test_case(&[("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "delta")], Temporality::Delta; "delta")]
    #[test_case(&[("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "Delta")], Temporality::Delta; "Delta mixed case")]
    #[test_case(&[("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "lowmemory")], Temporality::LowMemory; "lowmemory")]
    #[test_case(&[("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "cumulative")], Temporality::Cumulative; "cumulative")]
    #[test_case(&[("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "bogus")], Temporality::Cumulative; "unknown defaults to cumulative")]
    fn temporality_preference_from_env(
        env: &[(&'static str, &'static str)],
        expected: Temporality,
    ) {
        let _guard = clear_otel_env(env);
        assert_eq!(temporality_preference(), expected);
    }
}
