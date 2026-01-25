//! RLM (Recursive Language Models) support for Goose
//!
//! This module implements the RLM technique from the paper "Recursive Language Models"
//! (arXiv:2512.24601) which enables handling arbitrarily long contexts by treating them
//! as external environment variables that can be programmatically examined, decomposed,
//! and recursively processed through sub-agent calls.
//! See https://arxiv.org/abs/2512.24601 for more info.

pub mod context_store;
pub mod prompts;

use serde::{Deserialize, Serialize};

/// Configuration for RLM mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    /// Whether RLM mode is enabled
    pub enabled: bool,
    /// Character threshold above which RLM mode is auto-enabled
    pub context_threshold: usize,
    /// Target chunk size for sub-agent calls (in characters)
    pub chunk_size: usize,
    /// Maximum iterations before forcing completion
    pub max_iterations: u32,
    /// Maximum recursion depth for sub-agent calls
    pub max_recursion_depth: u32,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_threshold: 100_000,  // 100K characters
            chunk_size: 500_000,         // 500K characters per chunk
            max_iterations: 50,
            max_recursion_depth: 1,
        }
    }
}

/// Check if a given content should trigger RLM mode
pub fn is_rlm_candidate(content: &str, config: &RlmConfig) -> bool {
    config.enabled && content.len() > config.context_threshold
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_rlm_config_default() {
        let config = RlmConfig::default();
        assert!(config.enabled);
        assert_eq!(config.context_threshold, 100_000);
        assert_eq!(config.chunk_size, 500_000);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_recursion_depth, 1);
    }

    #[test]
    fn test_is_rlm_candidate() {
        let config = RlmConfig::default();

        // Below threshold
        let small_content = "a".repeat(50_000);
        assert!(!is_rlm_candidate(&small_content, &config));

        // Above threshold
        let large_content = "a".repeat(150_000);
        assert!(is_rlm_candidate(&large_content, &config));

        // Disabled config
        let disabled_config = RlmConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!is_rlm_candidate(&large_content, &disabled_config));
    }
}

/// Test utilities for RLM (needle-in-haystack generators, etc.)
/// These are public to allow integration tests to use them.
pub mod test_utils;
