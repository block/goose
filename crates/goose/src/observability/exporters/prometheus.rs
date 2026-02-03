//! Prometheus Exporter
//!
//! Export metrics and cost data in Prometheus format.

use super::super::cost_tracker::CostTracker;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prometheus metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (can go up and down)
    Gauge,
    /// Histogram (distribution of values)
    Histogram,
    /// Summary (statistical summary)
    Summary,
}

/// A single Prometheus metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Help text
    pub help: String,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Value
    pub value: f64,
}

impl PrometheusMetric {
    /// Create a new counter metric
    pub fn counter(name: impl Into<String>, help: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            metric_type: MetricType::Counter,
            help: help.into(),
            labels: HashMap::new(),
            value,
        }
    }

    /// Create a new gauge metric
    pub fn gauge(name: impl Into<String>, help: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            metric_type: MetricType::Gauge,
            help: help.into(),
            labels: HashMap::new(),
            value,
        }
    }

    /// Add a label
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Format as Prometheus exposition format
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::new();

        // HELP line
        output.push_str(&format!("# HELP {} {}\n", self.name, self.help));

        // TYPE line
        let type_str = match self.metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Summary => "summary",
        };
        output.push_str(&format!("# TYPE {} {}\n", self.name, type_str));

        // Metric value with labels
        if self.labels.is_empty() {
            output.push_str(&format!("{} {}\n", self.name, self.value));
        } else {
            let labels_str: Vec<String> = self
                .labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            output.push_str(&format!(
                "{}{{{}}} {}\n",
                self.name,
                labels_str.join(","),
                self.value
            ));
        }

        output
    }
}

/// Prometheus exporter for observability data
pub struct PrometheusExporter {
    /// Metric prefix
    prefix: String,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter with default prefix
    pub fn new() -> Self {
        Self {
            prefix: "goose".to_string(),
        }
    }

    /// Create a new Prometheus exporter with custom prefix
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Export cost tracker data as Prometheus metrics
    pub async fn export_cost_metrics(&self, tracker: &CostTracker) -> String {
        let mut output = String::new();

        // Get all session costs
        let sessions = tracker.get_all_session_costs().await;
        let total_cost = tracker.get_total_cost().await;
        let total_requests = tracker.get_total_requests().await;
        let (total_input, total_output, total_cached) = tracker.get_total_tokens().await;

        // Total metrics
        output.push_str(&self.format_metric(
            "cost_total_usd",
            MetricType::Counter,
            "Total cost in USD",
            total_cost,
            &[],
        ));

        output.push_str(&self.format_metric(
            "requests_total",
            MetricType::Counter,
            "Total number of GenAI requests",
            total_requests as f64,
            &[],
        ));

        output.push_str(&self.format_metric(
            "tokens_input_total",
            MetricType::Counter,
            "Total input tokens",
            total_input as f64,
            &[],
        ));

        output.push_str(&self.format_metric(
            "tokens_output_total",
            MetricType::Counter,
            "Total output tokens",
            total_output as f64,
            &[],
        ));

        output.push_str(&self.format_metric(
            "tokens_cached_total",
            MetricType::Counter,
            "Total cached tokens",
            total_cached as f64,
            &[],
        ));

        output.push_str(&self.format_metric(
            "sessions_active",
            MetricType::Gauge,
            "Number of active sessions",
            sessions.len() as f64,
            &[],
        ));

        // Per-session metrics
        for session in sessions.values() {
            let labels = [("session_id", session.session_id.as_str())];

            output.push_str(&self.format_metric(
                "session_cost_usd",
                MetricType::Gauge,
                "Session cost in USD",
                session.total_cost_usd,
                &labels,
            ));

            output.push_str(&self.format_metric(
                "session_tokens_input",
                MetricType::Counter,
                "Session input tokens",
                session.total_input_tokens as f64,
                &labels,
            ));

            output.push_str(&self.format_metric(
                "session_tokens_output",
                MetricType::Counter,
                "Session output tokens",
                session.total_output_tokens as f64,
                &labels,
            ));

            output.push_str(&self.format_metric(
                "session_requests",
                MetricType::Counter,
                "Session request count",
                session.requests.len() as f64,
                &labels,
            ));

            // Per-model metrics within session
            let mut model_costs: HashMap<String, f64> = HashMap::new();
            let mut model_counts: HashMap<String, u64> = HashMap::new();

            for request in &session.requests {
                *model_costs.entry(request.model.clone()).or_default() += request.cost_usd;
                *model_counts.entry(request.model.clone()).or_default() += 1;
            }

            for (model, cost) in &model_costs {
                let model_labels = [
                    ("session_id", session.session_id.as_str()),
                    ("model", model.as_str()),
                ];

                output.push_str(&self.format_metric(
                    "session_model_cost_usd",
                    MetricType::Gauge,
                    "Cost per model in session",
                    *cost,
                    &model_labels,
                ));

                output.push_str(&self.format_metric(
                    "session_model_requests",
                    MetricType::Counter,
                    "Request count per model in session",
                    *model_counts.get(model).unwrap_or(&0) as f64,
                    &model_labels,
                ));
            }
        }

        // Metadata
        output.push_str(&self.format_metric(
            "exporter_last_scrape_timestamp",
            MetricType::Gauge,
            "Timestamp of last metrics export",
            Utc::now().timestamp() as f64,
            &[],
        ));

        output
    }

    /// Format a single metric in Prometheus exposition format
    fn format_metric(
        &self,
        name: &str,
        metric_type: MetricType,
        help: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> String {
        let full_name = format!("{}_{}", self.prefix, name);
        let type_str = match metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Summary => "summary",
        };

        let mut output = String::new();
        output.push_str(&format!("# HELP {} {}\n", full_name, help));
        output.push_str(&format!("# TYPE {} {}\n", full_name, type_str));

        if labels.is_empty() {
            output.push_str(&format!("{} {}\n", full_name, value));
        } else {
            let labels_str: Vec<String> = labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            output.push_str(&format!(
                "{}{{{}}} {}\n",
                full_name,
                labels_str.join(","),
                value
            ));
        }

        output
    }

    /// Generate a complete metrics endpoint response
    pub async fn generate_metrics_response(&self, tracker: &CostTracker) -> String {
        let mut response = String::new();

        // Header comment
        response.push_str("# Goose Observability Metrics\n");
        response.push_str(&format!(
            "# Generated at: {}\n\n",
            Utc::now().to_rfc3339()
        ));

        // Cost metrics
        response.push_str(&self.export_cost_metrics(tracker).await);

        response
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct for Grafana dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaDashboard {
    /// Dashboard title
    pub title: String,
    /// Dashboard description
    pub description: String,
    /// Dashboard panels
    pub panels: Vec<GrafanaPanel>,
}

/// Grafana panel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaPanel {
    /// Panel title
    pub title: String,
    /// Panel type (graph, stat, table, etc.)
    pub panel_type: String,
    /// PromQL queries
    pub queries: Vec<String>,
    /// Grid position
    pub grid_pos: GridPos,
}

/// Grid position for Grafana panels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPos {
    pub h: u32,
    pub w: u32,
    pub x: u32,
    pub y: u32,
}

impl GrafanaDashboard {
    /// Create a default Goose observability dashboard
    pub fn goose_default() -> Self {
        Self {
            title: "Goose Observability".to_string(),
            description: "GenAI and MCP metrics for Goose".to_string(),
            panels: vec![
                GrafanaPanel {
                    title: "Total Cost (USD)".to_string(),
                    panel_type: "stat".to_string(),
                    queries: vec!["goose_cost_total_usd".to_string()],
                    grid_pos: GridPos { h: 4, w: 6, x: 0, y: 0 },
                },
                GrafanaPanel {
                    title: "Total Requests".to_string(),
                    panel_type: "stat".to_string(),
                    queries: vec!["goose_requests_total".to_string()],
                    grid_pos: GridPos { h: 4, w: 6, x: 6, y: 0 },
                },
                GrafanaPanel {
                    title: "Active Sessions".to_string(),
                    panel_type: "stat".to_string(),
                    queries: vec!["goose_sessions_active".to_string()],
                    grid_pos: GridPos { h: 4, w: 6, x: 12, y: 0 },
                },
                GrafanaPanel {
                    title: "Token Usage".to_string(),
                    panel_type: "stat".to_string(),
                    queries: vec![
                        "goose_tokens_input_total".to_string(),
                        "goose_tokens_output_total".to_string(),
                    ],
                    grid_pos: GridPos { h: 4, w: 6, x: 18, y: 0 },
                },
                GrafanaPanel {
                    title: "Cost Over Time".to_string(),
                    panel_type: "graph".to_string(),
                    queries: vec!["rate(goose_cost_total_usd[5m])".to_string()],
                    grid_pos: GridPos { h: 8, w: 12, x: 0, y: 4 },
                },
                GrafanaPanel {
                    title: "Token Usage Over Time".to_string(),
                    panel_type: "graph".to_string(),
                    queries: vec![
                        "rate(goose_tokens_input_total[5m])".to_string(),
                        "rate(goose_tokens_output_total[5m])".to_string(),
                    ],
                    grid_pos: GridPos { h: 8, w: 12, x: 12, y: 4 },
                },
                GrafanaPanel {
                    title: "Cost by Session".to_string(),
                    panel_type: "table".to_string(),
                    queries: vec!["goose_session_cost_usd".to_string()],
                    grid_pos: GridPos { h: 8, w: 12, x: 0, y: 12 },
                },
                GrafanaPanel {
                    title: "Cost by Model".to_string(),
                    panel_type: "piechart".to_string(),
                    queries: vec!["sum by (model) (goose_session_model_cost_usd)".to_string()],
                    grid_pos: GridPos { h: 8, w: 12, x: 12, y: 12 },
                },
            ],
        }
    }

    /// Export as JSON for Grafana import
    pub fn to_grafana_json(&self) -> Result<String, serde_json::Error> {
        // Create a simplified Grafana dashboard structure
        let dashboard = serde_json::json!({
            "dashboard": {
                "title": self.title,
                "description": self.description,
                "panels": self.panels.iter().enumerate().map(|(i, p)| {
                    serde_json::json!({
                        "id": i + 1,
                        "title": p.title,
                        "type": p.panel_type,
                        "gridPos": {
                            "h": p.grid_pos.h,
                            "w": p.grid_pos.w,
                            "x": p.grid_pos.x,
                            "y": p.grid_pos.y
                        },
                        "targets": p.queries.iter().enumerate().map(|(j, q)| {
                            serde_json::json!({
                                "refId": format!("{}", (b'A' + j as u8) as char),
                                "expr": q
                            })
                        }).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>(),
                "time": {
                    "from": "now-1h",
                    "to": "now"
                },
                "refresh": "5s"
            }
        });

        serde_json::to_string_pretty(&dashboard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::cost_tracker::TokenUsage;

    #[test]
    fn test_prometheus_metric_creation() {
        let metric = PrometheusMetric::counter("test_counter", "A test counter", 42.0);
        assert_eq!(metric.name, "test_counter");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.value, 42.0);
    }

    #[test]
    fn test_prometheus_metric_with_labels() {
        let metric = PrometheusMetric::gauge("test_gauge", "A test gauge", 3.14)
            .with_label("env", "production")
            .with_label("region", "us-west");

        assert_eq!(metric.labels.len(), 2);
        assert_eq!(metric.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_prometheus_format_no_labels() {
        let metric = PrometheusMetric::counter("requests_total", "Total requests", 100.0);
        let output = metric.to_prometheus_format();

        assert!(output.contains("# HELP requests_total Total requests"));
        assert!(output.contains("# TYPE requests_total counter"));
        assert!(output.contains("requests_total 100"));
    }

    #[test]
    fn test_prometheus_format_with_labels() {
        let metric = PrometheusMetric::gauge("memory_usage", "Memory usage in bytes", 1024.0)
            .with_label("host", "server1");

        let output = metric.to_prometheus_format();
        assert!(output.contains("memory_usage{host=\"server1\"} 1024"));
    }

    #[tokio::test]
    async fn test_prometheus_exporter_basic() {
        let exporter = PrometheusExporter::new();
        let tracker = CostTracker::new();

        let output = exporter.export_cost_metrics(&tracker).await;

        assert!(output.contains("goose_cost_total_usd"));
        assert!(output.contains("goose_requests_total"));
        assert!(output.contains("goose_sessions_active"));
    }

    #[tokio::test]
    async fn test_prometheus_exporter_with_data() {
        let exporter = PrometheusExporter::new();
        let tracker = CostTracker::new();

        // Add some data
        let usage = TokenUsage::new(100, 50);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;

        let output = exporter.export_cost_metrics(&tracker).await;

        assert!(output.contains("goose_sessions_active 1"));
        assert!(output.contains("session_id=\"session-1\""));
    }

    #[tokio::test]
    async fn test_prometheus_exporter_custom_prefix() {
        let exporter = PrometheusExporter::with_prefix("myapp");
        let tracker = CostTracker::new();

        let output = exporter.export_cost_metrics(&tracker).await;

        assert!(output.contains("myapp_cost_total_usd"));
        assert!(!output.contains("goose_cost_total_usd"));
    }

    #[tokio::test]
    async fn test_generate_metrics_response() {
        let exporter = PrometheusExporter::new();
        let tracker = CostTracker::new();

        let response = exporter.generate_metrics_response(&tracker).await;

        assert!(response.contains("# Goose Observability Metrics"));
        assert!(response.contains("# Generated at:"));
    }

    #[test]
    fn test_grafana_dashboard_default() {
        let dashboard = GrafanaDashboard::goose_default();

        assert_eq!(dashboard.title, "Goose Observability");
        assert!(!dashboard.panels.is_empty());
    }

    #[test]
    fn test_grafana_dashboard_to_json() {
        let dashboard = GrafanaDashboard::goose_default();
        let json = dashboard.to_grafana_json().unwrap();

        assert!(json.contains("\"title\": \"Goose Observability\""));
        assert!(json.contains("\"panels\""));
    }
}
