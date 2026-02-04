//! Main provider router implementation

use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    ErrorCategory, HandoffMemo, ProjectId, ProjectProviderPolicy, ProviderConfig, ProviderRegistry,
    RoutingError, RoutingResult, RunId, RunProviderState, SwitchReason,
};

/// Configuration for the provider router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Health check interval in seconds
    pub health_check_interval_secs: u64,
    /// Maximum switch attempts before giving up
    pub max_switch_attempts: u32,
    /// Delay between switch attempts in seconds
    pub switch_retry_delay_secs: u64,
    /// Whether to enable automatic health checks
    pub auto_health_checks: bool,
    /// Whether to log all provider operations
    pub verbose_logging: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            health_check_interval_secs: 300, // 5 minutes
            max_switch_attempts: 3,
            switch_retry_delay_secs: 1,
            auto_health_checks: true,
            verbose_logging: false,
        }
    }
}

/// Main provider router that handles switching and fallback logic
pub struct ProviderRouter {
    /// Provider registry
    registry: Arc<RwLock<ProviderRegistry>>,
    /// Router configuration
    #[allow(dead_code)]
    config: RouterConfig,
    /// Active run states
    run_states: Arc<RwLock<std::collections::HashMap<RunId, RunProviderState>>>,
    /// Project policies
    project_policies: Arc<RwLock<std::collections::HashMap<ProjectId, ProjectProviderPolicy>>>,
}

impl ProviderRouter {
    /// Create a new provider router
    pub fn new(config: RouterConfig) -> Self {
        Self {
            registry: Arc::new(RwLock::new(ProviderRegistry::new())),
            config,
            run_states: Arc::new(RwLock::new(std::collections::HashMap::new())),
            project_policies: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get the provider registry
    pub fn registry(&self) -> Arc<RwLock<ProviderRegistry>> {
        Arc::clone(&self.registry)
    }

    /// Register a project policy
    pub async fn register_project_policy(
        &self,
        project_id: ProjectId,
        policy: ProjectProviderPolicy,
    ) -> RoutingResult<()> {
        let mut policies = self.project_policies.write().await;
        policies.insert(project_id, policy);
        Ok(())
    }

    /// Get project policy
    pub async fn get_project_policy(
        &self,
        project_id: ProjectId,
    ) -> RoutingResult<ProjectProviderPolicy> {
        let policies = self.project_policies.read().await;
        Ok(policies
            .get(&project_id)
            .cloned()
            .unwrap_or_else(ProjectProviderPolicy::default))
    }

    /// Start a new run with provider selection
    pub async fn start_run(
        &self,
        project_id: ProjectId,
        preferred_provider: Option<String>,
        preferred_model: Option<String>,
    ) -> RoutingResult<(RunId, ProviderConfig)> {
        let run_id = RunId::new();
        let policy = self.get_project_policy(project_id).await?;

        // Determine effective provider and model
        let provider = preferred_provider
            .as_deref()
            .unwrap_or_else(|| policy.get_effective_provider());
        let model = preferred_model
            .as_deref()
            .unwrap_or_else(|| policy.get_effective_model());

        // Validate provider is allowed
        if !policy.is_provider_allowed(provider) {
            return Err(RoutingError::policy_violation(format!(
                "Provider '{}' not allowed by project policy",
                provider
            )));
        }

        // Find best endpoint for provider
        let registry = self.registry.read().await;
        let endpoint_config = registry.get_best_endpoint(provider)?;
        let endpoint_id = endpoint_config.endpoint_id.clone();

        // Create initial provider config
        let provider_config = ProviderConfig::new(provider, endpoint_id, model);

        // Create and store run state
        let run_state = RunProviderState::new(run_id, project_id, provider_config.clone());
        let mut states = self.run_states.write().await;
        states.insert(run_id, run_state);

        Ok((run_id, provider_config))
    }

    /// Switch provider for a run
    pub async fn switch_provider(
        &self,
        run_id: RunId,
        target_provider: String,
        target_model: Option<String>,
        reason: SwitchReason,
        force: bool,
    ) -> RoutingResult<ProviderConfig> {
        let mut states = self.run_states.write().await;
        let run_state = states
            .get_mut(&run_id)
            .ok_or_else(|| RoutingError::switch_failed("Run not found".to_string()))?;

        // Get project policy
        let policy = {
            let policies = self.project_policies.read().await;
            policies
                .get(&run_state.project_id)
                .cloned()
                .unwrap_or_default()
        };

        // Determine target model
        let target_model = target_model.unwrap_or_else(|| {
            // Try to map current model to target provider
            let current_model = &run_state.active.model;
            policy.map_model(
                current_model,
                &super::ModelMappingStrategy::Balanced, // Use balanced as default
            )
        });

        // Validate switch
        policy.validate_switch(&target_provider, &target_model, force)?;

        // Find best endpoint for target provider
        let registry = self.registry.read().await;
        let endpoint_config = registry.get_best_endpoint(&target_provider)?;
        let endpoint_id = endpoint_config.endpoint_id.clone();

        // Create new provider config
        let new_config = ProviderConfig::new(target_provider, endpoint_id, target_model);

        // Generate handoff memo
        let handoff_memo = HandoffMemo::generate_for_switch(
            &run_state.active,
            &new_config,
            &reason,
            run_state.total_requests,
        )?;
        let handoff_digest = handoff_memo.compute_digest();

        // Record the switch
        let is_user_initiated = matches!(reason, SwitchReason::UserInitiated);
        run_state.record_switch(
            new_config.clone(),
            reason,
            is_user_initiated,
            Some(handoff_digest),
        );

        Ok(new_config)
    }

    /// Handle an error and potentially trigger fallback
    pub async fn handle_error(
        &self,
        run_id: RunId,
        error: &anyhow::Error,
    ) -> RoutingResult<Option<ProviderConfig>> {
        let mut states = self.run_states.write().await;
        let run_state = states
            .get_mut(&run_id)
            .ok_or_else(|| RoutingError::switch_failed("Run not found".to_string()))?;

        // Classify the error
        let registry = self.registry.read().await;
        let error_category = registry.classify_error(error);

        // Record the error
        run_state.record_error(error_category, error.to_string(), false);

        // Check if we should attempt fallback
        if !error_category.should_fallback() {
            return Ok(None); // Not a fallback-worthy error
        }

        // Get project policy
        let policy = {
            let policies = self.project_policies.read().await;
            policies
                .get(&run_state.project_id)
                .cloned()
                .unwrap_or_default()
        };

        // Check if auto-fallback is enabled
        if !policy.auto_fallback {
            return Ok(None);
        }

        // Get next fallback provider
        let current_provider = &run_state.active.provider;
        let fallback_config = match policy.get_next_fallback(current_provider) {
            Some(config) => config,
            None => return Err(RoutingError::NoFallbackAvailable),
        };

        // Check if target provider has healthy endpoints
        if !registry.has_healthy_provider(&fallback_config.provider) {
            return Err(RoutingError::switch_failed(format!(
                "Fallback provider '{}' has no healthy endpoints",
                fallback_config.provider
            )));
        }

        // Attempt the switch
        drop(states); // Release lock before recursive call
        drop(registry); // Release lock before recursive call

        let reason = match error_category {
            ErrorCategory::QuotaExhausted => SwitchReason::QuotaExhausted,
            ErrorCategory::RateLimited => SwitchReason::RateLimited,
            ErrorCategory::EndpointUnreachable => SwitchReason::EndpointUnreachable,
            ErrorCategory::Timeout => SwitchReason::Timeout,
            ErrorCategory::ServerError => SwitchReason::ServerError,
            _ => SwitchReason::AutoFallback,
        };

        // Map the model
        let current_model = {
            let states = self.run_states.read().await;
            states.get(&run_id).unwrap().active.model.clone()
        };
        let target_model = policy.map_model(&current_model, &fallback_config.model_map);

        match self
            .switch_provider(
                run_id,
                fallback_config.provider.clone(),
                Some(target_model),
                reason,
                false, // Don't force, respect policy
            )
            .await
        {
            Ok(config) => {
                // Mark error as resolved
                let mut states = self.run_states.write().await;
                if let Some(run_state) = states.get_mut(&run_id) {
                    run_state.resolve_last_error();
                }
                Ok(Some(config))
            }
            Err(_) => {
                // Fallback failed, try next in chain if available
                if let Some(next_fallback) = policy.get_next_fallback(&fallback_config.provider) {
                    let next_model = policy.map_model(&current_model, &next_fallback.model_map);
                    match self
                        .switch_provider(
                            run_id,
                            next_fallback.provider.clone(),
                            Some(next_model),
                            SwitchReason::AutoFallback,
                            false,
                        )
                        .await
                    {
                        Ok(config) => Ok(Some(config)),
                        Err(_) => Err(RoutingError::AllProvidersFailed),
                    }
                } else {
                    Err(RoutingError::AllProvidersFailed)
                }
            }
        }
    }

    /// Update usage statistics for a run
    pub async fn update_usage(
        &self,
        run_id: RunId,
        requests: u64,
        tokens: u64,
        cost: Option<f64>,
    ) -> RoutingResult<()> {
        let mut states = self.run_states.write().await;
        if let Some(run_state) = states.get_mut(&run_id) {
            run_state.update_usage(requests, tokens, cost);
        }
        Ok(())
    }

    /// Get run state
    pub async fn get_run_state(&self, run_id: RunId) -> RoutingResult<RunProviderState> {
        let states = self.run_states.read().await;
        states
            .get(&run_id)
            .cloned()
            .ok_or_else(|| RoutingError::switch_failed("Run not found".to_string()))
    }

    /// End a run and clean up state
    pub async fn end_run(&self, run_id: RunId) -> RoutingResult<RunProviderState> {
        let mut states = self.run_states.write().await;
        states
            .remove(&run_id)
            .ok_or_else(|| RoutingError::switch_failed("Run not found".to_string()))
    }

    /// Perform health checks on all endpoints
    pub async fn health_check_all(&self) -> RoutingResult<()> {
        let mut registry = self.registry.write().await;
        let endpoints: Vec<_> = registry
            .list_endpoints()
            .iter()
            .map(|e| e.endpoint_id.clone())
            .collect();

        for endpoint_id in endpoints {
            // In a real implementation, this would make HTTP requests to check health
            // For now, we'll simulate health checks
            let health_result = super::registry::HealthCheckResult::healthy(100);
            registry.update_health(&endpoint_id, health_result);
        }

        Ok(())
    }

    /// Get statistics for all runs
    pub async fn get_statistics(&self) -> RoutingResult<RouterStatistics> {
        let states = self.run_states.read().await;
        let mut stats = RouterStatistics::default();

        for run_state in states.values() {
            let summary = run_state.get_summary();
            stats.total_runs += 1;
            stats.total_requests += summary.total_requests;
            stats.total_tokens_used += summary.total_tokens_used;
            stats.total_switches += summary.total_switches;
            stats.failure_switches += summary.failure_switches;

            if let Some(cost) = summary.total_cost {
                stats.total_cost = Some(stats.total_cost.unwrap_or(0.0) + cost);
            }
        }

        Ok(stats)
    }

    /// Check if a project can be loaded without its preferred provider
    pub async fn validate_project_loadable(
        &self,
        project_id: ProjectId,
        preferred_provider: Option<&str>,
    ) -> RoutingResult<ValidationResult> {
        let policy = self.get_project_policy(project_id).await?;
        let registry = self.registry.read().await;

        let target_provider = preferred_provider.unwrap_or_else(|| policy.get_effective_provider());

        // Check if provider is pinned and missing
        if policy.pinned {
            if let Some(pinned_provider) = &policy.pinned_provider {
                if !registry.has_healthy_provider(pinned_provider) {
                    return Ok(ValidationResult::RequiresUnpin {
                        pinned_provider: pinned_provider.clone(),
                        reason: "Pinned provider has no healthy endpoints".to_string(),
                    });
                }
            }
        }

        // Check if preferred provider is available
        if !registry.has_healthy_provider(target_provider) {
            let alternatives: Vec<_> = policy
                .fallback_chain
                .iter()
                .filter(|f| registry.has_healthy_provider(&f.provider))
                .map(|f| f.provider.clone())
                .collect();

            if alternatives.is_empty() {
                return Ok(ValidationResult::NotLoadable {
                    reason: "No healthy providers available".to_string(),
                });
            }

            return Ok(ValidationResult::RequiresProviderSelection {
                preferred_unavailable: target_provider.to_string(),
                alternatives,
            });
        }

        Ok(ValidationResult::LoadableAsIs)
    }
}

/// Result of project loadability validation
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Project can be loaded as-is
    LoadableAsIs,
    /// Project requires provider selection due to unavailable preferred provider
    RequiresProviderSelection {
        preferred_unavailable: String,
        alternatives: Vec<String>,
    },
    /// Project is pinned and requires unpinning
    RequiresUnpin {
        pinned_provider: String,
        reason: String,
    },
    /// Project cannot be loaded
    NotLoadable { reason: String },
}

/// Router statistics
#[derive(Debug, Default, Clone)]
pub struct RouterStatistics {
    pub total_runs: usize,
    pub total_requests: u64,
    pub total_tokens_used: u64,
    pub total_cost: Option<f64>,
    pub total_switches: usize,
    pub failure_switches: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::routing::{registry::AuthConfig, EndpointConfig};

    #[tokio::test]
    async fn test_router_creation() {
        let config = RouterConfig::default();
        let router = ProviderRouter::new(config);

        // Register a test endpoint
        let registry_ref = router.registry();
        let mut registry = registry_ref.write().await;
        let endpoint = EndpointConfig {
            endpoint_id: "test".into(),
            provider: "anthropic".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            auth: AuthConfig::None,
            default_headers: std::collections::HashMap::new(),
            tls: super::super::registry::TlsConfig::default(),
            timeout_seconds: 30,
            max_retries: 3,
            available_models: None,
        };
        registry.register_endpoint(endpoint).unwrap();
    }

    #[tokio::test]
    async fn test_run_lifecycle() {
        let router = ProviderRouter::new(RouterConfig::default());

        // Register endpoint
        {
            let registry_ref = router.registry();
            let mut registry = registry_ref.write().await;
            let endpoint = EndpointConfig {
                endpoint_id: "anthropic_test".into(),
                provider: "anthropic".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                auth: AuthConfig::None,
                default_headers: std::collections::HashMap::new(),
                tls: super::super::registry::TlsConfig::default(),
                timeout_seconds: 30,
                max_retries: 3,
                available_models: None,
            };
            registry.register_endpoint(endpoint).unwrap();

            // Mark as healthy
            let health = super::super::registry::HealthCheckResult::healthy(100);
            registry.update_health(&"anthropic_test".into(), health);
        }

        let project_id = ProjectId::new();

        // Start run
        let (run_id, config) = router
            .start_run(
                project_id,
                Some("anthropic".to_string()),
                Some("claude-sonnet-3.5".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-sonnet-3.5");

        // Update usage
        router
            .update_usage(run_id, 5, 1000, Some(0.50))
            .await
            .unwrap();

        // Get state
        let state = router.get_run_state(run_id).await.unwrap();
        assert_eq!(state.total_requests, 5);
        assert_eq!(state.total_tokens_used, 1000);

        // End run
        let final_state = router.end_run(run_id).await.unwrap();
        assert_eq!(final_state.run_id, run_id);
    }
}
