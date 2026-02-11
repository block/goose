use std::env;

/// The type of exporter to use for a signal.
#[derive(Debug, Clone, PartialEq)]
pub enum ExporterType {
    Otlp,
    Console,
    None,
}

impl ExporterType {
    /// Parses an OTel exporter env var value into an ExporterType.
    /// Empty string defaults to Otlp (the OTel SDK default).
    pub fn from_env_value(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "" | "otlp" => ExporterType::Otlp,
            "console" | "stdout" => ExporterType::Console,
            _ => ExporterType::None,
        }
    }
}

/// Per-signal configuration. Only contains exporter type since
/// endpoint/timeout/sampler are handled by the SDK automatically.
#[derive(Debug, Clone)]
pub struct SignalConfig {
    pub exporter: ExporterType,
}

/// Raw env var detection result before config-file fallback.
#[derive(Debug, PartialEq)]
pub struct OtelEnv {
    pub traces_enabled: bool,
    pub metrics_enabled: bool,
    pub logs_enabled: bool,
    traces_exporter: Option<ExporterType>,
    metrics_exporter: Option<ExporterType>,
    logs_exporter: Option<ExporterType>,
}

impl OtelEnv {
    /// Detects OTel configuration from environment variables only.
    /// Returns None if SDK is disabled or no signals are enabled.
    pub fn detect() -> Option<Self> {
        // OTEL_SDK_DISABLED=true disables everything
        if env::var("OTEL_SDK_DISABLED")
            .ok()
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        {
            return None;
        }

        let traces_exporter = env::var("OTEL_TRACES_EXPORTER")
            .ok()
            .map(|v| ExporterType::from_env_value(&v));
        let metrics_exporter = env::var("OTEL_METRICS_EXPORTER")
            .ok()
            .map(|v| ExporterType::from_env_value(&v));
        let logs_exporter = env::var("OTEL_LOGS_EXPORTER")
            .ok()
            .map(|v| ExporterType::from_env_value(&v));

        let traces_enabled = traces_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None));
        let metrics_enabled = metrics_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None));
        let logs_enabled = logs_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None));

        if !traces_enabled && !metrics_enabled && !logs_enabled {
            return None;
        }

        Some(Self {
            traces_enabled,
            metrics_enabled,
            logs_enabled,
            traces_exporter,
            metrics_exporter,
            logs_exporter,
        })
    }
}

/// Resolved OTel configuration after env + config-file cascade.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    pub traces: Option<SignalConfig>,
    pub metrics: Option<SignalConfig>,
    pub logs: Option<SignalConfig>,
}

impl OtelConfig {
    /// Detects OTel configuration with cascade: env vars → config file fallback.
    pub fn detect() -> Option<Self> {
        if let Some(env) = OtelEnv::detect() {
            // Env vars are set — use them directly
            let traces = if env.traces_enabled {
                Some(SignalConfig {
                    exporter: env.traces_exporter.unwrap_or(ExporterType::Otlp),
                })
            } else {
                None
            };
            let metrics = if env.metrics_enabled {
                Some(SignalConfig {
                    exporter: env.metrics_exporter.unwrap_or(ExporterType::Otlp),
                })
            } else {
                None
            };
            let logs = if env.logs_enabled {
                Some(SignalConfig {
                    exporter: env.logs_exporter.unwrap_or(ExporterType::Otlp),
                })
            } else {
                None
            };
            return Some(Self {
                traces,
                metrics,
                logs,
            });
        }

        // Fall back to config file — if endpoint is present, enable all signals as OTLP
        Self::detect_from_config()
    }

    /// Fallback: check goose config file for otel_exporter_otlp_endpoint.
    fn detect_from_config() -> Option<Self> {
        let config = crate::config::Config::global();
        // If the config file has an endpoint, enable all three signals
        config
            .get_param::<String>("otel_exporter_otlp_endpoint")
            .ok()
            .map(|_| {
                let signal = SignalConfig {
                    exporter: ExporterType::Otlp,
                };
                Self {
                    traces: Some(signal.clone()),
                    metrics: Some(signal.clone()),
                    logs: Some(signal),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    mod otel_env {
        use super::*;
        use test_case::test_case;

        #[test_case(Some("true"), Some("console"), None, None; "sdk disabled")]
        #[test_case(None, None, None, None; "no vars")]
        #[test_case(None, Some("none"), Some("none"), Some("none"); "all none")]
        fn detect_returns_none(
            sdk_disabled: Option<&str>,
            traces: Option<&str>,
            metrics: Option<&str>,
            logs: Option<&str>,
        ) {
            let _guard = env_lock::lock_env([
                ("OTEL_SDK_DISABLED", sdk_disabled),
                ("OTEL_TRACES_EXPORTER", traces),
                ("OTEL_METRICS_EXPORTER", metrics),
                ("OTEL_LOGS_EXPORTER", logs),
            ]);
            assert!(OtelEnv::detect().is_none());
        }

        #[test_case(Some("console"), None, None, true, false, false; "traces only")]
        #[test_case(None, Some("otlp"), None, false, true, false; "metrics only")]
        #[test_case(None, None, Some("console"), false, false, true; "logs only")]
        #[test_case(Some("otlp"), Some("console"), Some("otlp"), true, true, true; "all enabled")]
        #[test_case(Some("console"), Some("none"), Some("otlp"), true, false, true; "mixed")]
        fn detect(
            traces: Option<&str>,
            metrics: Option<&str>,
            logs: Option<&str>,
            expect_traces: bool,
            expect_metrics: bool,
            expect_logs: bool,
        ) {
            let _guard = env_lock::lock_env([
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", traces),
                ("OTEL_METRICS_EXPORTER", metrics),
                ("OTEL_LOGS_EXPORTER", logs),
            ]);
            let env = OtelEnv::detect().unwrap();
            assert_eq!(env.traces_enabled, expect_traces);
            assert_eq!(env.metrics_enabled, expect_metrics);
            assert_eq!(env.logs_enabled, expect_logs);
        }
    }
}
