//! Prompts Module
//!
//! Reusable prompt patterns and templates for effective AI interactions.
//! Based on best practices extracted from various system prompts and prompt engineering research.
//!
//! This module provides:
//! - Pre-built prompt patterns for common use cases
//! - Template system with variable substitution
//! - Pattern composition and chaining
//! - Best practice guidelines and documentation

pub mod errors;
pub mod patterns;
pub mod templates;

pub use errors::PromptError;
pub use patterns::{
    Pattern, PatternBuilder, PatternCategory, PatternLibrary, PatternMetadata, PatternRegistry,
};
pub use templates::{Template, TemplateEngine, TemplateVariable, VariableType};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Prompt manager - orchestrates patterns and templates
pub struct PromptManager {
    pattern_registry: Arc<PatternRegistry>,
    template_engine: Arc<TemplateEngine>,
    config: PromptConfig,
}

/// Configuration for the prompt manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Enable prompt caching
    pub cache_enabled: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Default template directory
    pub template_dir: Option<String>,
    /// Enable validation of generated prompts
    pub validation_enabled: bool,
    /// Maximum prompt length (characters)
    pub max_prompt_length: usize,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            max_cache_size: 100,
            template_dir: None,
            validation_enabled: true,
            max_prompt_length: 100_000,
        }
    }
}

impl PromptManager {
    /// Create a new prompt manager with default configuration
    pub fn new() -> Self {
        Self::with_config(PromptConfig::default())
    }

    /// Create a prompt manager with custom configuration
    pub fn with_config(config: PromptConfig) -> Self {
        let pattern_registry = Arc::new(PatternRegistry::with_defaults());
        let template_engine = Arc::new(TemplateEngine::new());

        Self {
            pattern_registry,
            template_engine,
            config,
        }
    }

    /// Get a pattern by name
    pub fn get_pattern(&self, name: &str) -> Option<Pattern> {
        self.pattern_registry.get(name)
    }

    /// Get all patterns in a category
    pub fn get_patterns_by_category(&self, category: PatternCategory) -> Vec<Pattern> {
        self.pattern_registry.get_by_category(category)
    }

    /// List all available patterns
    pub fn list_patterns(&self) -> Vec<PatternMetadata> {
        self.pattern_registry.list()
    }

    /// Render a template with variables
    pub fn render_template(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, PromptError> {
        self.template_engine.render(template_name, variables)
    }

    /// Build a prompt using a pattern
    pub fn build_prompt(&self, pattern_name: &str) -> Result<PatternBuilder, PromptError> {
        let pattern = self
            .pattern_registry
            .get(pattern_name)
            .ok_or_else(|| PromptError::PatternNotFound(pattern_name.to_string()))?;

        Ok(PatternBuilder::new(pattern))
    }

    /// Compose multiple patterns into a single prompt
    pub fn compose_patterns(&self, pattern_names: &[&str]) -> Result<String, PromptError> {
        let mut composed = String::new();

        for (i, name) in pattern_names.iter().enumerate() {
            let pattern = self
                .pattern_registry
                .get(name)
                .ok_or_else(|| PromptError::PatternNotFound(name.to_string()))?;

            if i > 0 {
                composed.push_str("\n\n");
            }
            composed.push_str(&pattern.content);
        }

        if self.config.validation_enabled {
            self.validate_prompt(&composed)?;
        }

        Ok(composed)
    }

    /// Validate a prompt against configured constraints
    pub fn validate_prompt(&self, prompt: &str) -> Result<(), PromptError> {
        if prompt.len() > self.config.max_prompt_length {
            return Err(PromptError::PromptTooLong {
                length: prompt.len(),
                max: self.config.max_prompt_length,
            });
        }

        Ok(())
    }

    /// Get pattern registry for direct access
    pub fn registry(&self) -> &Arc<PatternRegistry> {
        &self.pattern_registry
    }

    /// Get template engine for direct access
    pub fn templates(&self) -> &Arc<TemplateEngine> {
        &self.template_engine
    }

    /// Get statistics about loaded patterns
    pub fn get_stats(&self) -> PromptStats {
        let patterns = self.pattern_registry.list();
        let by_category = patterns.iter().fold(HashMap::new(), |mut acc, p| {
            *acc.entry(p.category).or_insert(0) += 1;
            acc
        });

        PromptStats {
            total_patterns: patterns.len(),
            patterns_by_category: by_category,
            templates_loaded: self.template_engine.template_count(),
        }
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about loaded prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptStats {
    /// Total number of patterns
    pub total_patterns: usize,
    /// Patterns by category
    pub patterns_by_category: HashMap<PatternCategory, usize>,
    /// Number of templates loaded
    pub templates_loaded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_manager_creation() {
        let manager = PromptManager::new();
        let stats = manager.get_stats();
        assert!(stats.total_patterns > 0);
    }

    #[test]
    fn test_prompt_config_default() {
        let config = PromptConfig::default();
        assert!(config.cache_enabled);
        assert!(config.validation_enabled);
        assert_eq!(config.max_prompt_length, 100_000);
    }

    #[test]
    fn test_get_pattern() {
        let manager = PromptManager::new();
        let pattern = manager.get_pattern("chain_of_thought");
        assert!(pattern.is_some());
    }

    #[test]
    fn test_list_patterns() {
        let manager = PromptManager::new();
        let patterns = manager.list_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_validate_prompt() {
        let manager = PromptManager::new();

        // Valid prompt
        assert!(manager.validate_prompt("Hello, world!").is_ok());

        // Too long prompt
        let config = PromptConfig {
            max_prompt_length: 10,
            ..Default::default()
        };
        let manager = PromptManager::with_config(config);
        assert!(manager.validate_prompt("This is too long").is_err());
    }

    #[test]
    fn test_compose_patterns() {
        let manager = PromptManager::new();
        let result = manager.compose_patterns(&["role_definition", "chain_of_thought"]);
        assert!(result.is_ok());
        let composed = result.unwrap();
        assert!(composed.contains("role"));
        assert!(composed.contains("step"));
    }
}
