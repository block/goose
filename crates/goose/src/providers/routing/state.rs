//! Provider state tracking for runs

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::{EndpointId, ProjectId, ProviderCapabilities, RunId};

/// Reason for a provider switch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitchReason {
    /// User manually initiated the switch
    UserInitiated,
    /// Quota exhausted on current provider
    QuotaExhausted,
    /// Rate limited by current provider
    RateLimited,
    /// Current provider endpoint unreachable
    EndpointUnreachable,
    /// Request timeout on current provider
    Timeout,
    /// Server error from current provider
    ServerError,
    /// Authentication failed
    AuthenticationFailed,
    /// Model not available on current provider
    ModelNotAvailable,
    /// Capability mismatch detected
    CapabilityMismatch,
    /// Cost budget exceeded
    BudgetExceeded,
    /// Automatic fallback triggered
    AutoFallback,
    /// Project policy changed
    PolicyChange,
}

impl SwitchReason {
    /// Whether this switch reason indicates a provider failure
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::QuotaExhausted
                | Self::RateLimited
                | Self::EndpointUnreachable
                | Self::Timeout
                | Self::ServerError
                | Self::AuthenticationFailed
                | Self::ModelNotAvailable
        )
    }

    /// Whether this switch was planned/intentional
    pub fn is_intentional(&self) -> bool {
        matches!(
            self,
            Self::UserInitiated | Self::PolicyChange | Self::BudgetExceeded
        )
    }
}

/// Information about a provider switch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSwitch {
    /// When the switch occurred
    pub timestamp: SystemTime,
    /// Previous provider configuration
    pub from: ProviderConfig,
    /// New provider configuration
    pub to: ProviderConfig,
    /// Reason for the switch
    pub reason: SwitchReason,
    /// Whether user initiated this switch
    pub user_initiated: bool,
    /// Hash of handoff memo generated for this switch
    pub handoff_digest: Option<String>,
    /// Any additional context about the switch
    pub context: Option<String>,
}

/// Configuration of a provider at a point in time
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (anthropic, openai, etc.)
    pub provider: String,
    /// Endpoint identifier
    pub endpoint_id: EndpointId,
    /// Model name
    pub model: String,
    /// Capabilities detected/configured
    pub capabilities: Option<ProviderCapabilities>,
}

impl ProviderConfig {
    pub fn new(
        provider: impl Into<String>,
        endpoint_id: impl Into<EndpointId>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            endpoint_id: endpoint_id.into(),
            model: model.into(),
            capabilities: None,
        }
    }

    pub fn with_capabilities(mut self, capabilities: ProviderCapabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }
}

/// State of provider usage for a specific run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunProviderState {
    /// Schema version for compatibility
    pub schema_version: u32,
    /// Run identifier
    pub run_id: RunId,
    /// Project identifier
    pub project_id: ProjectId,
    /// When the run started
    pub started_at: SystemTime,
    /// Currently active provider configuration
    pub active: ProviderConfig,
    /// History of provider switches during this run
    pub switch_history: Vec<ProviderSwitch>,
    /// Total requests made in this run
    pub total_requests: u64,
    /// Total tokens consumed in this run
    pub total_tokens_used: u64,
    /// Total cost incurred (if tracked)
    pub total_cost: Option<f64>,
    /// Any errors encountered
    pub errors: Vec<ProviderError>,
}

/// Error information for provider operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    /// When the error occurred
    pub timestamp: SystemTime,
    /// Provider that caused the error
    pub provider: String,
    /// Model being used
    pub model: String,
    /// Error category
    pub category: super::ErrorCategory,
    /// Error message
    pub message: String,
    /// Whether a retry was attempted
    pub retry_attempted: bool,
    /// Whether the operation eventually succeeded
    pub resolved: bool,
}

impl RunProviderState {
    /// Create new run state
    pub fn new(run_id: RunId, project_id: ProjectId, initial_config: ProviderConfig) -> Self {
        Self {
            schema_version: 1,
            run_id,
            project_id,
            started_at: SystemTime::now(),
            active: initial_config,
            switch_history: Vec::new(),
            total_requests: 0,
            total_tokens_used: 0,
            total_cost: None,
            errors: Vec::new(),
        }
    }

    /// Record a provider switch
    pub fn record_switch(
        &mut self,
        to_config: ProviderConfig,
        reason: SwitchReason,
        user_initiated: bool,
        handoff_digest: Option<String>,
    ) {
        let switch = ProviderSwitch {
            timestamp: SystemTime::now(),
            from: self.active.clone(),
            to: to_config.clone(),
            reason,
            user_initiated,
            handoff_digest,
            context: None,
        };

        self.switch_history.push(switch);
        self.active = to_config;
    }

    /// Record a provider error
    pub fn record_error(
        &mut self,
        category: super::ErrorCategory,
        message: String,
        retry_attempted: bool,
    ) {
        let error = ProviderError {
            timestamp: SystemTime::now(),
            provider: self.active.provider.clone(),
            model: self.active.model.clone(),
            category,
            message,
            retry_attempted,
            resolved: false,
        };

        self.errors.push(error);
    }

    /// Mark the most recent error as resolved
    pub fn resolve_last_error(&mut self) {
        if let Some(error) = self.errors.last_mut() {
            error.resolved = true;
        }
    }

    /// Update usage statistics
    pub fn update_usage(&mut self, requests: u64, tokens: u64, cost: Option<f64>) {
        self.total_requests += requests;
        self.total_tokens_used += tokens;
        if let Some(additional_cost) = cost {
            self.total_cost = Some(self.total_cost.unwrap_or(0.0) + additional_cost);
        }
    }

    /// Get the number of switches for a specific reason
    pub fn switch_count_by_reason(&self, reason: &SwitchReason) -> usize {
        self.switch_history
            .iter()
            .filter(|s| &s.reason == reason)
            .count()
    }

    /// Get the current provider name
    pub fn current_provider(&self) -> &str {
        &self.active.provider
    }

    /// Get the current model name
    pub fn current_model(&self) -> &str {
        &self.active.model
    }

    /// Get the number of unresolved errors
    pub fn unresolved_error_count(&self) -> usize {
        self.errors.iter().filter(|e| !e.resolved).count()
    }

    /// Get errors for a specific provider
    pub fn errors_for_provider(&self, provider: &str) -> Vec<&ProviderError> {
        self.errors
            .iter()
            .filter(|e| e.provider == provider)
            .collect()
    }

    /// Check if a provider has had recent failures
    pub fn has_recent_failures(&self, provider: &str, within_seconds: u64) -> bool {
        let cutoff = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - within_seconds;

        self.errors.iter().any(|e| {
            e.provider == provider
                && !e.resolved
                && e.timestamp
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    > cutoff
        })
    }

    /// Get a summary of the run
    pub fn get_summary(&self) -> RunSummary {
        let unique_providers: std::collections::HashSet<_> = std::iter::once(&self.active.provider)
            .chain(self.switch_history.iter().map(|s| &s.from.provider))
            .collect();

        let failure_switches = self
            .switch_history
            .iter()
            .filter(|s| s.reason.is_failure())
            .count();

        RunSummary {
            run_id: self.run_id,
            total_requests: self.total_requests,
            total_tokens_used: self.total_tokens_used,
            total_cost: self.total_cost,
            providers_used: unique_providers.len(),
            total_switches: self.switch_history.len(),
            failure_switches,
            unresolved_errors: self.unresolved_error_count(),
        }
    }
}

/// Summary statistics for a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: RunId,
    pub total_requests: u64,
    pub total_tokens_used: u64,
    pub total_cost: Option<f64>,
    pub providers_used: usize,
    pub total_switches: usize,
    pub failure_switches: usize,
    pub unresolved_errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_switch_creation() {
        let mut state = RunProviderState::new(
            RunId::new(),
            ProjectId::new(),
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5"),
        );

        assert_eq!(state.current_provider(), "anthropic");
        assert_eq!(state.switch_history.len(), 0);

        // Record a switch
        let new_config = ProviderConfig::new("openai", "openai_primary", "gpt-4o");
        state.record_switch(
            new_config,
            SwitchReason::QuotaExhausted,
            false,
            Some("abc123".to_string()),
        );

        assert_eq!(state.current_provider(), "openai");
        assert_eq!(state.switch_history.len(), 1);
        assert_eq!(state.switch_history[0].reason, SwitchReason::QuotaExhausted);
    }

    #[test]
    fn test_error_tracking() {
        let mut state = RunProviderState::new(
            RunId::new(),
            ProjectId::new(),
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5"),
        );

        state.record_error(
            super::super::ErrorCategory::QuotaExhausted,
            "API quota exceeded".to_string(),
            true,
        );

        assert_eq!(state.unresolved_error_count(), 1);

        state.resolve_last_error();
        assert_eq!(state.unresolved_error_count(), 0);
    }

    #[test]
    fn test_usage_tracking() {
        let mut state = RunProviderState::new(
            RunId::new(),
            ProjectId::new(),
            ProviderConfig::new("anthropic", "anthropic_primary", "claude-sonnet-3.5"),
        );

        state.update_usage(5, 1000, Some(0.50));
        state.update_usage(3, 800, Some(0.40));

        assert_eq!(state.total_requests, 8);
        assert_eq!(state.total_tokens_used, 1800);
        assert_eq!(state.total_cost, Some(0.90));
    }

    #[test]
    fn test_switch_reason_classification() {
        assert!(SwitchReason::QuotaExhausted.is_failure());
        assert!(!SwitchReason::QuotaExhausted.is_intentional());

        assert!(!SwitchReason::UserInitiated.is_failure());
        assert!(SwitchReason::UserInitiated.is_intentional());

        assert!(!SwitchReason::AutoFallback.is_failure());
        assert!(!SwitchReason::AutoFallback.is_intentional());
    }
}
