use std::env;
use std::time::Duration;

/// Environment-based OpenTelemetry signal detection.
/// Returns None if no OTEL env vars are set or SDK is disabled.
#[derive(Debug, Clone, PartialEq)]
pub struct OtelEnv {
    pub traces_enabled: bool,
    pub metrics_enabled: bool,
}

/// Configuration for OpenTelemetry traces and metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct OtelConfig {
    pub traces: Option<TracingConfig>,
    pub metrics: Option<MetricsConfig>,
}

/// Configuration for trace exporting
#[derive(Debug, Clone, PartialEq)]
pub struct TracingConfig {
    pub exporter: ExporterType,
    pub endpoint: Option<String>,
    pub timeout: Option<Duration>,
}

/// Configuration for metrics exporting
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsConfig {
    pub exporter: ExporterType,
    pub endpoint: Option<String>,
    pub timeout: Option<Duration>,
}

/// Type of OpenTelemetry exporter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExporterType {
    /// OTLP exporter (sends to OpenTelemetry collector or compatible backend)
    Otlp,
    /// Console/stdout exporter (prints to terminal, for debugging)
    Console,
    /// No exporter (explicitly disabled)
    None,
}

impl ExporterType {
    /// Parse exporter type from environment variable value
    fn from_env_value(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "none" => Self::None,
            "console" | "stdout" | "logging" => Self::Console,
            "otlp" | "" => Self::Otlp,
            _ => {
                tracing::warn!("Unknown OTEL exporter type '{}', treating as 'none'", value);
                Self::None
            }
        }
    }
}

impl OtelEnv {
    /// Detect which signals are enabled from environment variables only.
    /// Returns None if no env vars set or SDK disabled.
    pub fn detect() -> Option<Self> {
        // Check if SDK is globally disabled
        if env::var("OTEL_SDK_DISABLED")
            .ok()
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false)
        {
            return None;
        }

        let traces_var = env::var("OTEL_TRACES_EXPORTER").ok();
        let metrics_var = env::var("OTEL_METRICS_EXPORTER").ok();

        // If neither env var is set, return None
        if traces_var.is_none() && metrics_var.is_none() {
            return None;
        }

        let traces_enabled = traces_var
            .as_deref()
            .map(|v| ExporterType::from_env_value(v) != ExporterType::None)
            .unwrap_or(false);

        let metrics_enabled = metrics_var
            .as_deref()
            .map(|v| ExporterType::from_env_value(v) != ExporterType::None)
            .unwrap_or(false);

        // Return None if both are explicitly disabled
        if !traces_enabled && !metrics_enabled {
            return None;
        }

        Some(OtelEnv {
            traces_enabled,
            metrics_enabled,
        })
    }
}

impl OtelConfig {
    /// Detect OpenTelemetry configuration.
    ///
    /// Detection logic:
    /// 1. Check environment variables for signal enablement
    /// 2. If no env vars, check config file
    /// 3. Build full config with exporter types, endpoints, timeouts
    pub fn detect() -> Option<Self> {
        // First check environment variables
        if let Some(env_config) = OtelEnv::detect() {
            let traces = if env_config.traces_enabled {
                Self::build_traces_config()
            } else {
                None
            };

            let metrics = if env_config.metrics_enabled {
                Self::build_metrics_config()
            } else {
                None
            };

            return Some(OtelConfig { traces, metrics });
        }

        // No env vars set, check config file
        Self::detect_from_config()
    }

    fn build_traces_config() -> Option<TracingConfig> {
        let exporter_str = env::var("OTEL_TRACES_EXPORTER").ok()?;
        let exporter = ExporterType::from_env_value(&exporter_str);

        if exporter == ExporterType::None {
            return None;
        }

        if exporter == ExporterType::Console {
            return Some(TracingConfig {
                exporter,
                endpoint: None,
                timeout: None,
            });
        }

        Some(TracingConfig {
            exporter,
            endpoint: Self::resolve_traces_endpoint(),
            timeout: Self::resolve_timeout(),
        })
    }

    fn build_metrics_config() -> Option<MetricsConfig> {
        let exporter_str = env::var("OTEL_METRICS_EXPORTER").ok()?;
        let exporter = ExporterType::from_env_value(&exporter_str);

        if exporter == ExporterType::None {
            return None;
        }

        if exporter == ExporterType::Console {
            return Some(MetricsConfig {
                exporter,
                endpoint: None,
                timeout: None,
            });
        }

        Some(MetricsConfig {
            exporter,
            endpoint: Self::resolve_metrics_endpoint(),
            timeout: Self::resolve_timeout(),
        })
    }

    /// Fallback to config file when no env vars are set
    fn detect_from_config() -> Option<Self> {
        let config = crate::config::Config::global();

        let endpoint = config
            .get_param::<String>("otel_exporter_otlp_endpoint")
            .ok();
        let timeout = config
            .get_param::<u64>("otel_exporter_otlp_timeout")
            .ok()
            .map(Duration::from_millis);

        if endpoint.is_some() || timeout.is_some() {
            Some(OtelConfig {
                traces: Some(TracingConfig {
                    exporter: ExporterType::Otlp,
                    endpoint: endpoint.clone(),
                    timeout,
                }),
                metrics: Some(MetricsConfig {
                    exporter: ExporterType::Otlp,
                    endpoint,
                    timeout,
                }),
            })
        } else {
            None
        }
    }

    fn resolve_traces_endpoint() -> Option<String> {
        env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
            .ok()
            .or_else(|| env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok())
            .or_else(|| {
                crate::config::Config::global()
                    .get_param::<String>("otel_exporter_otlp_endpoint")
                    .ok()
            })
    }

    fn resolve_metrics_endpoint() -> Option<String> {
        env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
            .ok()
            .or_else(|| env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok())
            .or_else(|| {
                crate::config::Config::global()
                    .get_param::<String>("otel_exporter_otlp_endpoint")
                    .ok()
            })
    }

    fn resolve_timeout() -> Option<Duration> {
        env::var("OTEL_EXPORTER_OTLP_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .or_else(|| {
                crate::config::Config::global()
                    .get_param::<u64>("otel_exporter_otlp_timeout")
                    .ok()
                    .map(Duration::from_millis)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env::with_vars;

    #[test]
    fn test_exporter_type_from_env_value() {
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
    }

    #[test]
    fn test_otel_env_detect_sdk_disabled() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", Some("true")),
                ("OTEL_TRACES_EXPORTER", Some("console")),
            ],
            || {
                assert_eq!(OtelEnv::detect(), None);
            },
        );
    }

    #[test]
    fn test_otel_env_detect_no_vars() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", None::<&str>),
                ("OTEL_METRICS_EXPORTER", None::<&str>),
            ],
            || {
                assert_eq!(OtelEnv::detect(), None);
            },
        );
    }

    #[test]
    fn test_otel_env_traces_only() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("console")),
                ("OTEL_METRICS_EXPORTER", None::<&str>),
            ],
            || {
                let env = OtelEnv::detect().expect("Should detect traces");
                assert!(env.traces_enabled);
                assert!(!env.metrics_enabled);
            },
        );
    }

    #[test]
    fn test_otel_env_metrics_only() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", None::<&str>),
                ("OTEL_METRICS_EXPORTER", Some("console")),
            ],
            || {
                let env = OtelEnv::detect().expect("Should detect metrics");
                assert!(!env.traces_enabled);
                assert!(env.metrics_enabled);
            },
        );
    }

    #[test]
    fn test_otel_env_both_enabled() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("console")),
                ("OTEL_METRICS_EXPORTER", Some("otlp")),
            ],
            || {
                let env = OtelEnv::detect().expect("Should detect both");
                assert!(env.traces_enabled);
                assert!(env.metrics_enabled);
            },
        );
    }

    #[test]
    fn test_otel_env_both_none() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("none")),
                ("OTEL_METRICS_EXPORTER", Some("none")),
            ],
            || {
                assert_eq!(OtelEnv::detect(), None);
            },
        );
    }

    #[test]
    fn test_otel_env_one_none_one_enabled() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("console")),
                ("OTEL_METRICS_EXPORTER", Some("none")),
            ],
            || {
                let env = OtelEnv::detect().expect("Should detect traces");
                assert!(env.traces_enabled);
                assert!(!env.metrics_enabled);
            },
        );
    }

    #[test]
    fn test_otel_config_traces_console() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("console")),
                ("OTEL_METRICS_EXPORTER", Some("none")),
            ],
            || {
                let config = OtelConfig::detect().expect("Should have config");
                assert!(config.traces.is_some());
                let traces = config.traces.unwrap();
                assert_eq!(traces.exporter, ExporterType::Console);
                assert_eq!(traces.endpoint, None);
                assert!(config.metrics.is_none());
            },
        );
    }

    #[test]
    fn test_otel_config_traces_otlp_with_endpoint() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_METRICS_EXPORTER", Some("none")),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://localhost:4318")),
            ],
            || {
                let config = OtelConfig::detect().expect("Should have config");
                assert!(config.traces.is_some());
                let traces = config.traces.unwrap();
                assert_eq!(traces.exporter, ExporterType::Otlp);
                assert_eq!(traces.endpoint, Some("http://localhost:4318".to_string()));
            },
        );
    }

    #[test]
    fn test_otel_config_mixed_exporters() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_METRICS_EXPORTER", Some("console")),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://localhost:4318")),
            ],
            || {
                let config = OtelConfig::detect().expect("Should have config");

                assert!(config.traces.is_some());
                let traces = config.traces.unwrap();
                assert_eq!(traces.exporter, ExporterType::Otlp);
                assert_eq!(traces.endpoint, Some("http://localhost:4318".to_string()));

                assert!(config.metrics.is_some());
                let metrics = config.metrics.unwrap();
                assert_eq!(metrics.exporter, ExporterType::Console);
                assert_eq!(metrics.endpoint, None);
            },
        );
    }

    #[test]
    fn test_otel_config_signal_specific_endpoints() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_METRICS_EXPORTER", Some("otlp")),
                ("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://general:4318")),
                (
                    "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
                    Some("http://traces:4318"),
                ),
                (
                    "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
                    Some("http://metrics:4318"),
                ),
            ],
            || {
                let config = OtelConfig::detect().expect("Should have config");

                let traces = config.traces.expect("Traces should be configured");
                assert_eq!(traces.endpoint, Some("http://traces:4318".to_string()));

                let metrics = config.metrics.expect("Metrics should be configured");
                assert_eq!(metrics.endpoint, Some("http://metrics:4318".to_string()));
            },
        );
    }

    #[test]
    fn test_otel_config_timeout() {
        with_vars(
            vec![
                ("OTEL_SDK_DISABLED", None::<&str>),
                ("OTEL_TRACES_EXPORTER", Some("otlp")),
                ("OTEL_EXPORTER_OTLP_TIMEOUT", Some("5000")),
            ],
            || {
                let config = OtelConfig::detect().expect("Should have config");
                let traces = config.traces.expect("Traces should be configured");
                assert_eq!(traces.timeout, Some(Duration::from_millis(5000)));
            },
        );
    }
}
