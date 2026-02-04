//! Memory Retrieval Module
//!
//! Handles search, reranking, and retrieval of memories across all tiers.
//! Implements weighted scoring and context-aware filtering.

use super::{MemoryEntry, MemoryResult, RecallContext};

/// Memory retriever that handles search and reranking
#[derive(Debug)]
pub struct MemoryRetriever {
    /// Default maximum results
    default_max_results: usize,
    /// Enable recency boosting
    recency_boost: bool,
    /// Enable diversity in results
    diversity_penalty: f64,
}

impl MemoryRetriever {
    /// Create a new retriever
    pub fn new() -> Self {
        Self {
            default_max_results: 10,
            recency_boost: true,
            diversity_penalty: 0.1,
        }
    }

    /// Create with custom max results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.default_max_results = max;
        self
    }

    /// Enable or disable recency boosting
    pub fn with_recency_boost(mut self, enabled: bool) -> Self {
        self.recency_boost = enabled;
        self
    }

    /// Set diversity penalty (0.0 = no penalty, 1.0 = strong penalty for similar results)
    pub fn with_diversity_penalty(mut self, penalty: f64) -> Self {
        self.diversity_penalty = penalty.clamp(0.0, 1.0);
        self
    }

    /// Rerank a list of memories based on query and context
    pub fn rerank(
        &self,
        memories: Vec<MemoryEntry>,
        query: &str,
        context: &RecallContext,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        if memories.is_empty() {
            return Ok(Vec::new());
        }

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        // Score each memory
        let mut scored: Vec<(f64, MemoryEntry)> = memories
            .into_iter()
            .map(|entry| {
                let score = self.calculate_score(&entry, &query_words, context);
                (score, entry)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Apply diversity penalty if enabled
        if self.diversity_penalty > 0.0 {
            scored = self.apply_diversity_penalty(scored);
        }

        // Filter by minimum relevance
        let results: Vec<MemoryEntry> = scored
            .into_iter()
            .filter(|(score, _)| *score >= context.min_relevance)
            .take(context.max_results)
            .map(|(_, entry)| entry)
            .collect();

        Ok(results)
    }

    /// Calculate the relevance score for a memory entry
    fn calculate_score(
        &self,
        entry: &MemoryEntry,
        query_words: &[&str],
        context: &RecallContext,
    ) -> f64 {
        // Text similarity component
        let text_score = self.calculate_text_similarity(&entry.content, query_words);

        // Recency component
        let recency_score = if self.recency_boost {
            self.calculate_recency_score(entry)
        } else {
            0.5
        };

        // Importance component
        let importance_score = entry.importance_score;

        // Access frequency component
        let access_score = self.calculate_access_score(entry);

        // Weighted combination
        let score = text_score * context.similarity_weight
            + recency_score * context.recency_weight
            + importance_score * context.importance_weight
            + access_score * context.access_weight;

        // Apply confidence multiplier
        score * entry.metadata.confidence
    }

    /// Calculate text similarity using word overlap
    fn calculate_text_similarity(&self, content: &str, query_words: &[&str]) -> f64 {
        if query_words.is_empty() {
            return 0.0;
        }

        let content_lower = content.to_lowercase();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        if content_words.is_empty() {
            return 0.0;
        }

        let mut total_score = 0.0;

        for qw in query_words {
            let mut best_match: f64 = 0.0;

            for cw in &content_words {
                let match_score: f64 = if *cw == *qw {
                    1.0 // Exact match
                } else if cw.contains(qw) || qw.contains(cw) {
                    0.7 // Partial match
                } else if levenshtein_distance(cw, qw) <= 2 {
                    0.5 // Close match (typo tolerance)
                } else {
                    0.0
                };

                best_match = best_match.max(match_score);
            }

            total_score += best_match;
        }

        total_score / query_words.len() as f64
    }

    /// Calculate recency score based on last access time
    fn calculate_recency_score(&self, entry: &MemoryEntry) -> f64 {
        let hours_since = (chrono::Utc::now() - entry.accessed_at).num_hours() as f64;

        // Exponential decay: 50% after 24 hours, ~10% after 72 hours
        (-0.029 * hours_since).exp()
    }

    /// Calculate access frequency score
    fn calculate_access_score(&self, entry: &MemoryEntry) -> f64 {
        // Logarithmic scaling: diminishing returns for very high access counts
        let access_factor = (entry.access_count as f64 + 1.0).ln() / 10.0;
        access_factor.min(1.0)
    }

    /// Apply diversity penalty to reduce similar results
    fn apply_diversity_penalty(
        &self,
        mut scored: Vec<(f64, MemoryEntry)>,
    ) -> Vec<(f64, MemoryEntry)> {
        if scored.len() <= 1 {
            return scored;
        }

        let mut result = Vec::with_capacity(scored.len());

        // Always keep the top result
        let first = scored.remove(0);
        let mut selected_contents: Vec<String> = vec![first.1.content.clone()];
        result.push(first);

        // Apply penalty to remaining based on similarity to already selected
        for (mut score, entry) in scored {
            let max_similarity = selected_contents
                .iter()
                .map(|c| self.content_similarity(&entry.content, c))
                .fold(0.0f64, |a, b| a.max(b));

            // Apply penalty proportional to similarity
            score *= 1.0 - (self.diversity_penalty * max_similarity);

            selected_contents.push(entry.content.clone());
            result.push((score, entry));
        }

        // Re-sort after penalty
        result.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        result
    }

    /// Calculate content similarity between two strings
    fn content_similarity(&self, a: &str, b: &str) -> f64 {
        let lower_a = a.to_lowercase();
        let lower_b = b.to_lowercase();
        let words_a: std::collections::HashSet<&str> = lower_a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = lower_b.split_whitespace().collect();

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Search multiple memory sources and combine results
    pub fn search_and_merge(
        &self,
        sources: Vec<Vec<MemoryEntry>>,
        query: &str,
        context: &RecallContext,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        // Flatten all sources
        let all_entries: Vec<MemoryEntry> = sources.into_iter().flatten().collect();

        // Rerank the combined results
        self.rerank(all_entries, query, context)
    }
}

impl Default for MemoryRetriever {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Quick check for equality
    if s1 == s2 {
        return 0;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for (i, c1) in s1_chars.iter().enumerate() {
        curr_row[0] = i + 1;

        for (j, c2) in s2_chars.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };

            curr_row[j + 1] = (prev_row[j + 1] + 1) // deletion
                .min(curr_row[j] + 1) // insertion
                .min(prev_row[j] + cost); // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryMetadata, MemoryType};
    use chrono::{Duration, Utc};

    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Semantic, content)
            .with_id(id)
            .with_importance(0.5)
    }

    #[test]
    fn test_retriever_creation() {
        let retriever = MemoryRetriever::new();
        assert_eq!(retriever.default_max_results, 10);
        assert!(retriever.recency_boost);
    }

    #[test]
    fn test_retriever_builder() {
        let retriever = MemoryRetriever::new()
            .with_max_results(20)
            .with_recency_boost(false)
            .with_diversity_penalty(0.5);

        assert_eq!(retriever.default_max_results, 20);
        assert!(!retriever.recency_boost);
        assert!((retriever.diversity_penalty - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rerank_empty() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext::default();
        let result = retriever.rerank(vec![], "test query", &context).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_rerank_single() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext::default();
        let entries = vec![create_test_entry("1", "test content")];

        let result = retriever.rerank(entries, "test", &context).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "1");
    }

    #[test]
    fn test_rerank_by_relevance() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext::default();

        let entries = vec![
            create_test_entry("1", "completely unrelated content"),
            create_test_entry("2", "dark mode preference settings"),
            create_test_entry("3", "dark theme enabled"),
        ];

        let result = retriever.rerank(entries, "dark mode", &context).unwrap();

        // Entries mentioning "dark" should rank higher
        let ids: Vec<&str> = result.iter().map(|e| e.id.as_str()).collect();
        assert!(ids[0] == "2" || ids[0] == "3");
    }

    #[test]
    fn test_rerank_by_importance() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext {
            importance_weight: 0.9,
            similarity_weight: 0.1,
            recency_weight: 0.0,
            access_weight: 0.0,
            ..Default::default()
        };

        let entries = vec![
            create_test_entry("1", "test content").with_importance(0.2),
            create_test_entry("2", "test content").with_importance(0.9),
        ];

        let result = retriever.rerank(entries, "test", &context).unwrap();
        assert_eq!(result[0].id, "2"); // Higher importance
    }

    #[test]
    fn test_rerank_min_relevance_filter() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext {
            min_relevance: 0.8,
            ..Default::default()
        };

        let entries = vec![
            create_test_entry("1", "completely unrelated xyz abc"),
            create_test_entry("2", "test content matching query"),
        ];

        let result = retriever.rerank(entries, "test content", &context).unwrap();

        // Unrelated entry should be filtered out
        assert!(result.len() <= 1);
        if !result.is_empty() {
            assert_eq!(result[0].id, "2");
        }
    }

    #[test]
    fn test_text_similarity_exact() {
        let retriever = MemoryRetriever::new();
        let score = retriever.calculate_text_similarity("hello world", &["hello", "world"]);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_text_similarity_partial() {
        let retriever = MemoryRetriever::new();
        let score = retriever.calculate_text_similarity("hello universe", &["hello", "world"]);
        assert!(score > 0.3); // At least "hello" matches
        assert!(score < 1.0); // "world" doesn't match
    }

    #[test]
    fn test_text_similarity_none() {
        let retriever = MemoryRetriever::new();
        let score = retriever.calculate_text_similarity("foo bar", &["hello", "world"]);
        assert!(score < 0.1);
    }

    #[test]
    fn test_recency_score() {
        let retriever = MemoryRetriever::new();

        let mut recent_entry = create_test_entry("1", "test");
        recent_entry.accessed_at = Utc::now();

        let mut old_entry = create_test_entry("2", "test");
        old_entry.accessed_at = Utc::now() - Duration::hours(72);

        let recent_score = retriever.calculate_recency_score(&recent_entry);
        let old_score = retriever.calculate_recency_score(&old_entry);

        assert!(recent_score > old_score);
        assert!(recent_score > 0.9); // Very recent
        assert!(old_score < 0.2); // 72 hours old
    }

    #[test]
    fn test_access_score() {
        let retriever = MemoryRetriever::new();

        let mut low_access = create_test_entry("1", "test");
        low_access.access_count = 1;

        let mut high_access = create_test_entry("2", "test");
        high_access.access_count = 100;

        let low_score = retriever.calculate_access_score(&low_access);
        let high_score = retriever.calculate_access_score(&high_access);

        assert!(high_score > low_score);
        assert!(high_score <= 1.0);
    }

    #[test]
    fn test_diversity_penalty() {
        let retriever = MemoryRetriever::new().with_diversity_penalty(0.5);

        let scored = vec![
            (1.0, create_test_entry("1", "dark mode preference")),
            (0.9, create_test_entry("2", "dark mode settings")), // Similar to 1
            (0.8, create_test_entry("3", "completely different topic")),
        ];

        let result = retriever.apply_diversity_penalty(scored);

        // After penalty, the different topic might rank higher than the similar one
        // Entry 1 stays on top
        assert_eq!(result[0].1.id, "1");
    }

    #[test]
    fn test_search_and_merge() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext::default();

        let source1 = vec![create_test_entry("1", "dark mode enabled")];
        let source2 = vec![create_test_entry("2", "dark theme settings")];

        let result = retriever
            .search_and_merge(vec![source1, source2], "dark mode", &context)
            .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", "world"), 5);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_typo_tolerance() {
        let retriever = MemoryRetriever::new();

        // "prefernce" is 1 edit away from "preference"
        let score = retriever.calculate_text_similarity("user preference", &["prefernce"]);
        assert!(score > 0.3); // Should still match due to typo tolerance
    }

    #[test]
    fn test_content_similarity() {
        let retriever = MemoryRetriever::new();

        let sim1 = retriever.content_similarity("the quick brown fox", "the quick brown dog");
        let sim2 =
            retriever.content_similarity("the quick brown fox", "completely different content");

        assert!(sim1 > sim2);
        assert!(sim1 > 0.5);
    }

    #[test]
    fn test_confidence_multiplier() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext::default();

        let entry1 = create_test_entry("1", "test query content")
            .with_metadata(MemoryMetadata::default().confidence(1.0));
        let entry2 = create_test_entry("2", "test query content")
            .with_metadata(MemoryMetadata::default().confidence(0.5));

        let entries = vec![entry1, entry2];
        let result = retriever.rerank(entries, "test", &context).unwrap();

        // Higher confidence should rank first
        assert_eq!(result[0].id, "1");
    }

    #[test]
    fn test_max_results_limit() {
        let retriever = MemoryRetriever::new();
        let context = RecallContext {
            max_results: 2,
            ..Default::default()
        };

        let entries = vec![
            create_test_entry("1", "test"),
            create_test_entry("2", "test"),
            create_test_entry("3", "test"),
            create_test_entry("4", "test"),
        ];

        let result = retriever.rerank(entries, "test", &context).unwrap();
        assert_eq!(result.len(), 2);
    }
}
