# Goose Observability

## Overview

The Observability module provides comprehensive monitoring, metrics, and cost tracking for AI operations. It implements OpenTelemetry GenAI semantic conventions and supports multiple export formats.

## Features

- **OpenTelemetry GenAI Conventions**: Standard span attributes for AI operations
- **Token Cost Tracking**: Real-time cost calculation with model-specific pricing
- **MCP Metrics**: Specialized metrics for MCP server operations
- **Multiple Export Formats**: Prometheus, JSON, CSV, Markdown, Grafana dashboards
- **Session Tracking**: Per-session cost and usage aggregation

## Quick Start

```rust
use goose::observability::{
    ObservabilityOrchestrator, ObservabilityConfig,
    CostTracker, TokenUsage
};

// Create orchestrator
let config = ObservabilityConfig::default();
let orchestrator = ObservabilityOrchestrator::new(config);

// Record token usage
let usage = TokenUsage {
    input_tokens: 1000,
    output_tokens: 500,
    cached_tokens: 200,
};

orchestrator.record_usage("session-123", &usage, "claude-3-5-sonnet-20241022").await;

// Get session cost
let session_cost = orchestrator.get_session_cost("session-123").await?;
println!("Total cost: ${:.4}", session_cost.total_cost_usd);
```

## Semantic Conventions

### GenAI Span Attributes

```rust
use goose::observability::semantic_conventions::gen_ai;

// Standard GenAI attributes
gen_ai::SYSTEM                    // "gen_ai.system"
gen_ai::REQUEST_MODEL            // "gen_ai.request.model"
gen_ai::REQUEST_MAX_TOKENS       // "gen_ai.request.max_tokens"
gen_ai::REQUEST_TEMPERATURE      // "gen_ai.request.temperature"
gen_ai::RESPONSE_ID              // "gen_ai.response.id"
gen_ai::RESPONSE_MODEL           // "gen_ai.response.model"
gen_ai::RESPONSE_FINISH_REASONS  // "gen_ai.response.finish_reasons"

// Token usage
gen_ai::USAGE_INPUT_TOKENS       // "gen_ai.usage.input_tokens"
gen_ai::USAGE_OUTPUT_TOKENS      // "gen_ai.usage.output_tokens"
gen_ai::USAGE_TOTAL_TOKENS       // "gen_ai.usage.total_tokens"
gen_ai::USAGE_CACHED_TOKENS      // "gen_ai.usage.cached_tokens"
gen_ai::USAGE_COST_USD           // "gen_ai.usage.cost_usd"

// Tool calling
gen_ai::TOOL_NAME                // "gen_ai.tool.name"
gen_ai::TOOL_CALL_ID             // "gen_ai.tool.call_id"
```

### MCP Span Attributes

```rust
use goose::observability::semantic_conventions::mcp;

mcp::SERVER_NAME      // "mcp.server.name"
mcp::SERVER_VERSION   // "mcp.server.version"
mcp::TRANSPORT_TYPE   // "mcp.transport.type"
mcp::TOOL_COUNT       // "mcp.tools.count"
mcp::RESOURCE_COUNT   // "mcp.resources.count"
```

### Creating Spans

```rust
use goose::observability::GenAiSpanBuilder;

let span = GenAiSpanBuilder::new("chat_completion")
    .with_system("anthropic")
    .with_request_model("claude-3-5-sonnet-20241022")
    .with_max_tokens(4096)
    .with_temperature(0.7)
    .build();

// After response
span.set_response_id("resp_123");
span.set_usage(1000, 500, 200);  // input, output, cached
span.set_cost(0.045);
```

## Cost Tracking

### Model Pricing

Built-in pricing for popular models:

```rust
// Anthropic models
"claude-3-opus-20240229"      // $15.00/$75.00 per 1M tokens (input/output)
"claude-3-5-sonnet-20241022"  // $3.00/$15.00 per 1M tokens
"claude-3-5-haiku-20241022"   // $0.80/$4.00 per 1M tokens

// OpenAI models
"gpt-4o"                      // $2.50/$10.00 per 1M tokens
"gpt-4o-mini"                 // $0.15/$0.60 per 1M tokens
"gpt-4-turbo"                 // $10.00/$30.00 per 1M tokens

// Google models
"gemini-1.5-pro"              // $1.25/$5.00 per 1M tokens
"gemini-1.5-flash"            // $0.075/$0.30 per 1M tokens
```

### Custom Pricing

```rust
use goose::observability::cost_tracker::{CostTracker, ModelPricing};

let tracker = CostTracker::new();

// Add custom pricing
tracker.set_pricing("my-custom-model", ModelPricing {
    input_per_1k: 0.001,
    output_per_1k: 0.002,
    cached_input_per_1k: 0.0005,
}).await;
```

### Recording Usage

```rust
let usage = TokenUsage {
    input_tokens: 1500,
    output_tokens: 800,
    cached_tokens: 500,  // Cached tokens reduce cost
};

tracker.record("session-123", &usage, "claude-3-5-sonnet-20241022").await;
```

### Cost Calculation

```rust
// Calculate cost for a single request
let cost = tracker.calculate_cost(&usage, "claude-3-5-sonnet-20241022");

// Get session summary
let session = tracker.get_session_cost("session-123").await?;
println!("Total cost: ${:.4}", session.total_cost_usd);
println!("Input tokens: {}", session.total_input_tokens);
println!("Output tokens: {}", session.total_output_tokens);
println!("Cached tokens: {}", session.total_cached_tokens);
println!("Requests: {}", session.requests.len());
```

## Metrics

### Recording Metrics

```rust
use goose::observability::metrics::McpMetrics;
use opentelemetry::metrics::Meter;

let meter = /* get OpenTelemetry meter */;
let metrics = McpMetrics::new(&meter);

// Record tool call
metrics.record_tool_call(
    "read_file",           // tool name
    "filesystem-server",   // server name
    25.5,                  // duration (ms)
    true                   // success
);

// Record permission denial
metrics.record_permission_denial("write_file", "Insufficient permissions");

// Update server connections
metrics.update_server_connections(5);
```

### Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `mcp.tool.calls` | Counter | Number of MCP tool calls |
| `mcp.tool.duration` | Histogram | Duration of tool calls (ms) |
| `mcp.server.connections` | UpDownCounter | Active server connections |
| `mcp.permission.denials` | Counter | Permission denial count |
| `mcp.cache.hit_ratio` | Histogram | Tool result cache hit ratio |

## Export Formats

### Prometheus Export

```rust
use goose::observability::exporters::PrometheusExporter;

let exporter = PrometheusExporter::new();

// Add metrics
exporter.add_gauge("goose_sessions_active", 5.0, &[]);
exporter.add_counter("goose_requests_total", 1000.0, &[("model", "claude-3-5-sonnet")]);

// Export as Prometheus format
let output = exporter.export();
// # HELP goose_sessions_active Active sessions
// # TYPE goose_sessions_active gauge
// goose_sessions_active 5
// ...
```

### JSON Export

```rust
let report = tracker.export_report(ReportFormat::Json).await?;
// {"sessions": {"session-123": {"total_cost_usd": 0.045, ...}}}
```

### CSV Export

```rust
let report = tracker.export_report(ReportFormat::Csv).await?;
// session_id,timestamp,model,input_tokens,output_tokens,cached_tokens,cost_usd
// session-123,2024-01-01T00:00:00Z,claude-3-5-sonnet-20241022,1000,500,200,0.0045
```

### Markdown Export

```rust
let report = tracker.export_report(ReportFormat::Markdown).await?;
// # Cost Report
// ## Session: session-123
// | Metric | Value |
// |--------|-------|
// | Total Cost | $0.0045 |
// ...
```

### Grafana Dashboard

```rust
use goose::observability::exporters::GrafanaDashboard;

let dashboard = GrafanaDashboard::generate("Goose Metrics");
// Returns JSON for Grafana dashboard import
```

## Configuration

### ObservabilityConfig

```rust
pub struct ObservabilityConfig {
    /// Enable cost tracking
    pub cost_tracking_enabled: bool,

    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Enable span creation
    pub tracing_enabled: bool,

    /// Default export format
    pub default_export_format: ReportFormat,

    /// Session cost retention (hours)
    pub session_retention_hours: u64,
}
```

## Session Management

```rust
// Create orchestrator
let orchestrator = ObservabilityOrchestrator::new(config);

// Record to session
orchestrator.record_usage("session-1", &usage, "model").await;

// Get all sessions
let sessions = orchestrator.list_sessions().await;

// Clear session data
orchestrator.clear_session("session-1").await;

// Clear all sessions
orchestrator.clear_all_sessions().await;
```

## Integration Example

```rust
use goose::observability::{
    ObservabilityOrchestrator, ObservabilityConfig, TokenUsage,
    semantic_conventions::gen_ai, GenAiSpanBuilder
};

async fn process_request(prompt: &str) -> Result<String> {
    let orchestrator = ObservabilityOrchestrator::default();

    // Create span
    let span = GenAiSpanBuilder::new("chat_completion")
        .with_system("anthropic")
        .with_request_model("claude-3-5-sonnet-20241022")
        .build();

    // Make API call
    let response = call_api(prompt).await?;

    // Record usage
    let usage = TokenUsage {
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        cached_tokens: response.usage.cache_read_input_tokens,
    };

    orchestrator.record_usage(
        &session_id,
        &usage,
        "claude-3-5-sonnet-20241022"
    ).await;

    // Update span
    span.set_usage(
        usage.input_tokens,
        usage.output_tokens,
        usage.cached_tokens
    );
    span.set_cost(orchestrator.calculate_cost(&usage, "claude-3-5-sonnet-20241022"));

    Ok(response.content)
}
```

## Testing

```bash
# Run observability unit tests
cargo test --package goose observability::

# Run integration tests
cargo test --package goose --test observability_integration_test
```

## See Also

- [Enterprise Integration Action Plan](07_ENTERPRISE_INTEGRATION_ACTION_PLAN.md)
- [Comprehensive Audit Report](08_COMPREHENSIVE_AUDIT_REPORT.md)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
