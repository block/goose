//! Project provider policy definitions

use serde::{Deserialize, Serialize};

use super::{ModelMappingStrategy, ProviderCapabilities, RoutingError, RoutingResult};

/// Capability requirements for a project
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    /// Requires tool/function calling support
    pub tools: bool,
    /// Requires streaming response support
    pub streaming: bool,
    /// Requires JSON schema constraint support
    pub json_schema: bool,
    /// Minimum context window size in tokens
    pub min_context_tokens: u32,
}

impl CapabilityRequirement {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn require_tools(mut self) -> Self {
        self.tools = true;
        self
    }

    pub fn require_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    pub fn require_json_schema(mut self) -> Self {
        self.json_schema = true;
        self
    }

    pub fn with_min_context(mut self, tokens: u32) -> Self {
        self.min_context_tokens = tokens;
        self
    }

    /// Check if provider capabilities meet these requirements
    pub fn is_satisfied_by(&self, capabilities: &ProviderCapabilities) -> bool {
        capabilities.meets_requirements(self)
    }
}

/// Fallback configuration for a provider chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Target provider name
    pub provider: String,
    /// Model mapping strategy
    pub model_map: ModelMappingStrategy,
    /// Maximum attempts before giving up
    pub max_attempts: u32,
    /// Delay between attempts in seconds
    pub retry_delay_seconds: u64,
}

impl FallbackConfig {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model_map: ModelMappingStrategy::Balanced,
            max_attempts: 3,
            retry_delay_seconds: 1,
        }
    }

    pub fn with_strategy(mut self, strategy: ModelMappingStrategy) -> Self {
        self.model_map = strategy;
        self
    }

    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }
}

/// Provider policy for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProviderPolicy {
    /// Whether the project is pinned to a specific provider
    pub pinned: bool,
    /// Pinned provider name (if pinned)
    pub pinned_provider: Option<String>,
    /// Pinned model name (if pinned)
    pub pinned_model: Option<String>,
    /// Default preferred provider
    pub default_provider: String,
    /// Default preferred model
    pub default_model: String,
    /// List of allowed providers (empty = all allowed)
    pub allowed_providers: Vec<String>,
    /// Whether to enable automatic fallback on failures
    pub auto_fallback: bool,
    /// Fallback chain configuration
    pub fallback_chain: Vec<FallbackConfig>,
    /// Required capabilities for this project
    pub capability_requirements: CapabilityRequirement,
    /// Budget constraints (optional)
    pub max_cost_per_request: Option<f64>,
    /// Maximum tokens per request
    pub max_tokens_per_request: Option<u32>,
}

impl ProjectProviderPolicy {
    /// Create a new policy with default settings
    pub fn new(default_provider: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            pinned: false,
            pinned_provider: None,
            pinned_model: None,
            default_provider: default_provider.into(),
            default_model: default_model.into(),
            allowed_providers: Vec::new(), // Empty = all allowed
            auto_fallback: true,
            fallback_chain: Vec::new(),
            capability_requirements: CapabilityRequirement::default(),
            max_cost_per_request: None,
            max_tokens_per_request: None,
        }
    }

    /// Pin the project to a specific provider and model
    pub fn pin_to(mut self, provider: impl Into<String>, model: impl Into<String>) -> Self {
        self.pinned = true;
        self.pinned_provider = Some(provider.into());
        self.pinned_model = Some(model.into());
        self.auto_fallback = false; // Disable fallback when pinned
        self
    }

    /// Unpin the project (allow provider switching)
    pub fn unpin(mut self) -> Self {
        self.pinned = false;
        self.pinned_provider = None;
        self.pinned_model = None;
        self
    }

    /// Set allowed providers list
    pub fn allow_providers(mut self, providers: Vec<String>) -> Self {
        self.allowed_providers = providers;
        self
    }

    /// Add a fallback provider to the chain
    pub fn add_fallback(mut self, fallback: FallbackConfig) -> Self {
        self.fallback_chain.push(fallback);
        self
    }

    /// Set capability requirements
    pub fn with_capabilities(mut self, requirements: CapabilityRequirement) -> Self {
        self.capability_requirements = requirements;
        self
    }

    /// Set budget constraints
    pub fn with_budget(mut self, max_cost: f64) -> Self {
        self.max_cost_per_request = Some(max_cost);
        self
    }

    /// Check if a provider is allowed by this policy
    pub fn is_provider_allowed(&self, provider: &str) -> bool {
        if self.allowed_providers.is_empty() {
            true // Empty list means all providers allowed
        } else {
            self.allowed_providers.contains(&provider.to_string())
        }
    }

    /// Check if switching is allowed
    pub fn allows_switching(&self) -> bool {
        !self.pinned
    }

    /// Get the effective provider to use (pinned or default)
    pub fn get_effective_provider(&self) -> &str {
        self.pinned_provider
            .as_ref()
            .unwrap_or(&self.default_provider)
    }

    /// Get the effective model to use (pinned or default)
    pub fn get_effective_model(&self) -> &str {
        self.pinned_model.as_ref().unwrap_or(&self.default_model)
    }

    /// Validate a provider switch request
    pub fn validate_switch(
        &self,
        target_provider: &str,
        _target_model: &str,
        force: bool,
    ) -> RoutingResult<()> {
        // Check if pinned
        if self.pinned && !force {
            return Err(RoutingError::project_pinned(
                self.pinned_provider.as_ref().unwrap(),
            ));
        }

        // Check if provider is allowed
        if !self.is_provider_allowed(target_provider) {
            return Err(RoutingError::policy_violation(format!(
                "Provider '{}' is not in the allowed list",
                target_provider
            )));
        }

        Ok(())
    }

    /// Get the next fallback provider in the chain
    pub fn get_next_fallback(&self, current_provider: &str) -> Option<&FallbackConfig> {
        // Find current provider in fallback chain
        let current_index = self
            .fallback_chain
            .iter()
            .position(|f| f.provider == current_provider);

        match current_index {
            Some(index) if index + 1 < self.fallback_chain.len() => {
                Some(&self.fallback_chain[index + 1])
            }
            None if !self.fallback_chain.is_empty() => {
                // Current provider not in chain, start from beginning
                Some(&self.fallback_chain[0])
            }
            _ => None, // End of chain or no fallbacks
        }
    }

    /// Map a model name using the specified strategy
    pub fn map_model(&self, source_model: &str, strategy: &ModelMappingStrategy) -> String {
        match strategy {
            ModelMappingStrategy::Exact => source_model.to_string(),
            ModelMappingStrategy::MostCapable => {
                // Would need provider-specific logic here
                source_model.to_string()
            }
            ModelMappingStrategy::Cheapest => {
                // Would need provider-specific logic here
                source_model.to_string()
            }
            ModelMappingStrategy::Fastest => {
                // Would need provider-specific logic here
                source_model.to_string()
            }
            ModelMappingStrategy::Balanced => {
                // Default balanced mappings
                match source_model {
                    "claude-3-5-sonnet-20241022" | "claude-sonnet-3.5" => {
                        "gpt-4o".to_string() // Similar capability tier
                    }
                    "claude-3-haiku-20240307" | "claude-haiku-3" => {
                        "gpt-4o-mini".to_string() // Fast, cheaper tier
                    }
                    "gpt-4o" => "claude-3-5-sonnet-20241022".to_string(),
                    "gpt-4o-mini" => "claude-3-haiku-20240307".to_string(),
                    _ => source_model.to_string(), // No mapping, use as-is
                }
            }
            ModelMappingStrategy::Custom(mapping) => mapping
                .get(source_model)
                .cloned()
                .unwrap_or_else(|| source_model.to_string()),
        }
    }
}

impl Default for ProjectProviderPolicy {
    fn default() -> Self {
        Self::new("anthropic", "claude-3-5-sonnet-20241022")
            .add_fallback(
                FallbackConfig::new("openai").with_strategy(ModelMappingStrategy::Balanced),
            )
            .add_fallback(
                FallbackConfig::new("openrouter").with_strategy(ModelMappingStrategy::Cheapest),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = ProjectProviderPolicy::new("anthropic", "claude-sonnet-3.5");
        assert_eq!(policy.default_provider, "anthropic");
        assert_eq!(policy.default_model, "claude-sonnet-3.5");
        assert!(!policy.pinned);
        assert!(policy.auto_fallback);
    }

    #[test]
    fn test_pinning() {
        let policy =
            ProjectProviderPolicy::new("anthropic", "claude-sonnet-3.5").pin_to("openai", "gpt-4o");

        assert!(policy.pinned);
        assert_eq!(policy.get_effective_provider(), "openai");
        assert_eq!(policy.get_effective_model(), "gpt-4o");
        assert!(!policy.allows_switching());
    }

    #[test]
    fn test_provider_restrictions() {
        let policy = ProjectProviderPolicy::new("anthropic", "claude-sonnet-3.5")
            .allow_providers(vec!["anthropic".to_string(), "openai".to_string()]);

        assert!(policy.is_provider_allowed("anthropic"));
        assert!(policy.is_provider_allowed("openai"));
        assert!(!policy.is_provider_allowed("huggingface"));
    }

    #[test]
    fn test_fallback_chain() {
        let policy = ProjectProviderPolicy::new("anthropic", "claude-sonnet-3.5")
            .add_fallback(FallbackConfig::new("openai"))
            .add_fallback(FallbackConfig::new("openrouter"));

        // First fallback from non-chain provider
        let fallback = policy.get_next_fallback("anthropic").unwrap();
        assert_eq!(fallback.provider, "openai");

        // Next fallback in chain
        let fallback = policy.get_next_fallback("openai").unwrap();
        assert_eq!(fallback.provider, "openrouter");

        // End of chain
        assert!(policy.get_next_fallback("openrouter").is_none());
    }

    #[test]
    fn test_model_mapping() {
        let policy = ProjectProviderPolicy::default();

        // Balanced mapping
        let mapped = policy.map_model(
            "claude-3-5-sonnet-20241022",
            &ModelMappingStrategy::Balanced,
        );
        assert_eq!(mapped, "gpt-4o");

        // Exact mapping
        let mapped = policy.map_model("claude-3-5-sonnet-20241022", &ModelMappingStrategy::Exact);
        assert_eq!(mapped, "claude-3-5-sonnet-20241022");

        // Custom mapping
        let custom_map = [("my-model".to_string(), "mapped-model".to_string())]
            .into_iter()
            .collect();
        let mapped = policy.map_model("my-model", &ModelMappingStrategy::Custom(custom_map));
        assert_eq!(mapped, "mapped-model");
    }

    #[test]
    fn test_capability_requirements() {
        let requirements = CapabilityRequirement::new()
            .require_tools()
            .require_streaming()
            .with_min_context(100000);

        let capabilities = ProviderCapabilities {
            tools: true,
            streaming: true,
            json_schema: false,
            context_tokens: 150000,
            max_output_tokens: Some(4096),
            image_formats: vec![],
        };

        assert!(requirements.is_satisfied_by(&capabilities));

        let insufficient_caps = ProviderCapabilities {
            tools: false, // Missing required capability
            streaming: true,
            json_schema: false,
            context_tokens: 150000,
            max_output_tokens: Some(4096),
            image_formats: vec![],
        };

        assert!(!requirements.is_satisfied_by(&insufficient_caps));
    }
}
