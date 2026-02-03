//! Guardrails configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Main guardrails configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    /// Global enable/disable
    pub enabled: bool,

    /// Fail-open or fail-closed on errors
    pub fail_mode: FailMode,

    /// Maximum execution time for all detectors (ms)
    pub timeout_ms: u64,

    /// Prompt injection detector configuration
    pub prompt_injection: DetectorConfig,

    /// PII detector configuration
    pub pii: PiiConfig,

    /// Jailbreak detector configuration
    pub jailbreak: DetectorConfig,

    /// Topic detector configuration
    pub topics: TopicConfig,

    /// Keyword detector configuration
    pub keywords: KeywordConfig,

    /// Secret detector configuration
    pub secrets: DetectorConfig,
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_mode: FailMode::FailClosed,
            timeout_ms: 5000,
            prompt_injection: DetectorConfig {
                enabled: true,
                sensitivity: Sensitivity::Medium,
                confidence_threshold: 0.7,
            },
            pii: PiiConfig {
                enabled: true,
                sensitivity: Sensitivity::High,
                confidence_threshold: 0.8,
                allowed_types: HashSet::new(),
                redact_on_detect: true,
            },
            jailbreak: DetectorConfig {
                enabled: true,
                sensitivity: Sensitivity::Medium,
                confidence_threshold: 0.7,
            },
            topics: TopicConfig {
                enabled: true,
                sensitivity: Sensitivity::Medium,
                confidence_threshold: 0.7,
                banned_topics: vec![
                    "violence".to_string(),
                    "illegal_activities".to_string(),
                    "self_harm".to_string(),
                ],
                allowed_topics: vec![],
                mode: TopicMode::Blocklist,
            },
            keywords: KeywordConfig {
                enabled: true,
                sensitivity: Sensitivity::Medium,
                confidence_threshold: 0.9,
                blocked_keywords: vec![],
                case_sensitive: false,
                use_fuzzy_match: false,
            },
            secrets: DetectorConfig {
                enabled: true,
                sensitivity: Sensitivity::High,
                confidence_threshold: 0.9,
            },
        }
    }
}

/// Base detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Whether this detector is enabled
    pub enabled: bool,

    /// Detection sensitivity level
    pub sensitivity: Sensitivity,

    /// Confidence threshold for triggering detection (0.0 - 1.0)
    pub confidence_threshold: f64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: Sensitivity::Medium,
            confidence_threshold: 0.7,
        }
    }
}

/// PII-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiConfig {
    /// Whether this detector is enabled
    pub enabled: bool,

    /// Detection sensitivity level
    pub sensitivity: Sensitivity,

    /// Confidence threshold
    pub confidence_threshold: f64,

    /// PII types that are allowed (won't trigger detection)
    pub allowed_types: HashSet<String>,

    /// Whether to redact detected PII in logs
    pub redact_on_detect: bool,
}

impl Default for PiiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: Sensitivity::High,
            confidence_threshold: 0.8,
            allowed_types: HashSet::new(),
            redact_on_detect: true,
        }
    }
}

/// Topic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    /// Whether this detector is enabled
    pub enabled: bool,

    /// Detection sensitivity level
    pub sensitivity: Sensitivity,

    /// Confidence threshold
    pub confidence_threshold: f64,

    /// Topics that are banned (blocklist mode)
    pub banned_topics: Vec<String>,

    /// Topics that are allowed (allowlist mode)
    pub allowed_topics: Vec<String>,

    /// Operating mode
    pub mode: TopicMode,
}

impl Default for TopicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: Sensitivity::Medium,
            confidence_threshold: 0.7,
            banned_topics: vec![],
            allowed_topics: vec![],
            mode: TopicMode::Blocklist,
        }
    }
}

/// Keyword-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordConfig {
    /// Whether this detector is enabled
    pub enabled: bool,

    /// Detection sensitivity level
    pub sensitivity: Sensitivity,

    /// Confidence threshold
    pub confidence_threshold: f64,

    /// Keywords to block
    pub blocked_keywords: Vec<String>,

    /// Whether matching is case-sensitive
    pub case_sensitive: bool,

    /// Whether to use fuzzy matching
    pub use_fuzzy_match: bool,
}

impl Default for KeywordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: Sensitivity::Medium,
            confidence_threshold: 0.9,
            blocked_keywords: vec![],
            case_sensitive: false,
            use_fuzzy_match: false,
        }
    }
}

/// Detection sensitivity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Sensitivity {
    /// Fewer false positives, may miss some threats
    Low,
    /// Balanced detection
    #[default]
    Medium,
    /// More aggressive detection, may have more false positives
    High,
}

impl Sensitivity {
    /// Get sensitivity multiplier for confidence scores
    pub fn multiplier(&self) -> f64 {
        match self {
            Sensitivity::Low => 0.8,
            Sensitivity::Medium => 1.0,
            Sensitivity::High => 1.2,
        }
    }
}

/// Fail mode for error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FailMode {
    /// Block on errors (safer, more conservative)
    #[default]
    FailClosed,
    /// Allow through on errors (more permissive)
    FailOpen,
}

/// Topic detection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TopicMode {
    /// Block topics in banned_topics list
    #[default]
    Blocklist,
    /// Only allow topics in allowed_topics list
    Allowlist,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GuardrailsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.fail_mode, FailMode::FailClosed);
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn test_sensitivity_multiplier() {
        assert_eq!(Sensitivity::Low.multiplier(), 0.8);
        assert_eq!(Sensitivity::Medium.multiplier(), 1.0);
        assert_eq!(Sensitivity::High.multiplier(), 1.2);
    }

    #[test]
    fn test_config_serialization() {
        let config = GuardrailsConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: GuardrailsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.enabled, restored.enabled);
        assert_eq!(config.timeout_ms, restored.timeout_ms);
    }
}
