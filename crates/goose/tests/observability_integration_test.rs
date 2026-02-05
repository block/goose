//! Integration tests for the Observability module
//!
//! Tests the complete observability pipeline including cost tracking,
//! metrics collection, and export capabilities.

use goose::observability::{
    cost_tracker::{CostTracker, ModelPricing, TokenUsage},
    exporters::prometheus::{GrafanaDashboard, PrometheusExporter},
    metrics::ObservabilityMetrics,
    semantic_conventions::{gen_ai, goose as goose_conv, mcp},
    GenAiSpanBuilder, McpSpanBuilder, Observability, ObservabilityConfig, ReportFormat,
};
use opentelemetry::global;
use std::collections::HashMap;

// ============================================================================
// COST TRACKING INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_full_cost_tracking_pipeline() {
    let tracker = CostTracker::new();

    // Simulate a realistic session with multiple models
    let models = vec![
        (
            "claude-3-5-sonnet-20241022",
            TokenUsage::with_cache(1500, 800, 200),
        ),
        ("gpt-4o-mini", TokenUsage::new(500, 300)),
        (
            "claude-3-5-sonnet-20241022",
            TokenUsage::with_cache(2000, 1200, 500),
        ),
        ("gpt-4o", TokenUsage::new(1000, 600)),
    ];

    for (model, usage) in &models {
        tracker.record("session-integration", usage, model).await;
    }

    // Verify session cost
    let session = tracker
        .get_session_cost("session-integration")
        .await
        .unwrap();

    assert_eq!(session.requests.len(), 4);
    assert!(session.total_cost_usd > 0.0);
    assert_eq!(session.total_input_tokens, 5000);
    assert_eq!(session.total_output_tokens, 2900);
    assert_eq!(session.total_cached_tokens, 700);

    // Verify cost calculation is reasonable
    // Claude 3.5 Sonnet: ~$0.003/1K input, ~$0.015/1K output
    // GPT-4o-mini: ~$0.00015/1K input, ~$0.0006/1K output
    // GPT-4o: ~$0.0025/1K input, ~$0.01/1K output
    assert!(session.total_cost_usd > 0.01);
    assert!(session.total_cost_usd < 1.0);
}

#[tokio::test]
async fn test_multi_session_tracking() {
    let tracker = CostTracker::new();

    // Create multiple sessions
    let sessions = ["session-1", "session-2", "session-3"];

    for (i, session_id) in sessions.iter().enumerate() {
        let usage = TokenUsage::new(100 * (i as u64 + 1), 50 * (i as u64 + 1));
        tracker.record(session_id, &usage, "gpt-4o-mini").await;
    }

    // Verify each session
    for (i, session_id) in sessions.iter().enumerate() {
        let session = tracker.get_session_cost(session_id).await.unwrap();
        assert_eq!(session.total_input_tokens, 100 * (i as u64 + 1));
        assert_eq!(session.total_output_tokens, 50 * (i as u64 + 1));
    }

    // Verify totals
    let (total_input, total_output, _) = tracker.get_total_tokens().await;
    assert_eq!(total_input, 600); // 100 + 200 + 300
    assert_eq!(total_output, 300); // 50 + 100 + 150

    let total_requests = tracker.get_total_requests().await;
    assert_eq!(total_requests, 3);
}

#[tokio::test]
async fn test_custom_pricing_override() {
    let tracker = CostTracker::new();

    // Set custom pricing for a hypothetical model
    let custom_pricing = ModelPricing::with_cache_cost(0.05, 0.1, 0.01);
    tracker
        .set_pricing("custom-expensive-model", custom_pricing)
        .await;

    // Record usage with the custom model
    let usage = TokenUsage::new(1000, 1000);
    tracker
        .record("custom-session", &usage, "custom-expensive-model")
        .await;

    let session = tracker.get_session_cost("custom-session").await.unwrap();

    // 1K * 0.05 + 1K * 0.1 = 0.15
    assert!((session.total_cost_usd - 0.15).abs() < 0.0001);
}

#[tokio::test]
async fn test_cache_cost_savings() {
    let tracker = CostTracker::new();

    // Test without cache
    let usage_no_cache = TokenUsage::new(1000, 500);
    let cost_no_cache = tracker.calculate_cost(&usage_no_cache, "claude-3-5-sonnet-20241022");

    // Test with 50% cache
    let usage_with_cache = TokenUsage::with_cache(1000, 500, 500);
    let cost_with_cache = tracker.calculate_cost(&usage_with_cache, "claude-3-5-sonnet-20241022");

    // Cached version should be cheaper
    assert!(cost_with_cache < cost_no_cache);

    // Calculate expected savings
    // Regular: 500 tokens * $0.003/1K = $0.0015
    // Cached: 500 tokens * $0.00075/1K = $0.000375
    // Savings: $0.001125
    let expected_savings = (500.0 / 1000.0) * (0.003 - 0.00075);
    let actual_savings = cost_no_cache - cost_with_cache;
    assert!((actual_savings - expected_savings).abs() < 0.0001);
}

// ============================================================================
// EXPORT FORMAT TESTS
// ============================================================================

#[tokio::test]
async fn test_json_export_format() {
    let tracker = CostTracker::new();

    tracker
        .record("json-test", &TokenUsage::new(100, 50), "gpt-4o-mini")
        .await;

    let json = tracker.export_report(ReportFormat::Json).await.unwrap();

    // Parse and verify structure
    let parsed: HashMap<String, serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert!(parsed.contains_key("json-test"));
}

#[tokio::test]
async fn test_csv_export_format() {
    let tracker = CostTracker::new();

    tracker
        .record("csv-test", &TokenUsage::new(100, 50), "gpt-4o-mini")
        .await;

    let csv = tracker.export_report(ReportFormat::Csv).await.unwrap();

    // Verify CSV structure
    assert!(csv.contains("session_id,timestamp,model,input_tokens,output_tokens"));
    assert!(csv.contains("csv-test"));
    assert!(csv.contains("gpt-4o-mini"));
    assert!(csv.contains("100"));
    assert!(csv.contains("50"));
}

#[tokio::test]
async fn test_markdown_export_format() {
    let tracker = CostTracker::new();

    tracker
        .record("md-test", &TokenUsage::new(100, 50), "gpt-4o-mini")
        .await;

    let md = tracker.export_report(ReportFormat::Markdown).await.unwrap();

    // Verify Markdown structure
    assert!(md.contains("# Cost Report"));
    assert!(md.contains("## Summary"));
    assert!(md.contains("## Sessions"));
    assert!(md.contains("md-test"));
    assert!(md.contains("**Total Cost:**"));
}

// ============================================================================
// PROMETHEUS EXPORT TESTS
// ============================================================================

#[tokio::test]
async fn test_prometheus_export_complete() {
    let tracker = CostTracker::new();
    let exporter = PrometheusExporter::new();

    // Add test data
    tracker
        .record(
            "prom-session-1",
            &TokenUsage::new(1000, 500),
            "claude-3-5-sonnet-20241022",
        )
        .await;
    tracker
        .record("prom-session-1", &TokenUsage::new(500, 250), "gpt-4o-mini")
        .await;
    tracker
        .record(
            "prom-session-2",
            &TokenUsage::new(2000, 1000),
            "claude-3-5-sonnet-20241022",
        )
        .await;

    let output = exporter.export_cost_metrics(&tracker).await;

    // Verify all expected metrics are present
    assert!(output.contains("# HELP goose_cost_total_usd"));
    assert!(output.contains("# TYPE goose_cost_total_usd counter"));
    assert!(output.contains("goose_requests_total"));
    assert!(output.contains("goose_tokens_input_total"));
    assert!(output.contains("goose_tokens_output_total"));
    assert!(output.contains("goose_sessions_active"));

    // Verify per-session metrics
    assert!(output.contains("session_id=\"prom-session-1\""));
    assert!(output.contains("session_id=\"prom-session-2\""));

    // Verify per-model metrics
    assert!(output.contains("model=\"claude-3-5-sonnet-20241022\""));
    assert!(output.contains("model=\"gpt-4o-mini\""));
}

#[tokio::test]
async fn test_grafana_dashboard_export() {
    let dashboard = GrafanaDashboard::goose_default();
    let json = dashboard.to_grafana_json().unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify structure
    assert!(parsed["dashboard"]["title"]
        .as_str()
        .unwrap()
        .contains("Goose"));
    assert!(parsed["dashboard"]["panels"].is_array());

    let panels = parsed["dashboard"]["panels"].as_array().unwrap();
    assert!(!panels.is_empty());

    // Verify panel structure
    for panel in panels {
        assert!(panel["id"].is_number());
        assert!(panel["title"].is_string());
        assert!(panel["type"].is_string());
        assert!(panel["targets"].is_array());
    }
}

// ============================================================================
// OBSERVABILITY ORCHESTRATOR TESTS
// ============================================================================

#[tokio::test]
async fn test_observability_orchestrator() {
    let config = ObservabilityConfig {
        cost_tracking_enabled: true,
        mcp_metrics_enabled: true,
        genai_conventions_enabled: true,
        metrics_export_interval_secs: 60,
        pricing_overrides: HashMap::new(),
    };

    let obs = Observability::with_config(config);

    // Record GenAI request
    let usage = TokenUsage::new(1000, 500);
    obs.record_genai_request(
        "orchestrator-session",
        "claude-3-5-sonnet-20241022",
        &usage,
        150.0,
        true,
    )
    .await;

    // Record MCP operations
    obs.record_mcp_tool_call("read_file", "filesystem", 25.0, true);
    obs.record_mcp_tool_call("write_file", "filesystem", 50.0, false);
    obs.record_mcp_permission_denial("execute_command", "not_in_allow_list");
    obs.record_mcp_server_connection("filesystem", true);

    // Verify cost tracking
    let session_cost = obs.get_session_cost("orchestrator-session").await.unwrap();
    assert!(session_cost.total_cost_usd > 0.0);
    assert_eq!(session_cost.total_input_tokens, 1000);

    // Get metrics snapshot
    let snapshot = obs.get_metrics_snapshot().await;
    assert!(snapshot.total_cost_usd > 0.0);
    assert_eq!(snapshot.total_requests, 1);
    assert_eq!(snapshot.total_input_tokens, 1000);
    assert_eq!(snapshot.total_output_tokens, 500);

    // Export cost report
    let report = obs.export_cost_report(ReportFormat::Json).await.unwrap();
    assert!(report.contains("orchestrator-session"));
}

#[tokio::test]
async fn test_observability_disabled_features() {
    let config = ObservabilityConfig {
        cost_tracking_enabled: false,
        mcp_metrics_enabled: false,
        genai_conventions_enabled: false,
        metrics_export_interval_secs: 60,
        pricing_overrides: HashMap::new(),
    };

    let obs = Observability::with_config(config);

    // Record operations (should be no-ops)
    let usage = TokenUsage::new(1000, 500);
    obs.record_genai_request(
        "disabled-session",
        "claude-3-5-sonnet-20241022",
        &usage,
        150.0,
        true,
    )
    .await;
    obs.record_mcp_tool_call("test", "server", 10.0, true);

    // Cost tracking is disabled, so session should not exist
    let session_cost = obs.get_session_cost("disabled-session").await;
    assert!(session_cost.is_none());
}

// ============================================================================
// SEMANTIC CONVENTIONS TESTS
// ============================================================================

#[test]
fn test_genai_span_builder_complete() {
    let attrs = GenAiSpanBuilder::new()
        .system(gen_ai::SYSTEM_ANTHROPIC)
        .request_model("claude-3-5-sonnet-20241022")
        .max_tokens(4096)
        .temperature(0.7)
        .top_p(0.9)
        .response_id("msg_12345")
        .response_model("claude-3-5-sonnet-20241022")
        .finish_reasons(vec![gen_ai::FINISH_REASON_STOP.to_string()])
        .input_tokens(1000)
        .output_tokens(500)
        .total_tokens(1500)
        .cached_tokens(200)
        .cost_usd(0.0225)
        .tool_name("read_file")
        .tool_call_id("call_abc123")
        .build();

    // Verify all attributes are present (15 total)
    assert_eq!(attrs.len(), 15);
}

#[test]
fn test_mcp_span_builder_complete() {
    let attrs = McpSpanBuilder::new()
        .server_name("filesystem")
        .server_version("1.0.0")
        .transport_type(mcp::TRANSPORT_STDIO)
        .tool_count(10)
        .resource_count(5)
        .prompt_count(3)
        .build();

    assert_eq!(attrs.len(), 6);
}

#[test]
fn test_semantic_convention_values() {
    // GenAI conventions
    assert_eq!(gen_ai::SYSTEM, "gen_ai.system");
    assert_eq!(gen_ai::USAGE_COST_USD, "gen_ai.usage.cost_usd");
    assert_eq!(gen_ai::SYSTEM_ANTHROPIC, "anthropic");
    assert_eq!(gen_ai::FINISH_REASON_TOOL_CALLS, "tool_calls");

    // MCP conventions
    assert_eq!(mcp::SERVER_NAME, "mcp.server.name");
    assert_eq!(mcp::TRANSPORT_STDIO, "stdio");
    assert_eq!(mcp::STATUS_CONNECTED, "connected");

    // Goose conventions
    assert_eq!(goose_conv::SESSION_ID, "goose.session.id");
    assert_eq!(goose_conv::GUARDRAIL_DETECTOR, "goose.guardrail.detector");
    assert_eq!(goose_conv::SESSION_COST_USD, "goose.session.cost_usd");
}

// ============================================================================
// METRICS TESTS
// ============================================================================

#[test]
fn test_metrics_recording() {
    let meter = global::meter("integration_test");
    let metrics = ObservabilityMetrics::new(&meter);

    // Record GenAI metrics
    let usage = TokenUsage::with_cache(1000, 500, 200);
    metrics
        .genai
        .record_request("claude-3-5-sonnet", &usage, 150.0, true);
    metrics
        .genai
        .record_request("gpt-4o", &TokenUsage::new(500, 250), 100.0, false);
    metrics
        .genai
        .record_error("claude-3-5-sonnet", "rate_limit");

    // Record MCP metrics
    metrics
        .mcp
        .record_tool_call("read_file", "filesystem", 25.0, true);
    metrics
        .mcp
        .record_tool_call("write_file", "filesystem", 50.0, false);
    metrics
        .mcp
        .record_permission_denial("execute_command", "not_in_allow_list");
    metrics.mcp.record_server_connection("filesystem", true);
    metrics.mcp.record_cache_hit_ratio("search", 0.85);

    // Verify connection count tracking
    assert_eq!(metrics.mcp.get_connection_count(), 1);
    metrics.mcp.record_server_connection("database", true);
    assert_eq!(metrics.mcp.get_connection_count(), 2);
    metrics.mcp.record_server_connection("filesystem", false);
    assert_eq!(metrics.mcp.get_connection_count(), 1);
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[tokio::test]
async fn test_empty_session() {
    let tracker = CostTracker::new();

    // Non-existent session should return None
    let session = tracker.get_session_cost("non-existent").await;
    assert!(session.is_none());
}

#[tokio::test]
async fn test_clear_operations() {
    let tracker = CostTracker::new();

    // Add data
    tracker
        .record("session-1", &TokenUsage::new(100, 50), "gpt-4o-mini")
        .await;
    tracker
        .record("session-2", &TokenUsage::new(100, 50), "gpt-4o-mini")
        .await;

    // Clear one session
    tracker.clear_session("session-1").await;
    assert!(tracker.get_session_cost("session-1").await.is_none());
    assert!(tracker.get_session_cost("session-2").await.is_some());

    // Clear all
    tracker.clear().await;
    assert!(tracker.get_session_cost("session-2").await.is_none());
    assert_eq!(tracker.get_total_cost().await, 0.0);
}

#[tokio::test]
async fn test_very_large_token_counts() {
    let tracker = CostTracker::new();

    // Simulate a very large request (1M tokens each)
    let usage = TokenUsage::new(1_000_000, 1_000_000);
    tracker
        .record("large-session", &usage, "claude-3-5-sonnet-20241022")
        .await;

    let session = tracker.get_session_cost("large-session").await.unwrap();

    // Claude 3.5 Sonnet: $0.003/1K input, $0.015/1K output
    // 1M input: $3.00, 1M output: $15.00 = $18.00
    assert!((session.total_cost_usd - 18.0).abs() < 0.01);
}

#[test]
fn test_free_models() {
    let tracker = CostTracker::new();

    // Ollama models should be free
    let usage = TokenUsage::new(10_000_000, 10_000_000);
    let cost_ollama = tracker.calculate_cost(&usage, "ollama/llama3");
    assert_eq!(cost_ollama, 0.0);

    // Local models should be free
    let cost_local = tracker.calculate_cost(&usage, "local/my-custom-model");
    assert_eq!(cost_local, 0.0);
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[tokio::test]
async fn test_high_throughput_recording() {
    let tracker = CostTracker::new();
    let start = std::time::Instant::now();

    // Record 1000 requests
    for i in 0..1000 {
        let usage = TokenUsage::new(100, 50);
        tracker
            .record(&format!("perf-session-{}", i % 10), &usage, "gpt-4o-mini")
            .await;
    }

    let elapsed = start.elapsed();

    // Should complete in under 1 second
    assert!(
        elapsed.as_secs() < 1,
        "Recording 1000 requests took {:?}",
        elapsed
    );

    // Verify data integrity
    assert_eq!(tracker.get_total_requests().await, 1000);
}

#[tokio::test]
async fn test_export_performance() {
    let tracker = CostTracker::new();

    // Create substantial data
    for i in 0..100 {
        let usage = TokenUsage::with_cache(1000, 500, 200);
        tracker
            .record(
                &format!("export-perf-{}", i % 10),
                &usage,
                "claude-3-5-sonnet-20241022",
            )
            .await;
    }

    let exporter = PrometheusExporter::new();

    let start = std::time::Instant::now();
    let _ = exporter.export_cost_metrics(&tracker).await;
    let elapsed = start.elapsed();

    // Export should be fast (under 100ms)
    assert!(elapsed.as_millis() < 100, "Export took {:?}", elapsed);
}
