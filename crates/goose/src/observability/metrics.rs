//! MCP and GenAI Metrics
//!
//! OpenTelemetry metrics for MCP operations and GenAI requests.

use super::cost_tracker::TokenUsage;
use opentelemetry::metrics::{Counter, Histogram, Meter, UpDownCounter};
use opentelemetry::KeyValue;
use std::sync::atomic::{AtomicI64, Ordering};

/// GenAI-specific metrics following OpenTelemetry semantic conventions
pub struct GenAiMetrics {
    /// Counter for total requests
    request_counter: Counter<u64>,
    /// Histogram for request duration
    duration_histogram: Histogram<f64>,
    /// Counter for input tokens
    input_tokens_counter: Counter<u64>,
    /// Counter for output tokens
    output_tokens_counter: Counter<u64>,
    /// Counter for cached tokens
    cached_tokens_counter: Counter<u64>,
    /// Histogram for tokens per request
    tokens_per_request_histogram: Histogram<f64>,
    /// Counter for errors
    error_counter: Counter<u64>,
}

impl GenAiMetrics {
    /// Create new GenAI metrics
    pub fn new(meter: &Meter) -> Self {
        Self {
            request_counter: meter
                .u64_counter("gen_ai.client.requests")
                .with_description("Number of GenAI client requests")
                .with_unit("{request}")
                .build(),

            duration_histogram: meter
                .f64_histogram("gen_ai.client.operation.duration")
                .with_description("Duration of GenAI operations")
                .with_unit("ms")
                .build(),

            input_tokens_counter: meter
                .u64_counter("gen_ai.client.token.usage.input")
                .with_description("Number of input tokens used")
                .with_unit("{token}")
                .build(),

            output_tokens_counter: meter
                .u64_counter("gen_ai.client.token.usage.output")
                .with_description("Number of output tokens generated")
                .with_unit("{token}")
                .build(),

            cached_tokens_counter: meter
                .u64_counter("gen_ai.client.token.usage.cached")
                .with_description("Number of cached tokens used")
                .with_unit("{token}")
                .build(),

            tokens_per_request_histogram: meter
                .f64_histogram("gen_ai.client.tokens_per_request")
                .with_description("Token usage per request")
                .with_unit("{token}")
                .build(),

            error_counter: meter
                .u64_counter("gen_ai.client.errors")
                .with_description("Number of GenAI client errors")
                .with_unit("{error}")
                .build(),
        }
    }

    /// Record a GenAI request
    pub fn record_request(
        &self,
        model: &str,
        usage: &TokenUsage,
        duration_ms: f64,
        success: bool,
    ) {
        let attributes = &[
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.operation.name", "chat"),
            KeyValue::new("success", success),
        ];

        // Record request count
        self.request_counter.add(1, attributes);

        // Record duration
        self.duration_histogram.record(duration_ms, attributes);

        // Record token usage
        let model_attr = &[KeyValue::new("gen_ai.request.model", model.to_string())];
        self.input_tokens_counter.add(usage.input_tokens, model_attr);
        self.output_tokens_counter.add(usage.output_tokens, model_attr);
        if usage.cached_tokens > 0 {
            self.cached_tokens_counter.add(usage.cached_tokens, model_attr);
        }

        // Record tokens per request
        self.tokens_per_request_histogram
            .record(usage.total_tokens() as f64, model_attr);

        // Record error if not successful
        if !success {
            self.error_counter.add(1, attributes);
        }
    }

    /// Record an error
    pub fn record_error(&self, model: &str, error_type: &str) {
        let attributes = &[
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.error.type", error_type.to_string()),
        ];
        self.error_counter.add(1, attributes);
    }
}

/// MCP-specific metrics
pub struct McpMetrics {
    /// Counter for tool calls
    tool_calls_counter: Counter<u64>,
    /// Histogram for tool call duration
    tool_duration_histogram: Histogram<f64>,
    /// Gauge for active server connections
    server_connections_gauge: UpDownCounter<i64>,
    /// Counter for permission denials
    permission_denials_counter: Counter<u64>,
    /// Histogram for cache hit ratio
    cache_hit_ratio_histogram: Histogram<f64>,
    /// Counter for successful tool calls
    tool_success_counter: Counter<u64>,
    /// Counter for failed tool calls
    tool_failure_counter: Counter<u64>,
    /// Internal connection count tracker
    connection_count: AtomicI64,
}

impl McpMetrics {
    /// Create new MCP metrics
    pub fn new(meter: &Meter) -> Self {
        Self {
            tool_calls_counter: meter
                .u64_counter("mcp.tool.calls")
                .with_description("Number of MCP tool calls")
                .with_unit("{call}")
                .build(),

            tool_duration_histogram: meter
                .f64_histogram("mcp.tool.duration")
                .with_description("Duration of MCP tool calls")
                .with_unit("ms")
                .build(),

            server_connections_gauge: meter
                .i64_up_down_counter("mcp.server.connections")
                .with_description("Number of active MCP server connections")
                .with_unit("{connection}")
                .build(),

            permission_denials_counter: meter
                .u64_counter("mcp.permission.denials")
                .with_description("Number of permission denials")
                .with_unit("{denial}")
                .build(),

            cache_hit_ratio_histogram: meter
                .f64_histogram("mcp.cache.hit_ratio")
                .with_description("Cache hit ratio for tool results")
                .with_unit("1")
                .build(),

            tool_success_counter: meter
                .u64_counter("mcp.tool.success")
                .with_description("Number of successful MCP tool calls")
                .with_unit("{call}")
                .build(),

            tool_failure_counter: meter
                .u64_counter("mcp.tool.failure")
                .with_description("Number of failed MCP tool calls")
                .with_unit("{call}")
                .build(),

            connection_count: AtomicI64::new(0),
        }
    }

    /// Record a tool call
    pub fn record_tool_call(
        &self,
        tool_name: &str,
        server_name: &str,
        duration_ms: f64,
        success: bool,
    ) {
        let attributes = &[
            KeyValue::new("mcp.tool.name", tool_name.to_string()),
            KeyValue::new("mcp.server.name", server_name.to_string()),
            KeyValue::new("success", success),
        ];

        self.tool_calls_counter.add(1, attributes);
        self.tool_duration_histogram.record(duration_ms, attributes);

        let tool_attr = &[
            KeyValue::new("mcp.tool.name", tool_name.to_string()),
            KeyValue::new("mcp.server.name", server_name.to_string()),
        ];

        if success {
            self.tool_success_counter.add(1, tool_attr);
        } else {
            self.tool_failure_counter.add(1, tool_attr);
        }
    }

    /// Record a permission denial
    pub fn record_permission_denial(&self, tool_name: &str, reason: &str) {
        let attributes = &[
            KeyValue::new("mcp.tool.name", tool_name.to_string()),
            KeyValue::new("mcp.permission.denial_reason", reason.to_string()),
        ];
        self.permission_denials_counter.add(1, attributes);
    }

    /// Record server connection change
    pub fn record_server_connection(&self, server_name: &str, connected: bool) {
        let attributes = &[KeyValue::new("mcp.server.name", server_name.to_string())];

        if connected {
            self.server_connections_gauge.add(1, attributes);
            self.connection_count.fetch_add(1, Ordering::SeqCst);
        } else {
            self.server_connections_gauge.add(-1, attributes);
            self.connection_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Get current connection count
    pub fn get_connection_count(&self) -> i64 {
        self.connection_count.load(Ordering::SeqCst)
    }

    /// Record cache hit ratio
    pub fn record_cache_hit_ratio(&self, tool_name: &str, ratio: f64) {
        let attributes = &[KeyValue::new("mcp.tool.name", tool_name.to_string())];
        self.cache_hit_ratio_histogram.record(ratio, attributes);
    }
}

/// Combined observability metrics
pub struct ObservabilityMetrics {
    /// GenAI metrics
    pub genai: GenAiMetrics,
    /// MCP metrics
    pub mcp: McpMetrics,
}

impl ObservabilityMetrics {
    /// Create new observability metrics
    pub fn new(meter: &Meter) -> Self {
        Self {
            genai: GenAiMetrics::new(meter),
            mcp: McpMetrics::new(meter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::global;

    fn get_test_meter() -> Meter {
        global::meter("test.observability")
    }

    #[test]
    fn test_genai_metrics_creation() {
        let meter = get_test_meter();
        let _metrics = GenAiMetrics::new(&meter);
        // If we get here without panicking, the test passes
    }

    #[test]
    fn test_genai_metrics_record_request() {
        let meter = get_test_meter();
        let metrics = GenAiMetrics::new(&meter);

        let usage = TokenUsage::new(100, 200);
        metrics.record_request("claude-3-5-sonnet", &usage, 150.0, true);
        // Metrics are recorded asynchronously, so we just verify no panic
    }

    #[test]
    fn test_genai_metrics_record_error() {
        let meter = get_test_meter();
        let metrics = GenAiMetrics::new(&meter);

        metrics.record_error("gpt-4o", "rate_limit");
        // Verify no panic
    }

    #[test]
    fn test_mcp_metrics_creation() {
        let meter = get_test_meter();
        let _metrics = McpMetrics::new(&meter);
    }

    #[test]
    fn test_mcp_metrics_record_tool_call() {
        let meter = get_test_meter();
        let metrics = McpMetrics::new(&meter);

        metrics.record_tool_call("read_file", "filesystem", 50.0, true);
        metrics.record_tool_call("write_file", "filesystem", 100.0, false);
    }

    #[test]
    fn test_mcp_metrics_permission_denial() {
        let meter = get_test_meter();
        let metrics = McpMetrics::new(&meter);

        metrics.record_permission_denial("execute_command", "not_in_allow_list");
    }

    #[test]
    fn test_mcp_metrics_server_connections() {
        let meter = get_test_meter();
        let metrics = McpMetrics::new(&meter);

        assert_eq!(metrics.get_connection_count(), 0);

        metrics.record_server_connection("server1", true);
        assert_eq!(metrics.get_connection_count(), 1);

        metrics.record_server_connection("server2", true);
        assert_eq!(metrics.get_connection_count(), 2);

        metrics.record_server_connection("server1", false);
        assert_eq!(metrics.get_connection_count(), 1);
    }

    #[test]
    fn test_mcp_metrics_cache_hit_ratio() {
        let meter = get_test_meter();
        let metrics = McpMetrics::new(&meter);

        metrics.record_cache_hit_ratio("search", 0.75);
        metrics.record_cache_hit_ratio("read_file", 0.95);
    }

    #[test]
    fn test_observability_metrics_creation() {
        let meter = get_test_meter();
        let metrics = ObservabilityMetrics::new(&meter);

        // Test both sub-metrics
        let usage = TokenUsage::new(100, 50);
        metrics.genai.record_request("test-model", &usage, 100.0, true);
        metrics.mcp.record_tool_call("test-tool", "test-server", 50.0, true);
    }

    #[test]
    fn test_token_usage_with_cache() {
        let meter = get_test_meter();
        let metrics = GenAiMetrics::new(&meter);

        let usage = TokenUsage::with_cache(1000, 500, 200);
        metrics.record_request("claude-3-5-sonnet", &usage, 200.0, true);
    }
}
