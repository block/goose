//! RLM (Recursive Language Models) configuration
//!
//! This module provides configuration management for RLM mode, which enables
//! handling arbitrarily long contexts by treating them as external environment
//! variables processed through recursive sub-agent calls.
//!
//! # Configuration
//!
//! RLM settings can be configured in `~/.config/goose/config.yaml`:
//!
//! ```yaml
//! rlm:
//!   enabled: true
//!   context_threshold: 100000  # Minimum chars to trigger RLM mode
//!   chunk_size: 500000         # Target chunk size for processing
//!   max_iterations: 50         # Maximum agent iterations
//!   max_recursion_depth: 1     # Maximum depth for sub-agent calls
//! ```
//!
//! Or via environment variables:
//! - `GOOSE_RLM_ENABLED=true`
//! - `GOOSE_RLM_CONTEXT_THRESHOLD=100000`
//! - `GOOSE_RLM_CHUNK_SIZE=500000`
//! - `GOOSE_RLM_MAX_ITERATIONS=50`
//! - `GOOSE_RLM_MAX_RECURSION_DEPTH=1`

use super::base::Config;
use crate::rlm::RlmConfig;
use anyhow::Result;
use std::env;

/// Manager for RLM configuration
pub struct RlmConfigManager;

impl RlmConfigManager {
    /// Get the current RLM configuration.
    ///
    /// Loads from:
    /// 1. Environment variables (highest priority)
    /// 2. Config file (~/.config/goose/config.yaml)
    /// 3. Default values (lowest priority)
    pub fn get() -> RlmConfig {
        let config = Config::global();

        // Start with defaults
        let mut rlm_config = RlmConfig::default();

        // Try to load from config file
        if let Ok(file_config) = config.get_param::<RlmConfig>("rlm") {
            rlm_config = file_config;
        }

        // Override with environment variables
        if let Ok(val) = env::var("GOOSE_RLM_ENABLED") {
            if let Ok(enabled) = val.parse() {
                rlm_config.enabled = enabled;
            }
        }

        if let Ok(val) = env::var("GOOSE_RLM_CONTEXT_THRESHOLD") {
            if let Ok(threshold) = val.parse() {
                rlm_config.context_threshold = threshold;
            }
        }

        if let Ok(val) = env::var("GOOSE_RLM_CHUNK_SIZE") {
            if let Ok(size) = val.parse() {
                rlm_config.chunk_size = size;
            }
        }

        if let Ok(val) = env::var("GOOSE_RLM_MAX_ITERATIONS") {
            if let Ok(iterations) = val.parse() {
                rlm_config.max_iterations = iterations;
            }
        }

        if let Ok(val) = env::var("GOOSE_RLM_MAX_RECURSION_DEPTH") {
            if let Ok(depth) = val.parse() {
                rlm_config.max_recursion_depth = depth;
            }
        }

        rlm_config
    }

    /// Save RLM configuration to the config file
    pub fn set(rlm_config: &RlmConfig) -> Result<()> {
        let config = Config::global();
        config.set_param("rlm", rlm_config)?;
        Ok(())
    }

    /// Check if RLM mode is enabled
    pub fn is_enabled() -> bool {
        Self::get().enabled
    }

    /// Enable or disable RLM mode
    pub fn set_enabled(enabled: bool) -> Result<()> {
        let mut config = Self::get();
        config.enabled = enabled;
        Self::set(&config)
    }

    /// Get the context threshold for triggering RLM mode
    pub fn get_context_threshold() -> usize {
        Self::get().context_threshold
    }

    /// Set the context threshold for triggering RLM mode
    pub fn set_context_threshold(threshold: usize) -> Result<()> {
        let mut config = Self::get();
        config.context_threshold = threshold;
        Self::set(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rlm_config_defaults() {
        // Without any config, should return defaults
        let config = RlmConfig::default();
        assert!(config.enabled);
        assert_eq!(config.context_threshold, 100_000);
        assert_eq!(config.chunk_size, 500_000);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_recursion_depth, 1);
    }

    #[test]
    fn test_env_override() {
        // Save original values
        let orig_enabled = env::var("GOOSE_RLM_ENABLED").ok();
        let orig_threshold = env::var("GOOSE_RLM_CONTEXT_THRESHOLD").ok();

        // Set environment variables
        env::set_var("GOOSE_RLM_ENABLED", "false");
        env::set_var("GOOSE_RLM_CONTEXT_THRESHOLD", "50000");

        let config = RlmConfigManager::get();
        assert!(!config.enabled);
        assert_eq!(config.context_threshold, 50_000);

        // Restore original values
        match orig_enabled {
            Some(v) => env::set_var("GOOSE_RLM_ENABLED", v),
            None => env::remove_var("GOOSE_RLM_ENABLED"),
        }
        match orig_threshold {
            Some(v) => env::set_var("GOOSE_RLM_CONTEXT_THRESHOLD", v),
            None => env::remove_var("GOOSE_RLM_CONTEXT_THRESHOLD"),
        }
    }
}
