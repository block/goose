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

#[derive(Debug, Clone)]
pub struct SignalConfig {
    pub exporter: ExporterType,
}

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
    pub fn detect() -> Option<Self> {
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

        let has_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok().is_some();

        let traces_enabled = traces_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None))
            || (has_endpoint && traces_exporter.is_none());
        let metrics_enabled = metrics_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None))
            || (has_endpoint && metrics_exporter.is_none());
        let logs_enabled = logs_exporter
            .as_ref()
            .is_some_and(|e| !matches!(e, ExporterType::None))
            || (has_endpoint && logs_exporter.is_none());

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

#[derive(Debug, Clone)]
pub struct OtelConfig {
    pub traces: Option<SignalConfig>,
    pub metrics: Option<SignalConfig>,
    pub logs: Option<SignalConfig>,
}

impl OtelConfig {
    pub fn detect() -> Option<Self> {
        if let Some(env) = OtelEnv::detect() {
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

        Self::detect_from_config()
    }

    fn detect_from_config() -> Option<Self> {
        let config = crate::config::Config::global();
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

        #[test_case(Some("true"), Some("console"), None, None, None; "sdk disabled")]
        #[test_case(None, None, None, None, None; "no vars")]
        #[test_case(None, Some("none"), Some("none"), Some("none"), None; "all none")]
        fn detect_returns_none(
            sdk_disabled: Option<&str>,
            traces: Option<&str>,
            metrics: Option<&str>,
            logs: Option<&str>,
            endpoint: Option<&str>,
        ) {
            let _guard = env_lock::lock_env([
                ("OTEL_SDK_DISABLED", sdk_disabled),
                ("OTEL_TRACES_EXPORTER", traces),
                ("OTEL_METRICS_EXPORTER", metrics),
                ("OTEL_LOGS_EXPORTER", logs),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint),
            ]);
            assert!(OtelEnv::detect().is_none());
        }

        #[test_case(Some("console"), None, None, None, true, false, false; "traces only")]
        #[test_case(None, Some("otlp"), None, None, false, true, false; "metrics only")]
        #[test_case(None, None, Some("console"), None, false, false, true; "logs only")]
        #[test_case(Some("otlp"), Some("console"), Some("otlp"), None, true, true, true; "all enabled")]
        #[test_case(Some("console"), Some("none"), Some("otlp"), None, true, false, true; "mixed")]
        #[test_case(None, None, None, Some("http://localhost:4318"), true, true, true; "endpoint enables all signals")]
        #[test_case(Some("none"), None, None, Some("http://localhost:4318"), false, true, true; "endpoint with traces none")]
        fn detect(
            traces: Option<&str>,
            metrics: Option<&str>,
            logs: Option<&str>,
            endpoint: Option<&str>,
            expect_traces: bool,
            expect_metrics: bool,
            expect_logs: bool,
        ) {
            let _guard = env_lock::lock_env([
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", traces),
                ("OTEL_METRICS_EXPORTER", metrics),
                ("OTEL_LOGS_EXPORTER", logs),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint),
            ]);
            let env = OtelEnv::detect().unwrap();
            assert_eq!(env.traces_enabled, expect_traces);
            assert_eq!(env.metrics_enabled, expect_metrics);
            assert_eq!(env.logs_enabled, expect_logs);
        }
    }
}
