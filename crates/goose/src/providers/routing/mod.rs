//! Provider Routing Module
//!
//! This module implements provider switching, project portability, and fail-safes
//! to ensure projects never get "locked" to a provider and users can seamlessly
//! switch providers when hitting token limits or other issues.

pub mod errors;
pub mod handoff;
pub mod policy;
pub mod portable;
pub mod registry;
pub mod router;
pub mod state;

pub use errors::{RoutingError, RoutingResult};
pub use handoff::{HandoffGenerator, HandoffMemo};
pub use policy::{CapabilityRequirement, FallbackConfig, ProjectProviderPolicy};
pub use portable::{ExportManifest, ImportMapping, PortableContextPack};
pub use registry::{EndpointConfig, EndpointHealth, EndpointId, ProviderRegistry};
pub use router::{ProviderRouter, RouterConfig};
pub use state::{ProviderConfig, ProviderSwitch, RunProviderState, SwitchReason};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Classification of provider errors for auto-fallback decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Bad credentials, forbidden access
    AuthError,
    /// Out of credits/tokens
    QuotaExhausted,
    /// Rate limited
    RateLimited,
    /// Network unreachable
    EndpointUnreachable,
    /// Request timeout
    Timeout,
    /// Model not found/available
    ModelNotFound,
    /// Required capability missing (tools, etc.)
    CapabilityMismatch,
    /// 5xx server errors
    ServerError,
    /// Unknown/unclassified error
    Unknown,
}

impl ErrorCategory {
    /// Whether this error type should trigger auto-fallback
    pub fn should_fallback(&self) -> bool {
        matches!(
            self,
            Self::QuotaExhausted
                | Self::RateLimited
                | Self::EndpointUnreachable
                | Self::Timeout
                | Self::ServerError
        )
    }

    /// Whether this error indicates a configuration issue
    pub fn is_config_error(&self) -> bool {
        matches!(self, Self::AuthError | Self::ModelNotFound)
    }
}

/// Provider capabilities for matching models
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports tool/function calling
    pub tools: bool,
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports JSON schema constraints
    pub json_schema: bool,
    /// Context window size in tokens
    pub context_tokens: u32,
    /// Maximum output tokens
    pub max_output_tokens: Option<u32>,
    /// Supported image formats (if multimodal)
    pub image_formats: Vec<String>,
}

impl ProviderCapabilities {
    /// Check if this provider meets the required capabilities
    pub fn meets_requirements(&self, required: &CapabilityRequirement) -> bool {
        if required.tools && !self.tools {
            return false;
        }
        if required.streaming && !self.streaming {
            return false;
        }
        if required.json_schema && !self.json_schema {
            return false;
        }
        if self.context_tokens < required.min_context_tokens {
            return false;
        }
        true
    }
}

/// Model mapping strategy for fallback chains
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelMappingStrategy {
    /// Use exact same model name
    Exact,
    /// Use most capable model available
    MostCapable,
    /// Use cheapest model that meets requirements
    Cheapest,
    /// Use fastest model that meets requirements
    Fastest,
    /// Use balanced model (good capability/cost ratio)
    Balanced,
    /// Custom mapping table
    Custom(HashMap<String, String>),
}

/// Unique identifier for a project run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "run-{}", &self.0.to_string()[..8])
    }
}

/// Unique identifier for a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(Uuid);

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "proj-{}", &self.0.to_string()[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_fallback() {
        assert!(ErrorCategory::QuotaExhausted.should_fallback());
        assert!(ErrorCategory::RateLimited.should_fallback());
        assert!(!ErrorCategory::AuthError.should_fallback());
    }

    #[test]
    fn test_capability_matching() {
        let caps = ProviderCapabilities {
            tools: true,
            streaming: true,
            json_schema: false,
            context_tokens: 100000,
            max_output_tokens: Some(4096),
            image_formats: vec!["jpeg".to_string(), "png".to_string()],
        };

        let req = CapabilityRequirement {
            tools: true,
            streaming: false,
            json_schema: false,
            min_context_tokens: 50000,
        };

        assert!(caps.meets_requirements(&req));

        let strict_req = CapabilityRequirement {
            tools: true,
            streaming: true,
            json_schema: true, // Not supported
            min_context_tokens: 50000,
        };

        assert!(!caps.meets_requirements(&strict_req));
    }

    #[test]
    fn test_id_generation() {
        let run1 = RunId::new();
        let run2 = RunId::new();
        assert_ne!(run1, run2);

        let proj1 = ProjectId::new();
        let proj2 = ProjectId::new();
        assert_ne!(proj1, proj2);
    }
}
