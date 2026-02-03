//! Observability Module
//!
//! Enhanced observability for Goose with OpenTelemetry GenAI semantic conventions,
//! cost tracking, and MCP-specific metrics.
//!
//! This module provides:
//! - OpenTelemetry semantic conventions for GenAI operations
//! - Cost tracking for LLM API usage
//! - MCP-specific metrics and instrumentation
//! - Export capabilities for Prometheus/OTLP

pub mod cost_tracker;
pub mod errors;
pub mod exporters;
pub mod metrics;
pub mod semantic_conventions;

pub use cost_tracker::{CostTracker, ModelPricing, RequestCost, SessionCost, TokenUsage};
pub use errors::ObservabilityError;
pub use metrics::{GenAiMetrics, McpMetrics, ObservabilityMetrics};
pub use semantic_conventions::{gen_ai, mcp};

use chrono::{DateTime, Utc};
use opentelemetry::{global, KeyValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Report format for cost exports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// Markdown format
    Markdown,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable cost tracking
    pub cost_tracking_enabled: bool,
    /// Enable MCP metrics
    pub mcp_metrics_enabled: bool,
    /// Enable GenAI semantic conventions
    pub genai_conventions_enabled: bool,
    /// Metrics export interval in seconds
    pub metrics_export_interval_secs: u64,
    /// Custom model pricing overrides
    pub pricing_overrides: HashMap<String, ModelPricing>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            cost_tracking_enabled: true,
            mcp_metrics_enabled: true,
            genai_conventions_enabled: true,
            metrics_export_interval_secs: 60,
            pricing_overrides: HashMap::new(),
        }
    }
}

/// Main observability orchestrator
pub struct Observability {
    config: ObservabilityConfig,
    cost_tracker: Arc<CostTracker>,
    genai_metrics: Arc<GenAiMetrics>,
    mcp_metrics: Arc<McpMetrics>,
}

impl Observability {
    /// Create new observability instance with default configuration
    pub fn new() -> Self {
        Self::with_config(ObservabilityConfig::default())
    }

    /// Create new observability instance with custom configuration
    pub fn with_config(config: ObservabilityConfig) -> Self {
        let cost_tracker = Arc::new(CostTracker::with_pricing_overrides(
            config.pricing_overrides.clone(),
        ));

        let meter = global::meter("goose.observability");
        let genai_metrics = Arc::new(GenAiMetrics::new(&meter));
        let mcp_metrics = Arc::new(McpMetrics::new(&meter));

        Self {
            config,
            cost_tracker,
            genai_metrics,
            mcp_metrics,
        }
    }

    /// Get the cost tracker
    pub fn cost_tracker(&self) -> &Arc<CostTracker> {
        &self.cost_tracker
    }

    /// Get the GenAI metrics
    pub fn genai_metrics(&self) -> &Arc<GenAiMetrics> {
        &self.genai_metrics
    }

    /// Get the MCP metrics
    pub fn mcp_metrics(&self) -> &Arc<McpMetrics> {
        &self.mcp_metrics
    }

    /// Record a GenAI request with full observability
    pub async fn record_genai_request(
        &self,
        session_id: &str,
        model: &str,
        usage: &TokenUsage,
        duration_ms: f64,
        success: bool,
    ) {
        // Record cost
        if self.config.cost_tracking_enabled {
            self.cost_tracker.record(session_id, usage, model).await;
        }

        // Record metrics
        if self.config.genai_conventions_enabled {
            self.genai_metrics
                .record_request(model, usage, duration_ms, success);
        }
    }

    /// Record an MCP tool call with full observability
    pub fn record_mcp_tool_call(
        &self,
        tool_name: &str,
        server_name: &str,
        duration_ms: f64,
        success: bool,
    ) {
        if self.config.mcp_metrics_enabled {
            self.mcp_metrics
                .record_tool_call(tool_name, server_name, duration_ms, success);
        }
    }

    /// Record an MCP permission denial
    pub fn record_mcp_permission_denial(&self, tool_name: &str, reason: &str) {
        if self.config.mcp_metrics_enabled {
            self.mcp_metrics.record_permission_denial(tool_name, reason);
        }
    }

    /// Record MCP server connection change
    pub fn record_mcp_server_connection(&self, server_name: &str, connected: bool) {
        if self.config.mcp_metrics_enabled {
            self.mcp_metrics
                .record_server_connection(server_name, connected);
        }
    }

    /// Get session cost summary
    pub async fn get_session_cost(&self, session_id: &str) -> Option<SessionCost> {
        self.cost_tracker.get_session_cost(session_id).await
    }

    /// Export cost report
    pub async fn export_cost_report(
        &self,
        format: ReportFormat,
    ) -> Result<String, ObservabilityError> {
        self.cost_tracker.export_report(format).await
    }

    /// Get aggregated metrics snapshot
    pub async fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let total_cost = self.cost_tracker.get_total_cost().await;
        let total_requests = self.cost_tracker.get_total_requests().await;
        let total_tokens = self.cost_tracker.get_total_tokens().await;

        MetricsSnapshot {
            timestamp: Utc::now(),
            total_cost_usd: total_cost,
            total_requests,
            total_input_tokens: total_tokens.0,
            total_output_tokens: total_tokens.1,
            total_cached_tokens: total_tokens.2,
        }
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of aggregated metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Total number of requests
    pub total_requests: u64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total cached tokens
    pub total_cached_tokens: u64,
}

/// Span attributes builder for GenAI operations
#[derive(Debug, Default)]
pub struct GenAiSpanBuilder {
    attributes: Vec<KeyValue>,
}

impl GenAiSpanBuilder {
    /// Create a new span builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the GenAI system (e.g., "anthropic", "openai")
    pub fn system(mut self, system: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::SYSTEM, system.to_string()));
        self
    }

    /// Set the request model
    pub fn request_model(mut self, model: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::REQUEST_MODEL, model.to_string()));
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: i64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::REQUEST_MAX_TOKENS, tokens));
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::REQUEST_TEMPERATURE, temp));
        self
    }

    /// Set top_p
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::REQUEST_TOP_P, top_p));
        self
    }

    /// Set response ID
    pub fn response_id(mut self, id: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::RESPONSE_ID, id.to_string()));
        self
    }

    /// Set response model
    pub fn response_model(mut self, model: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::RESPONSE_MODEL, model.to_string()));
        self
    }

    /// Set finish reasons
    pub fn finish_reasons(mut self, reasons: Vec<String>) -> Self {
        self.attributes.push(KeyValue::new(
            gen_ai::RESPONSE_FINISH_REASONS,
            reasons.join(","),
        ));
        self
    }

    /// Set input tokens
    pub fn input_tokens(mut self, tokens: i64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::USAGE_INPUT_TOKENS, tokens));
        self
    }

    /// Set output tokens
    pub fn output_tokens(mut self, tokens: i64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::USAGE_OUTPUT_TOKENS, tokens));
        self
    }

    /// Set total tokens
    pub fn total_tokens(mut self, tokens: i64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::USAGE_TOTAL_TOKENS, tokens));
        self
    }

    /// Set cached tokens
    pub fn cached_tokens(mut self, tokens: i64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::USAGE_CACHED_TOKENS, tokens));
        self
    }

    /// Set cost in USD
    pub fn cost_usd(mut self, cost: f64) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::USAGE_COST_USD, cost));
        self
    }

    /// Set tool name
    pub fn tool_name(mut self, name: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::TOOL_NAME, name.to_string()));
        self
    }

    /// Set tool call ID
    pub fn tool_call_id(mut self, id: &str) -> Self {
        self.attributes
            .push(KeyValue::new(gen_ai::TOOL_CALL_ID, id.to_string()));
        self
    }

    /// Build the attributes
    pub fn build(self) -> Vec<KeyValue> {
        self.attributes
    }
}

/// Span attributes builder for MCP operations
#[derive(Debug, Default)]
pub struct McpSpanBuilder {
    attributes: Vec<KeyValue>,
}

impl McpSpanBuilder {
    /// Create a new span builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server name
    pub fn server_name(mut self, name: &str) -> Self {
        self.attributes
            .push(KeyValue::new(mcp::SERVER_NAME, name.to_string()));
        self
    }

    /// Set the server version
    pub fn server_version(mut self, version: &str) -> Self {
        self.attributes
            .push(KeyValue::new(mcp::SERVER_VERSION, version.to_string()));
        self
    }

    /// Set the transport type
    pub fn transport_type(mut self, transport: &str) -> Self {
        self.attributes
            .push(KeyValue::new(mcp::TRANSPORT_TYPE, transport.to_string()));
        self
    }

    /// Set the tool count
    pub fn tool_count(mut self, count: i64) -> Self {
        self.attributes.push(KeyValue::new(mcp::TOOL_COUNT, count));
        self
    }

    /// Set the resource count
    pub fn resource_count(mut self, count: i64) -> Self {
        self.attributes
            .push(KeyValue::new(mcp::RESOURCE_COUNT, count));
        self
    }

    /// Set the prompt count
    pub fn prompt_count(mut self, count: i64) -> Self {
        self.attributes
            .push(KeyValue::new(mcp::PROMPT_COUNT, count));
        self
    }

    /// Build the attributes
    pub fn build(self) -> Vec<KeyValue> {
        self.attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_config_default() {
        let config = ObservabilityConfig::default();
        assert!(config.cost_tracking_enabled);
        assert!(config.mcp_metrics_enabled);
        assert!(config.genai_conventions_enabled);
        assert_eq!(config.metrics_export_interval_secs, 60);
    }

    #[test]
    fn test_genai_span_builder() {
        let attrs = GenAiSpanBuilder::new()
            .system("anthropic")
            .request_model("claude-3-5-sonnet")
            .max_tokens(4096)
            .temperature(0.7)
            .input_tokens(100)
            .output_tokens(200)
            .build();

        assert_eq!(attrs.len(), 6);
    }

    #[test]
    fn test_mcp_span_builder() {
        let attrs = McpSpanBuilder::new()
            .server_name("filesystem")
            .server_version("1.0.0")
            .transport_type("stdio")
            .tool_count(5)
            .build();

        assert_eq!(attrs.len(), 4);
    }

    #[tokio::test]
    async fn test_observability_creation() {
        let obs = Observability::new();
        assert!(obs.config.cost_tracking_enabled);
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let obs = Observability::new();
        let snapshot = obs.get_metrics_snapshot().await;

        assert_eq!(snapshot.total_cost_usd, 0.0);
        assert_eq!(snapshot.total_requests, 0);
    }
}
