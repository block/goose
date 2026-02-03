//! Cost Tracker for LLM API Usage
//!
//! Tracks token usage and calculates costs based on model pricing.
//! Supports session-level cost aggregation and reporting.

use super::errors::ObservabilityError;
use super::ReportFormat;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Model pricing (USD per 1K tokens)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per 1K input tokens
    pub input_per_1k: f64,
    /// Cost per 1K output tokens
    pub output_per_1k: f64,
    /// Cost per 1K cached input tokens
    pub cached_input_per_1k: f64,
}

impl ModelPricing {
    /// Create new pricing
    pub fn new(input_per_1k: f64, output_per_1k: f64) -> Self {
        Self {
            input_per_1k,
            output_per_1k,
            cached_input_per_1k: input_per_1k * 0.25, // Default: 25% of input cost
        }
    }

    /// Create pricing with custom cache cost
    pub fn with_cache_cost(
        input_per_1k: f64,
        output_per_1k: f64,
        cached_input_per_1k: f64,
    ) -> Self {
        Self {
            input_per_1k,
            output_per_1k,
            cached_input_per_1k,
        }
    }
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input_per_1k: 0.001,
            output_per_1k: 0.002,
            cached_input_per_1k: 0.00025,
        }
    }
}

/// Default model pricing database
static MODEL_PRICING: Lazy<HashMap<&str, ModelPricing>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // =========================================================================
    // Anthropic Models
    // =========================================================================
    m.insert(
        "claude-3-opus-20240229",
        ModelPricing::with_cache_cost(0.015, 0.075, 0.00375),
    );
    m.insert(
        "claude-3-5-sonnet-20241022",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    m.insert(
        "claude-3-5-sonnet-latest",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    m.insert(
        "claude-3-5-haiku-20241022",
        ModelPricing::with_cache_cost(0.0008, 0.004, 0.0002),
    );
    m.insert(
        "claude-3-haiku-20240307",
        ModelPricing::with_cache_cost(0.00025, 0.00125, 0.0000625),
    );
    m.insert(
        "claude-3-sonnet-20240229",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    // Claude 4 models (estimated pricing based on patterns)
    m.insert(
        "claude-sonnet-4-20250514",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    m.insert(
        "claude-opus-4-20250514",
        ModelPricing::with_cache_cost(0.015, 0.075, 0.00375),
    );

    // =========================================================================
    // OpenAI Models
    // =========================================================================
    m.insert(
        "gpt-4o",
        ModelPricing::with_cache_cost(0.0025, 0.01, 0.00125),
    );
    m.insert(
        "gpt-4o-2024-11-20",
        ModelPricing::with_cache_cost(0.0025, 0.01, 0.00125),
    );
    m.insert(
        "gpt-4o-2024-08-06",
        ModelPricing::with_cache_cost(0.0025, 0.01, 0.00125),
    );
    m.insert(
        "gpt-4o-mini",
        ModelPricing::with_cache_cost(0.00015, 0.0006, 0.000075),
    );
    m.insert(
        "gpt-4o-mini-2024-07-18",
        ModelPricing::with_cache_cost(0.00015, 0.0006, 0.000075),
    );
    m.insert(
        "gpt-4-turbo",
        ModelPricing::with_cache_cost(0.01, 0.03, 0.0025),
    );
    m.insert(
        "gpt-4-turbo-preview",
        ModelPricing::with_cache_cost(0.01, 0.03, 0.0025),
    );
    m.insert(
        "gpt-4",
        ModelPricing::with_cache_cost(0.03, 0.06, 0.0075),
    );
    m.insert(
        "gpt-4-32k",
        ModelPricing::with_cache_cost(0.06, 0.12, 0.015),
    );
    m.insert(
        "gpt-3.5-turbo",
        ModelPricing::with_cache_cost(0.0005, 0.0015, 0.000125),
    );
    m.insert(
        "gpt-3.5-turbo-16k",
        ModelPricing::with_cache_cost(0.001, 0.002, 0.00025),
    );
    // o1 models
    m.insert(
        "o1-preview",
        ModelPricing::with_cache_cost(0.015, 0.06, 0.00375),
    );
    m.insert(
        "o1-mini",
        ModelPricing::with_cache_cost(0.003, 0.012, 0.00075),
    );

    // =========================================================================
    // Google Models
    // =========================================================================
    m.insert(
        "gemini-1.5-pro",
        ModelPricing::with_cache_cost(0.00125, 0.005, 0.0003125),
    );
    m.insert(
        "gemini-1.5-flash",
        ModelPricing::with_cache_cost(0.000075, 0.0003, 0.00001875),
    );
    m.insert(
        "gemini-pro",
        ModelPricing::with_cache_cost(0.00025, 0.0005, 0.0000625),
    );

    // =========================================================================
    // AWS Bedrock Models (Claude)
    // =========================================================================
    m.insert(
        "anthropic.claude-3-opus-20240229-v1:0",
        ModelPricing::with_cache_cost(0.015, 0.075, 0.00375),
    );
    m.insert(
        "anthropic.claude-3-sonnet-20240229-v1:0",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    m.insert(
        "anthropic.claude-3-haiku-20240307-v1:0",
        ModelPricing::with_cache_cost(0.00025, 0.00125, 0.0000625),
    );

    // =========================================================================
    // Mistral Models
    // =========================================================================
    m.insert(
        "mistral-large",
        ModelPricing::with_cache_cost(0.004, 0.012, 0.001),
    );
    m.insert(
        "mistral-medium",
        ModelPricing::with_cache_cost(0.0027, 0.0081, 0.000675),
    );
    m.insert(
        "mistral-small",
        ModelPricing::with_cache_cost(0.001, 0.003, 0.00025),
    );
    m.insert(
        "mixtral-8x7b",
        ModelPricing::with_cache_cost(0.0007, 0.0007, 0.000175),
    );

    // =========================================================================
    // Cohere Models
    // =========================================================================
    m.insert(
        "command-r-plus",
        ModelPricing::with_cache_cost(0.003, 0.015, 0.00075),
    );
    m.insert(
        "command-r",
        ModelPricing::with_cache_cost(0.0005, 0.0015, 0.000125),
    );

    // =========================================================================
    // Local/Self-hosted Models (free)
    // =========================================================================
    m.insert("ollama/*", ModelPricing::new(0.0, 0.0));
    m.insert("local/*", ModelPricing::new(0.0, 0.0));

    m
});

/// Token usage for a single request
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens
    pub input_tokens: u64,
    /// Number of output tokens
    pub output_tokens: u64,
    /// Number of cached tokens (if applicable)
    pub cached_tokens: u64,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cached_tokens: 0,
        }
    }

    /// Create token usage with cache
    pub fn with_cache(input_tokens: u64, output_tokens: u64, cached_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cached_tokens,
        }
    }

    /// Total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Cost for a single request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCost {
    /// Timestamp of the request
    pub timestamp: DateTime<Utc>,
    /// Model used
    pub model: String,
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Cached tokens
    pub cached_tokens: u64,
    /// Cost in USD
    pub cost_usd: f64,
}

/// Aggregated cost for a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCost {
    /// Session identifier
    pub session_id: String,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total cached tokens
    pub total_cached_tokens: u64,
    /// Individual request costs
    pub requests: Vec<RequestCost>,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Last updated time
    pub updated_at: DateTime<Utc>,
}

impl SessionCost {
    /// Create a new session cost tracker
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.into(),
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cached_tokens: 0,
            requests: Vec::new(),
            started_at: now,
            updated_at: now,
        }
    }

    /// Total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Average cost per request
    pub fn average_cost_per_request(&self) -> f64 {
        if self.requests.is_empty() {
            0.0
        } else {
            self.total_cost_usd / self.requests.len() as f64
        }
    }

    /// Cost per 1K tokens
    pub fn cost_per_1k_tokens(&self) -> f64 {
        let total = self.total_tokens();
        if total == 0 {
            0.0
        } else {
            (self.total_cost_usd * 1000.0) / total as f64
        }
    }
}

/// Cost tracker for LLM API usage
pub struct CostTracker {
    /// Session costs
    session_costs: RwLock<HashMap<String, SessionCost>>,
    /// Custom pricing overrides
    pricing_overrides: RwLock<HashMap<String, ModelPricing>>,
}

impl CostTracker {
    /// Create a new cost tracker
    pub fn new() -> Self {
        Self {
            session_costs: RwLock::new(HashMap::new()),
            pricing_overrides: RwLock::new(HashMap::new()),
        }
    }

    /// Create cost tracker with custom pricing overrides
    pub fn with_pricing_overrides(overrides: HashMap<String, ModelPricing>) -> Self {
        Self {
            session_costs: RwLock::new(HashMap::new()),
            pricing_overrides: RwLock::new(overrides),
        }
    }

    /// Calculate cost for a single request
    pub fn calculate_cost(&self, usage: &TokenUsage, model: &str) -> f64 {
        let pricing = self.get_pricing_sync(model);

        // Calculate base input cost
        let input_cost = (usage.input_tokens as f64 / 1000.0) * pricing.input_per_1k;

        // Calculate output cost
        let output_cost = (usage.output_tokens as f64 / 1000.0) * pricing.output_per_1k;

        // Calculate cached token cost (replaces regular input cost for cached tokens)
        // If there are cached tokens, we subtract the regular input cost and add the cached cost
        let cache_adjustment = if usage.cached_tokens > 0 {
            let regular_cost_for_cached =
                (usage.cached_tokens as f64 / 1000.0) * pricing.input_per_1k;
            let cached_cost = (usage.cached_tokens as f64 / 1000.0) * pricing.cached_input_per_1k;
            cached_cost - regular_cost_for_cached
        } else {
            0.0
        };

        input_cost + output_cost + cache_adjustment
    }

    /// Get pricing for a model (sync version for internal use)
    fn get_pricing_sync(&self, model: &str) -> ModelPricing {
        // Try overrides first (blocking read for sync context)
        if let Ok(overrides) = self.pricing_overrides.try_read() {
            if let Some(pricing) = overrides.get(model) {
                return *pricing;
            }
        }

        // Try exact match in static pricing
        if let Some(pricing) = MODEL_PRICING.get(model) {
            return *pricing;
        }

        // Try prefix match for wildcards (e.g., "ollama/*")
        for (pattern, pricing) in MODEL_PRICING.iter() {
            if let Some(prefix) = pattern.strip_suffix("/*") {
                if model.starts_with(prefix) {
                    return *pricing;
                }
            }
        }

        // Default conservative pricing
        ModelPricing::default()
    }

    /// Get pricing for a model
    pub async fn get_pricing(&self, model: &str) -> ModelPricing {
        // Try overrides first
        {
            let overrides = self.pricing_overrides.read().await;
            if let Some(pricing) = overrides.get(model) {
                return *pricing;
            }
        }

        // Try exact match in static pricing
        if let Some(pricing) = MODEL_PRICING.get(model) {
            return *pricing;
        }

        // Try prefix match for wildcards
        for (pattern, pricing) in MODEL_PRICING.iter() {
            if let Some(prefix) = pattern.strip_suffix("/*") {
                if model.starts_with(prefix) {
                    return *pricing;
                }
            }
        }

        // Default conservative pricing
        ModelPricing::default()
    }

    /// Set custom pricing for a model
    pub async fn set_pricing(&self, model: impl Into<String>, pricing: ModelPricing) {
        let mut overrides = self.pricing_overrides.write().await;
        overrides.insert(model.into(), pricing);
    }

    /// Record usage and cost for a session
    pub async fn record(&self, session_id: &str, usage: &TokenUsage, model: &str) {
        let cost = self.calculate_cost(usage, model);
        let now = Utc::now();

        let request_cost = RequestCost {
            timestamp: now,
            model: model.to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cached_tokens: usage.cached_tokens,
            cost_usd: cost,
        };

        let mut costs = self.session_costs.write().await;
        let session_cost = costs.entry(session_id.to_string()).or_insert_with(|| {
            SessionCost::new(session_id)
        });

        session_cost.total_cost_usd += cost;
        session_cost.total_input_tokens += usage.input_tokens;
        session_cost.total_output_tokens += usage.output_tokens;
        session_cost.total_cached_tokens += usage.cached_tokens;
        session_cost.updated_at = now;
        session_cost.requests.push(request_cost);
    }

    /// Get session cost summary
    pub async fn get_session_cost(&self, session_id: &str) -> Option<SessionCost> {
        let costs = self.session_costs.read().await;
        costs.get(session_id).cloned()
    }

    /// Get all session costs
    pub async fn get_all_session_costs(&self) -> HashMap<String, SessionCost> {
        let costs = self.session_costs.read().await;
        costs.clone()
    }

    /// Get total cost across all sessions
    pub async fn get_total_cost(&self) -> f64 {
        let costs = self.session_costs.read().await;
        costs.values().map(|s| s.total_cost_usd).sum()
    }

    /// Get total number of requests across all sessions
    pub async fn get_total_requests(&self) -> u64 {
        let costs = self.session_costs.read().await;
        costs.values().map(|s| s.requests.len() as u64).sum()
    }

    /// Get total tokens (input, output, cached) across all sessions
    pub async fn get_total_tokens(&self) -> (u64, u64, u64) {
        let costs = self.session_costs.read().await;
        let input: u64 = costs.values().map(|s| s.total_input_tokens).sum();
        let output: u64 = costs.values().map(|s| s.total_output_tokens).sum();
        let cached: u64 = costs.values().map(|s| s.total_cached_tokens).sum();
        (input, output, cached)
    }

    /// Clear all session costs
    pub async fn clear(&self) {
        let mut costs = self.session_costs.write().await;
        costs.clear();
    }

    /// Clear a specific session
    pub async fn clear_session(&self, session_id: &str) {
        let mut costs = self.session_costs.write().await;
        costs.remove(session_id);
    }

    /// Export cost report
    pub async fn export_report(&self, format: ReportFormat) -> Result<String, ObservabilityError> {
        let costs = self.session_costs.read().await;

        match format {
            ReportFormat::Json => {
                serde_json::to_string_pretty(&*costs).map_err(ObservabilityError::SerializationError)
            }
            ReportFormat::Csv => self.to_csv(&costs),
            ReportFormat::Markdown => self.to_markdown(&costs),
        }
    }

    /// Convert to CSV format
    fn to_csv(&self, costs: &HashMap<String, SessionCost>) -> Result<String, ObservabilityError> {
        let mut csv = String::new();
        csv.push_str("session_id,timestamp,model,input_tokens,output_tokens,cached_tokens,cost_usd\n");

        for session in costs.values() {
            for request in &session.requests {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{:.6}\n",
                    session.session_id,
                    request.timestamp.to_rfc3339(),
                    request.model,
                    request.input_tokens,
                    request.output_tokens,
                    request.cached_tokens,
                    request.cost_usd
                ));
            }
        }

        Ok(csv)
    }

    /// Convert to Markdown format
    fn to_markdown(
        &self,
        costs: &HashMap<String, SessionCost>,
    ) -> Result<String, ObservabilityError> {
        let mut md = String::new();

        md.push_str("# Cost Report\n\n");
        md.push_str(&format!(
            "Generated: {}\n\n",
            Utc::now().to_rfc3339()
        ));

        // Summary
        let total_cost: f64 = costs.values().map(|s| s.total_cost_usd).sum();
        let total_requests: usize = costs.values().map(|s| s.requests.len()).sum();
        let total_input: u64 = costs.values().map(|s| s.total_input_tokens).sum();
        let total_output: u64 = costs.values().map(|s| s.total_output_tokens).sum();

        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total Cost:** ${:.4}\n", total_cost));
        md.push_str(&format!("- **Total Requests:** {}\n", total_requests));
        md.push_str(&format!("- **Total Input Tokens:** {}\n", total_input));
        md.push_str(&format!("- **Total Output Tokens:** {}\n", total_output));
        md.push('\n');

        // Per-session breakdown
        md.push_str("## Sessions\n\n");

        for session in costs.values() {
            md.push_str(&format!("### Session: {}\n\n", session.session_id));
            md.push_str(&format!("- **Cost:** ${:.4}\n", session.total_cost_usd));
            md.push_str(&format!("- **Requests:** {}\n", session.requests.len()));
            md.push_str(&format!("- **Input Tokens:** {}\n", session.total_input_tokens));
            md.push_str(&format!("- **Output Tokens:** {}\n", session.total_output_tokens));
            md.push_str(&format!(
                "- **Avg Cost/Request:** ${:.6}\n",
                session.average_cost_per_request()
            ));
            md.push('\n');

            // Request details table
            if !session.requests.is_empty() {
                md.push_str("| Timestamp | Model | Input | Output | Cost |\n");
                md.push_str("|-----------|-------|-------|--------|------|\n");

                for request in &session.requests {
                    md.push_str(&format!(
                        "| {} | {} | {} | {} | ${:.6} |\n",
                        request.timestamp.format("%H:%M:%S"),
                        request.model,
                        request.input_tokens,
                        request.output_tokens,
                        request.cost_usd
                    ));
                }
                md.push('\n');
            }
        }

        Ok(md)
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_creation() {
        let pricing = ModelPricing::new(0.01, 0.03);
        assert_eq!(pricing.input_per_1k, 0.01);
        assert_eq!(pricing.output_per_1k, 0.03);
        assert_eq!(pricing.cached_input_per_1k, 0.0025); // 25% of input
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 200);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
        assert_eq!(usage.total_tokens(), 300);
        assert_eq!(usage.cached_tokens, 0);

        let usage_with_cache = TokenUsage::with_cache(100, 200, 50);
        assert_eq!(usage_with_cache.cached_tokens, 50);
    }

    #[test]
    fn test_session_cost() {
        let session = SessionCost::new("test-session");
        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.total_cost_usd, 0.0);
        assert_eq!(session.total_tokens(), 0);
    }

    #[test]
    fn test_cost_calculation() {
        let tracker = CostTracker::new();

        // Claude 3.5 Sonnet pricing: $0.003/1K input, $0.015/1K output
        let usage = TokenUsage::new(1000, 1000);
        let cost = tracker.calculate_cost(&usage, "claude-3-5-sonnet-20241022");

        // 1K input * 0.003 + 1K output * 0.015 = 0.018
        assert!((cost - 0.018).abs() < 0.0001);
    }

    #[test]
    fn test_cost_calculation_with_cache() {
        let tracker = CostTracker::new();

        // 1000 input tokens, 500 of which are cached
        let usage = TokenUsage::with_cache(1000, 1000, 500);
        let cost = tracker.calculate_cost(&usage, "claude-3-5-sonnet-20241022");

        // Base: 1K input * 0.003 + 1K output * 0.015 = 0.018
        // Cache adjustment: (500/1000) * (0.00075 - 0.003) = -0.001125
        // Total: 0.018 - 0.001125 = 0.016875
        assert!((cost - 0.016875).abs() < 0.0001);
    }

    #[test]
    fn test_default_pricing_for_unknown_model() {
        let tracker = CostTracker::new();
        let usage = TokenUsage::new(1000, 1000);
        let cost = tracker.calculate_cost(&usage, "unknown-model-xyz");

        // Should use default pricing
        assert!(cost > 0.0);
    }

    #[tokio::test]
    async fn test_record_and_get_session_cost() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(1000, 500);
        tracker
            .record("session-1", &usage, "gpt-4o-mini")
            .await;

        let session = tracker.get_session_cost("session-1").await.unwrap();
        assert_eq!(session.session_id, "session-1");
        assert_eq!(session.total_input_tokens, 1000);
        assert_eq!(session.total_output_tokens, 500);
        assert_eq!(session.requests.len(), 1);
        assert!(session.total_cost_usd > 0.0);
    }

    #[tokio::test]
    async fn test_multiple_requests_same_session() {
        let tracker = CostTracker::new();

        let usage1 = TokenUsage::new(100, 50);
        let usage2 = TokenUsage::new(200, 100);

        tracker.record("session-1", &usage1, "gpt-4o-mini").await;
        tracker.record("session-1", &usage2, "gpt-4o-mini").await;

        let session = tracker.get_session_cost("session-1").await.unwrap();
        assert_eq!(session.total_input_tokens, 300);
        assert_eq!(session.total_output_tokens, 150);
        assert_eq!(session.requests.len(), 2);
    }

    #[tokio::test]
    async fn test_total_cost_across_sessions() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(1000, 500);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;
        tracker.record("session-2", &usage, "gpt-4o-mini").await;

        let total = tracker.get_total_cost().await;
        let session1 = tracker.get_session_cost("session-1").await.unwrap();

        // Total should be 2x the cost of one session
        assert!((total - session1.total_cost_usd * 2.0).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_custom_pricing_override() {
        let tracker = CostTracker::new();

        // Set custom pricing
        tracker
            .set_pricing("custom-model", ModelPricing::new(0.1, 0.2))
            .await;

        let usage = TokenUsage::new(1000, 1000);
        tracker.record("session-1", &usage, "custom-model").await;

        let session = tracker.get_session_cost("session-1").await.unwrap();

        // 1K * 0.1 + 1K * 0.2 = 0.3
        assert!((session.total_cost_usd - 0.3).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_export_json() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(100, 50);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;

        let json = tracker.export_report(ReportFormat::Json).await.unwrap();
        assert!(json.contains("session-1"));
        assert!(json.contains("total_cost_usd"));
    }

    #[tokio::test]
    async fn test_export_csv() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(100, 50);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;

        let csv = tracker.export_report(ReportFormat::Csv).await.unwrap();
        assert!(csv.contains("session_id,timestamp,model"));
        assert!(csv.contains("session-1"));
        assert!(csv.contains("gpt-4o-mini"));
    }

    #[tokio::test]
    async fn test_export_markdown() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(100, 50);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;

        let md = tracker.export_report(ReportFormat::Markdown).await.unwrap();
        assert!(md.contains("# Cost Report"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("session-1"));
    }

    #[tokio::test]
    async fn test_clear_session() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(100, 50);
        tracker.record("session-1", &usage, "gpt-4o-mini").await;
        tracker.record("session-2", &usage, "gpt-4o-mini").await;

        tracker.clear_session("session-1").await;

        assert!(tracker.get_session_cost("session-1").await.is_none());
        assert!(tracker.get_session_cost("session-2").await.is_some());
    }

    #[tokio::test]
    async fn test_get_total_tokens() {
        let tracker = CostTracker::new();

        tracker
            .record("s1", &TokenUsage::with_cache(100, 50, 10), "gpt-4o-mini")
            .await;
        tracker
            .record("s2", &TokenUsage::with_cache(200, 100, 20), "gpt-4o-mini")
            .await;

        let (input, output, cached) = tracker.get_total_tokens().await;
        assert_eq!(input, 300);
        assert_eq!(output, 150);
        assert_eq!(cached, 30);
    }

    #[test]
    fn test_ollama_free_pricing() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(10000, 5000);
        let cost = tracker.calculate_cost(&usage, "ollama/llama3");

        // Ollama models should be free
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_local_free_pricing() {
        let tracker = CostTracker::new();

        let usage = TokenUsage::new(10000, 5000);
        let cost = tracker.calculate_cost(&usage, "local/my-model");

        // Local models should be free
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_session_cost_metrics() {
        let mut session = SessionCost::new("test");
        session.total_cost_usd = 1.0;
        session.total_input_tokens = 5000;
        session.total_output_tokens = 5000;
        session.requests = vec![
            RequestCost {
                timestamp: Utc::now(),
                model: "test".to_string(),
                input_tokens: 2500,
                output_tokens: 2500,
                cached_tokens: 0,
                cost_usd: 0.5,
            },
            RequestCost {
                timestamp: Utc::now(),
                model: "test".to_string(),
                input_tokens: 2500,
                output_tokens: 2500,
                cached_tokens: 0,
                cost_usd: 0.5,
            },
        ];

        assert_eq!(session.average_cost_per_request(), 0.5);
        assert!((session.cost_per_1k_tokens() - 0.1).abs() < 0.0001);
    }
}
