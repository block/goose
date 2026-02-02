//! Execution observability and cost tracking for agentic workflows
//!
//! This module provides:
//! - Token usage tracking per LLM call
//! - Cost estimation based on model pricing
//! - Execution span tracking (traces)
//! - Performance metrics collection
//! - Export capabilities for analysis tools

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Unique identifier for a trace
pub type TraceId = String;

/// Unique identifier for a span within a trace
pub type SpanId = String;

/// Token usage for a single LLM call
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input/prompt tokens
    pub input_tokens: u64,
    /// Output/completion tokens
    pub output_tokens: u64,
    /// Cached/reused tokens (if supported)
    pub cached_tokens: u64,
}

impl TokenUsage {
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cached_tokens: 0,
        }
    }

    pub fn with_cached(mut self, cached: u64) -> Self {
        self.cached_tokens = cached;
        self
    }

    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Add another TokenUsage to this one
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cached_tokens += other.cached_tokens;
    }
}

impl std::ops::Add for TokenUsage {
    type Output = TokenUsage;

    fn add(self, other: TokenUsage) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            cached_tokens: self.cached_tokens + other.cached_tokens,
        }
    }
}

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, other: TokenUsage) {
        self.add(&other);
    }
}

/// Pricing per million tokens for a model
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Price per million input tokens
    pub input_per_million: f64,
    /// Price per million output tokens
    pub output_per_million: f64,
    /// Price per million cached tokens (usually discounted)
    pub cached_per_million: f64,
}

impl ModelPricing {
    pub fn new(input: f64, output: f64) -> Self {
        Self {
            input_per_million: input,
            output_per_million: output,
            cached_per_million: input * 0.5, // Default: 50% of input price
        }
    }

    pub fn with_cached_price(mut self, cached: f64) -> Self {
        self.cached_per_million = cached;
        self
    }

    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        let cached_cost = (usage.cached_tokens as f64 / 1_000_000.0) * self.cached_per_million;
        input_cost + output_cost + cached_cost
    }
}

impl Default for ModelPricing {
    fn default() -> Self {
        // Default to Claude 3.5 Sonnet pricing
        Self::new(3.0, 15.0)
    }
}

/// Common model pricing configurations
pub mod pricing {
    use super::ModelPricing;

    pub fn claude_opus() -> ModelPricing {
        ModelPricing::new(15.0, 75.0)
    }

    pub fn claude_sonnet() -> ModelPricing {
        ModelPricing::new(3.0, 15.0)
    }

    pub fn claude_haiku() -> ModelPricing {
        ModelPricing::new(0.25, 1.25)
    }

    pub fn gpt4o() -> ModelPricing {
        ModelPricing::new(2.5, 10.0)
    }

    pub fn gpt4o_mini() -> ModelPricing {
        ModelPricing::new(0.15, 0.6)
    }

    pub fn gpt4_turbo() -> ModelPricing {
        ModelPricing::new(10.0, 30.0)
    }

    pub fn gemini_pro() -> ModelPricing {
        ModelPricing::new(1.25, 5.0)
    }

    pub fn gemini_flash() -> ModelPricing {
        ModelPricing::new(0.075, 0.3)
    }
}

/// Type of span in the execution trace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpanType {
    /// LLM inference call
    LlmCall,
    /// Tool/function call
    ToolCall,
    /// Planning phase
    Planning,
    /// Critique/evaluation phase
    Critique,
    /// Reasoning step
    Reasoning,
    /// State transition
    StateTransition,
    /// External API call
    ExternalApi,
    /// Custom/other
    Custom,
}

impl std::fmt::Display for SpanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanType::LlmCall => write!(f, "llm_call"),
            SpanType::ToolCall => write!(f, "tool_call"),
            SpanType::Planning => write!(f, "planning"),
            SpanType::Critique => write!(f, "critique"),
            SpanType::Reasoning => write!(f, "reasoning"),
            SpanType::StateTransition => write!(f, "state_transition"),
            SpanType::ExternalApi => write!(f, "external_api"),
            SpanType::Custom => write!(f, "custom"),
        }
    }
}

/// A span represents a single operation in the execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique identifier for this span
    pub span_id: SpanId,
    /// Parent span ID (for nesting)
    pub parent_id: Option<SpanId>,
    /// Name/description of the span
    pub name: String,
    /// Type of operation
    pub span_type: SpanType,
    /// Input to the operation (serialized)
    pub input: Option<serde_json::Value>,
    /// Output from the operation (serialized)
    pub output: Option<serde_json::Value>,
    /// Token usage (for LLM calls)
    pub tokens: Option<TokenUsage>,
    /// Duration of the operation
    pub duration: Duration,
    /// Model used (for LLM calls)
    pub model: Option<String>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: DateTime<Utc>,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Custom attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Span {
    /// Create a new span (use SpanBuilder for easier construction)
    pub fn new(name: impl Into<String>, span_type: SpanType) -> Self {
        let now = Utc::now();
        Self {
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            name: name.into(),
            span_type,
            input: None,
            output: None,
            tokens: None,
            duration: Duration::ZERO,
            model: None,
            started_at: now,
            ended_at: now,
            success: true,
            error: None,
            attributes: HashMap::new(),
        }
    }

    /// Set the span as completed
    pub fn complete(&mut self, success: bool, error: Option<String>) {
        self.ended_at = Utc::now();
        self.duration = self
            .ended_at
            .signed_duration_since(self.started_at)
            .to_std()
            .unwrap_or(Duration::ZERO);
        self.success = success;
        self.error = error;
    }
}

/// Builder for creating spans
pub struct SpanBuilder {
    span: Span,
}

impl SpanBuilder {
    pub fn new(name: impl Into<String>, span_type: SpanType) -> Self {
        Self {
            span: Span::new(name, span_type),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.span.parent_id = Some(parent_id.into());
        self
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.span.input = Some(input);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.span.model = Some(model.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.span.attributes.insert(key.into(), value);
        self
    }

    pub fn build(self) -> Span {
        self.span
    }
}

/// Complete execution trace for a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// Unique identifier for this trace
    pub trace_id: TraceId,
    /// Name/description of the workflow
    pub name: String,
    /// All spans in the trace
    pub spans: Vec<Span>,
    /// Aggregated metrics
    pub metrics: ExecutionMetrics,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time (if completed)
    pub ended_at: Option<DateTime<Utc>>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExecutionTrace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            spans: Vec::new(),
            metrics: ExecutionMetrics::default(),
            started_at: Utc::now(),
            ended_at: None,
            metadata: HashMap::new(),
        }
    }

    pub fn add_span(&mut self, span: Span) {
        // Update metrics
        if let Some(tokens) = &span.tokens {
            self.metrics.total_tokens += *tokens;
        }

        match span.span_type {
            SpanType::LlmCall => self.metrics.llm_calls += 1,
            SpanType::ToolCall => self.metrics.tool_calls += 1,
            _ => {}
        }

        if !span.success {
            self.metrics.retries += 1;
        }

        self.metrics.total_duration += span.duration;
        self.spans.push(span);
    }

    pub fn complete(&mut self) {
        self.ended_at = Some(Utc::now());
    }

    /// Calculate the total cost based on pricing
    pub fn calculate_cost(&self, pricing: &ModelPricing) -> f64 {
        pricing.calculate_cost(&self.metrics.total_tokens)
    }

    /// Get spans of a specific type
    pub fn spans_of_type(&self, span_type: SpanType) -> Vec<&Span> {
        self.spans
            .iter()
            .filter(|s| s.span_type == span_type)
            .collect()
    }

    /// Get the root spans (no parent)
    pub fn root_spans(&self) -> Vec<&Span> {
        self.spans
            .iter()
            .filter(|s| s.parent_id.is_none())
            .collect()
    }

    /// Get child spans of a parent
    pub fn child_spans(&self, parent_id: &str) -> Vec<&Span> {
        self.spans
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(parent_id))
            .collect()
    }
}

/// Aggregated metrics for an execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total token usage
    pub total_tokens: TokenUsage,
    /// Estimated cost in USD
    pub estimated_cost_usd: f64,
    /// Total duration
    pub total_duration: Duration,
    /// Number of tool calls
    pub tool_calls: usize,
    /// Number of LLM calls
    pub llm_calls: usize,
    /// Number of retries/failures
    pub retries: usize,
}

impl ExecutionMetrics {
    /// Update the estimated cost based on pricing
    pub fn update_cost(&mut self, pricing: &ModelPricing) {
        self.estimated_cost_usd = pricing.calculate_cost(&self.total_tokens);
    }

    /// Format as a human-readable summary
    pub fn format_summary(&self) -> String {
        format!(
            "Tokens: {} (in: {}, out: {}) | Cost: ${:.4} | Duration: {:.1}s | LLM calls: {} | Tool calls: {}",
            self.total_tokens.total(),
            self.total_tokens.input_tokens,
            self.total_tokens.output_tokens,
            self.estimated_cost_usd,
            self.total_duration.as_secs_f64(),
            self.llm_calls,
            self.tool_calls
        )
    }
}

/// Thread-safe cost tracker for real-time monitoring
pub struct CostTracker {
    /// Accumulated tokens
    input_tokens: AtomicU64,
    output_tokens: AtomicU64,
    cached_tokens: AtomicU64,
    /// Number of calls
    llm_calls: AtomicU64,
    tool_calls: AtomicU64,
    /// Pricing to use
    pricing: RwLock<ModelPricing>,
    /// Budget limit (if set)
    budget_limit: RwLock<Option<f64>>,
}

impl CostTracker {
    pub fn new(pricing: ModelPricing) -> Self {
        Self {
            input_tokens: AtomicU64::new(0),
            output_tokens: AtomicU64::new(0),
            cached_tokens: AtomicU64::new(0),
            llm_calls: AtomicU64::new(0),
            tool_calls: AtomicU64::new(0),
            pricing: RwLock::new(pricing),
            budget_limit: RwLock::new(None),
        }
    }

    pub fn with_default_pricing() -> Self {
        Self::new(ModelPricing::default())
    }

    /// Record token usage from an LLM call
    pub fn record_llm_call(&self, tokens: &TokenUsage) {
        self.input_tokens
            .fetch_add(tokens.input_tokens, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(tokens.output_tokens, Ordering::Relaxed);
        self.cached_tokens
            .fetch_add(tokens.cached_tokens, Ordering::Relaxed);
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a tool call
    pub fn record_tool_call(&self) {
        self.tool_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current token usage
    pub fn get_tokens(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            cached_tokens: self.cached_tokens.load(Ordering::Relaxed),
        }
    }

    /// Get current estimated cost
    pub async fn get_cost(&self) -> f64 {
        let tokens = self.get_tokens();
        let pricing = self.pricing.read().await;
        pricing.calculate_cost(&tokens)
    }

    /// Set a budget limit
    pub async fn set_budget(&self, limit: f64) {
        let mut budget = self.budget_limit.write().await;
        *budget = Some(limit);
    }

    /// Check if we're over budget
    pub async fn is_over_budget(&self) -> bool {
        let budget = self.budget_limit.read().await;
        if let Some(limit) = *budget {
            self.get_cost().await > limit
        } else {
            false
        }
    }

    /// Get remaining budget (if set)
    pub async fn remaining_budget(&self) -> Option<f64> {
        let budget = self.budget_limit.read().await;
        if let Some(limit) = *budget {
            Some(limit - self.get_cost().await)
        } else {
            None
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.input_tokens.store(0, Ordering::Relaxed);
        self.output_tokens.store(0, Ordering::Relaxed);
        self.cached_tokens.store(0, Ordering::Relaxed);
        self.llm_calls.store(0, Ordering::Relaxed);
        self.tool_calls.store(0, Ordering::Relaxed);
    }

    /// Get summary statistics
    pub async fn get_summary(&self) -> String {
        let tokens = self.get_tokens();
        let cost = self.get_cost().await;
        let llm_calls = self.llm_calls.load(Ordering::Relaxed);
        let tool_calls = self.tool_calls.load(Ordering::Relaxed);

        format!(
            "Tokens: {} (in: {}, out: {}) | Cost: ${:.4} | LLM: {} | Tools: {}",
            tokens.total(),
            tokens.input_tokens,
            tokens.output_tokens,
            cost,
            llm_calls,
            tool_calls
        )
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::with_default_pricing()
    }
}

/// Manages execution traces and provides observability
pub struct ExecutionTracer {
    /// Current active trace
    current_trace: RwLock<Option<ExecutionTrace>>,
    /// History of completed traces
    trace_history: RwLock<Vec<ExecutionTrace>>,
    /// Cost tracker for real-time monitoring
    cost_tracker: Arc<CostTracker>,
    /// Default pricing
    pricing: ModelPricing,
}

impl ExecutionTracer {
    pub fn new(pricing: ModelPricing) -> Self {
        Self {
            current_trace: RwLock::new(None),
            trace_history: RwLock::new(Vec::new()),
            cost_tracker: Arc::new(CostTracker::new(pricing)),
            pricing,
        }
    }

    pub fn with_default_pricing() -> Self {
        Self::new(ModelPricing::default())
    }

    /// Start a new trace
    pub async fn start_trace(&self, name: impl Into<String>) -> TraceId {
        let trace = ExecutionTrace::new(name);
        let trace_id = trace.trace_id.clone();

        let mut current = self.current_trace.write().await;
        if let Some(old_trace) = current.take() {
            let mut history = self.trace_history.write().await;
            history.push(old_trace);
        }
        *current = Some(trace);

        // Reset cost tracker for new trace
        self.cost_tracker.reset();

        trace_id
    }

    /// Add a span to the current trace
    pub async fn add_span(&self, mut span: Span) {
        // Update cost tracker if this is an LLM call
        if span.span_type == SpanType::LlmCall {
            if let Some(tokens) = &span.tokens {
                self.cost_tracker.record_llm_call(tokens);
            }
        } else if span.span_type == SpanType::ToolCall {
            self.cost_tracker.record_tool_call();
        }

        let mut current = self.current_trace.write().await;
        if let Some(trace) = current.as_mut() {
            span.complete(span.success, span.error.clone());
            trace.add_span(span);
        }
    }

    /// Complete the current trace
    pub async fn complete_trace(&self) -> Option<ExecutionTrace> {
        let mut current = self.current_trace.write().await;
        if let Some(mut trace) = current.take() {
            trace.complete();
            trace.metrics.update_cost(&self.pricing);

            let mut history = self.trace_history.write().await;
            history.push(trace.clone());

            Some(trace)
        } else {
            None
        }
    }

    /// Get the current trace
    pub async fn current_trace(&self) -> Option<ExecutionTrace> {
        self.current_trace.read().await.clone()
    }

    /// Get the trace history
    pub async fn history(&self) -> Vec<ExecutionTrace> {
        self.trace_history.read().await.clone()
    }

    /// Get the cost tracker
    pub fn cost_tracker(&self) -> &Arc<CostTracker> {
        &self.cost_tracker
    }

    /// Get current cost summary
    pub async fn cost_summary(&self) -> String {
        self.cost_tracker.get_summary().await
    }

    /// Export traces as JSON
    pub async fn export_json(&self) -> serde_json::Value {
        let history = self.trace_history.read().await;
        let current = self.current_trace.read().await;

        serde_json::json!({
            "current": *current,
            "history": *history
        })
    }
}

impl Default for ExecutionTracer {
    fn default() -> Self {
        Self::with_default_pricing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage() {
        let mut usage1 = TokenUsage::new(100, 50);
        let usage2 = TokenUsage::new(200, 100);

        assert_eq!(usage1.total(), 150);

        usage1 += usage2;
        assert_eq!(usage1.input_tokens, 300);
        assert_eq!(usage1.output_tokens, 150);
    }

    #[test]
    fn test_model_pricing() {
        let pricing = ModelPricing::new(3.0, 15.0);
        let usage = TokenUsage::new(1_000_000, 500_000);

        let cost = pricing.calculate_cost(&usage);
        // 1M input * $3/M + 0.5M output * $15/M = $3 + $7.5 = $10.5
        assert!((cost - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_span_creation() {
        let span = SpanBuilder::new("test_call", SpanType::LlmCall)
            .with_model("claude-3-sonnet")
            .with_input(serde_json::json!({"prompt": "Hello"}))
            .build();

        assert_eq!(span.span_type, SpanType::LlmCall);
        assert_eq!(span.model, Some("claude-3-sonnet".to_string()));
    }

    #[test]
    fn test_execution_trace() {
        let mut trace = ExecutionTrace::new("test_workflow");

        let mut span1 = Span::new("llm_1", SpanType::LlmCall);
        span1.tokens = Some(TokenUsage::new(100, 50));
        span1.complete(true, None);

        let mut span2 = Span::new("tool_1", SpanType::ToolCall);
        span2.complete(true, None);

        trace.add_span(span1);
        trace.add_span(span2);
        trace.complete();

        assert_eq!(trace.metrics.llm_calls, 1);
        assert_eq!(trace.metrics.tool_calls, 1);
        assert_eq!(trace.metrics.total_tokens.input_tokens, 100);
    }

    #[tokio::test]
    async fn test_cost_tracker() {
        let tracker = CostTracker::with_default_pricing();

        tracker.record_llm_call(&TokenUsage::new(1000, 500));
        tracker.record_tool_call();

        let tokens = tracker.get_tokens();
        assert_eq!(tokens.input_tokens, 1000);
        assert_eq!(tokens.output_tokens, 500);

        let summary = tracker.get_summary().await;
        assert!(summary.contains("1500"));
    }

    #[tokio::test]
    async fn test_budget_tracking() {
        let tracker = CostTracker::new(ModelPricing::new(1.0, 1.0)); // $1/M tokens

        tracker.set_budget(0.01).await; // $0.01 budget

        // Add 10K tokens = $0.01 cost
        tracker.record_llm_call(&TokenUsage::new(5000, 5000));

        // Should be at budget
        assert!(!tracker.is_over_budget().await);

        // Add more tokens
        tracker.record_llm_call(&TokenUsage::new(1000, 0));

        // Now over budget
        assert!(tracker.is_over_budget().await);
    }

    #[tokio::test]
    async fn test_execution_tracer() {
        let tracer = ExecutionTracer::with_default_pricing();

        let trace_id = tracer.start_trace("test_workflow").await;
        assert!(!trace_id.is_empty());

        let mut span = Span::new("test_span", SpanType::LlmCall);
        span.tokens = Some(TokenUsage::new(100, 50));
        tracer.add_span(span).await;

        let trace = tracer.complete_trace().await.unwrap();
        assert_eq!(trace.spans.len(), 1);
        assert_eq!(trace.metrics.llm_calls, 1);
    }

    #[test]
    fn test_pricing_presets() {
        let opus = pricing::claude_opus();
        let sonnet = pricing::claude_sonnet();
        let haiku = pricing::claude_haiku();

        // Opus should be most expensive
        assert!(opus.input_per_million > sonnet.input_per_million);
        assert!(sonnet.input_per_million > haiku.input_per_million);
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = ExecutionMetrics {
            total_tokens: TokenUsage::new(10000, 5000),
            estimated_cost_usd: 0.075,
            total_duration: Duration::from_secs(5),
            tool_calls: 3,
            llm_calls: 2,
            retries: 1,
        };

        let summary = metrics.format_summary();
        assert!(summary.contains("15000")); // Total tokens
        assert!(summary.contains("$0.0750")); // Cost
    }
}
