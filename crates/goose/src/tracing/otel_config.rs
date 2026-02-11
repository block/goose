use std::env;

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

/// Clears all OTel env vars, then applies only the given overrides.
/// Returns an `EnvGuard` that restores everything on drop.
#[cfg(test)]
pub(crate) fn clear_otel_env(overrides: &[(&str, &str)]) -> env_lock::EnvGuard<'static> {
    let guard = env_lock::lock_env([
        ("OTEL_SDK_DISABLED", None::<&str>),
        ("OTEL_TRACES_EXPORTER", None),
        ("OTEL_METRICS_EXPORTER", None),
        ("OTEL_LOGS_EXPORTER", None),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", None),
        ("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", None),
        ("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT", None),
        ("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT", None),
        ("OTEL_EXPORTER_OTLP_TIMEOUT", None),
        ("OTEL_SERVICE_NAME", None),
        ("OTEL_RESOURCE_ATTRIBUTES", None),
    ]);
    for &(k, v) in overrides {
        std::env::set_var(k, v);
    }
    guard
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn signal_exporter_disabled(env: &[(&str, &str)]) {
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
    fn signal_exporter_by_var(signal: &str, env: &[(&str, &str)], expected: Option<ExporterType>) {
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
        env: &[(&str, &str)],
        expected: Option<ExporterType>,
    ) {
        let _guard = clear_otel_env(env);
        assert_eq!(signal_exporter(signal), expected);
    }
}
