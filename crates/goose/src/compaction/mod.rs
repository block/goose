//! Compaction Module - Context management for long-running agents
//!
//! Provides Claude Code-style compaction capabilities:
//! - Automatic context summarization when limits approach
//! - Preserves critical information while reducing tokens
//! - Configurable compaction strategies
//! - Pre-compact hooks for custom handling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    pub trigger_threshold: f32,
    pub target_reduction: f32,
    pub preserve_recent_messages: usize,
    pub preserve_system_prompts: bool,
    pub preserve_tool_results: bool,
    pub summary_max_tokens: usize,
    pub strategy: CompactionStrategy,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            trigger_threshold: 0.85,
            target_reduction: 0.5,
            preserve_recent_messages: 10,
            preserve_system_prompts: true,
            preserve_tool_results: true,
            summary_max_tokens: 2000,
            strategy: CompactionStrategy::Summarize,
        }
    }
}

/// Strategy for compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    #[default]
    Summarize,
    Truncate,
    Selective,
    Hybrid,
}

/// Trigger for compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    Manual,
    Auto,
    Threshold,
    Command,
}

/// Result of compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub success: bool,
    pub original_tokens: usize,
    pub compacted_tokens: usize,
    pub tokens_saved: usize,
    pub reduction_percent: f32,
    pub summary: String,
    pub preserved_messages: usize,
    pub trigger: CompactionTrigger,
    pub timestamp: DateTime<Utc>,
}

impl CompactionResult {
    pub fn new(
        original: usize,
        compacted: usize,
        summary: String,
        trigger: CompactionTrigger,
    ) -> Self {
        let saved = original.saturating_sub(compacted);
        let reduction = if original > 0 {
            (saved as f32 / original as f32) * 100.0
        } else {
            0.0
        };

        Self {
            success: true,
            original_tokens: original,
            compacted_tokens: compacted,
            tokens_saved: saved,
            reduction_percent: reduction,
            summary,
            preserved_messages: 0,
            trigger,
            timestamp: Utc::now(),
        }
    }
}

/// Message to be compacted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactableMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub token_count: usize,
    pub timestamp: DateTime<Utc>,
    pub importance: MessageImportance,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageImportance {
    Critical,
    High,
    #[default]
    Normal,
    Low,
    Disposable,
}

/// Compaction manager
pub struct CompactionManager {
    config: CompactionConfig,
    history: Vec<CompactionResult>,
}

impl CompactionManager {
    pub fn new(config: CompactionConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
        }
    }

    /// Check if compaction should be triggered
    pub fn should_compact(&self, current_tokens: usize, max_tokens: usize) -> bool {
        let usage = current_tokens as f32 / max_tokens as f32;
        usage >= self.config.trigger_threshold
    }

    /// Compact messages
    pub async fn compact(&mut self, messages: Vec<CompactableMessage>) -> CompactionResult {
        let original_tokens: usize = messages.iter().map(|m| m.token_count).sum();

        let (preserved, to_summarize) = self.partition_messages(messages);

        let summary = self.generate_summary(&to_summarize).await;
        let summary_tokens = self.estimate_tokens(&summary);

        let preserved_tokens: usize = preserved.iter().map(|m| m.token_count).sum();
        let compacted_tokens = preserved_tokens + summary_tokens;

        let result = CompactionResult {
            success: true,
            original_tokens,
            compacted_tokens,
            tokens_saved: original_tokens.saturating_sub(compacted_tokens),
            reduction_percent: if original_tokens > 0 {
                ((original_tokens - compacted_tokens) as f32 / original_tokens as f32) * 100.0
            } else {
                0.0
            },
            summary,
            preserved_messages: preserved.len(),
            trigger: CompactionTrigger::Auto,
            timestamp: Utc::now(),
        };

        self.history.push(result.clone());
        result
    }

    fn partition_messages(
        &self,
        messages: Vec<CompactableMessage>,
    ) -> (Vec<CompactableMessage>, Vec<CompactableMessage>) {
        let total = messages.len();
        let preserve_count = self.config.preserve_recent_messages.min(total);

        let mut preserved = Vec::new();
        let mut to_summarize = Vec::new();

        for (i, msg) in messages.into_iter().enumerate() {
            let is_recent = i >= total - preserve_count;
            let is_system = msg.role == MessageRole::System && self.config.preserve_system_prompts;
            let is_critical = msg.importance == MessageImportance::Critical;

            if is_recent || is_system || is_critical {
                preserved.push(msg);
            } else {
                to_summarize.push(msg);
            }
        }

        (preserved, to_summarize)
    }

    async fn generate_summary(&self, messages: &[CompactableMessage]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        // In a real implementation, this would use an LLM to generate a summary
        let mut summary = String::from("## Conversation Summary\n\n");

        // Group by role
        let mut user_queries = Vec::new();
        let mut assistant_responses = Vec::new();
        let mut tool_results = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => user_queries.push(&msg.content),
                MessageRole::Assistant => assistant_responses.push(&msg.content),
                MessageRole::Tool => tool_results.push(&msg.content),
                _ => {}
            }
        }

        if !user_queries.is_empty() {
            summary.push_str(&format!("**User queries:** {} total\n", user_queries.len()));
        }
        if !assistant_responses.is_empty() {
            summary.push_str(&format!(
                "**Assistant responses:** {} total\n",
                assistant_responses.len()
            ));
        }
        if !tool_results.is_empty() {
            summary.push_str(&format!(
                "**Tool executions:** {} total\n",
                tool_results.len()
            ));
        }

        summary
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 characters per token
        text.len() / 4
    }

    /// Get compaction history
    pub fn history(&self) -> &[CompactionResult] {
        &self.history
    }

    /// Get total tokens saved across all compactions
    pub fn total_tokens_saved(&self) -> usize {
        self.history.iter().map(|r| r.tokens_saved).sum()
    }

    /// Get statistics
    pub fn stats(&self) -> CompactionStats {
        let total_compactions = self.history.len();
        let total_saved = self.total_tokens_saved();
        let avg_reduction = if total_compactions > 0 {
            self.history
                .iter()
                .map(|r| r.reduction_percent)
                .sum::<f32>()
                / total_compactions as f32
        } else {
            0.0
        };

        CompactionStats {
            total_compactions,
            total_tokens_saved: total_saved,
            average_reduction_percent: avg_reduction,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionStats {
    pub total_compactions: usize,
    pub total_tokens_saved: usize,
    pub average_reduction_percent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionConfig::default();
        assert_eq!(config.trigger_threshold, 0.85);
        assert_eq!(config.preserve_recent_messages, 10);
    }

    #[test]
    fn test_should_compact() {
        let manager = CompactionManager::new(CompactionConfig::default());

        assert!(!manager.should_compact(8000, 10000)); // 80% - below threshold
        assert!(manager.should_compact(8600, 10000)); // 86% - above threshold
    }

    #[test]
    fn test_compaction_result() {
        let result = CompactionResult::new(
            10000,
            5000,
            "Test summary".to_string(),
            CompactionTrigger::Auto,
        );

        assert_eq!(result.tokens_saved, 5000);
        assert_eq!(result.reduction_percent, 50.0);
    }

    #[test]
    fn test_message_importance_ordering() {
        assert!(MessageImportance::Critical < MessageImportance::High);
        assert!(MessageImportance::High < MessageImportance::Normal);
        assert!(MessageImportance::Normal < MessageImportance::Low);
    }

    #[tokio::test]
    async fn test_compaction_manager() {
        let mut manager = CompactionManager::new(CompactionConfig::default());

        let messages = vec![CompactableMessage {
            id: "1".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            token_count: 10,
            timestamp: Utc::now(),
            importance: MessageImportance::Normal,
            metadata: HashMap::new(),
        }];

        let result = manager.compact(messages).await;
        assert!(result.success);
    }
}
