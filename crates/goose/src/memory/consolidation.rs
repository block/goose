//! Memory Consolidation Module
//!
//! Handles the promotion and consolidation of memories across tiers.
//! Working → Episodic → Semantic based on importance, access patterns, and age.

use super::{
    ConsolidationReport, EpisodicMemory, MemoryEntry, MemoryResult, MemoryType, SemanticStore,
    WorkingMemory,
};

/// Configuration for memory consolidation
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Minimum importance to promote from working to episodic
    pub working_to_episodic_importance: f64,
    /// Minimum access count to promote from working to episodic
    pub working_to_episodic_access: u64,
    /// Minimum importance to promote from episodic to semantic
    pub episodic_to_semantic_importance: f64,
    /// Minimum access count to promote from episodic to semantic
    pub episodic_to_semantic_access: u64,
    /// Minimum age (hours) for episodic to semantic promotion
    pub min_age_for_semantic_hours: f64,
    /// Whether to remove entries below threshold
    pub prune_low_importance: bool,
    /// Threshold below which to prune
    pub prune_threshold: f64,
    /// Whether to merge similar entries
    pub merge_similar: bool,
    /// Similarity threshold for merging
    pub merge_similarity_threshold: f64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            working_to_episodic_importance: 0.5,
            working_to_episodic_access: 2,
            episodic_to_semantic_importance: 0.7,
            episodic_to_semantic_access: 5,
            min_age_for_semantic_hours: 24.0,
            prune_low_importance: true,
            prune_threshold: 0.1,
            merge_similar: false,
            merge_similarity_threshold: 0.9,
        }
    }
}

impl ConsolidationConfig {
    /// Create a config that promotes more aggressively
    pub fn aggressive() -> Self {
        Self {
            working_to_episodic_importance: 0.3,
            working_to_episodic_access: 1,
            episodic_to_semantic_importance: 0.5,
            episodic_to_semantic_access: 3,
            min_age_for_semantic_hours: 1.0,
            prune_low_importance: true,
            prune_threshold: 0.05,
            merge_similar: true,
            merge_similarity_threshold: 0.85,
        }
    }

    /// Create a config that is more conservative
    pub fn conservative() -> Self {
        Self {
            working_to_episodic_importance: 0.7,
            working_to_episodic_access: 5,
            episodic_to_semantic_importance: 0.85,
            episodic_to_semantic_access: 10,
            min_age_for_semantic_hours: 72.0,
            prune_low_importance: false,
            prune_threshold: 0.05,
            merge_similar: false,
            merge_similarity_threshold: 0.95,
        }
    }
}

/// Memory consolidator that manages tier promotions
#[derive(Debug)]
pub struct MemoryConsolidator {
    /// Consolidation threshold (number of working memory entries to trigger)
    threshold: usize,
    /// Consolidation configuration
    config: ConsolidationConfig,
    /// Total consolidations performed
    consolidation_count: u64,
}

impl MemoryConsolidator {
    /// Create a new consolidator with default config
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            config: ConsolidationConfig::default(),
            consolidation_count: 0,
        }
    }

    /// Create with custom config
    pub fn with_config(threshold: usize, config: ConsolidationConfig) -> Self {
        Self {
            threshold,
            config,
            consolidation_count: 0,
        }
    }

    /// Get the consolidation threshold
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get consolidation count
    pub fn consolidation_count(&self) -> u64 {
        self.consolidation_count
    }

    /// Perform consolidation across all memory tiers
    pub fn consolidate(
        &self,
        working: &mut WorkingMemory,
        episodic: &mut EpisodicMemory,
        semantic: &mut SemanticStore,
    ) -> MemoryResult<ConsolidationReport> {
        let mut report = ConsolidationReport {
            working_to_episodic: 0,
            promoted_to_semantic: 0,
            merged: 0,
            removed: 0,
        };

        // Step 1: Promote from working to episodic
        let working_to_promote = working.drain_promotable(
            self.config.working_to_episodic_importance,
            self.config.working_to_episodic_access,
        );

        for mut entry in working_to_promote {
            // Skip if below prune threshold (will be pruned in step 3)
            if self.config.prune_low_importance
                && entry.importance_score < self.config.prune_threshold
            {
                continue;
            }
            entry.memory_type = MemoryType::Episodic;
            entry.decay_factor = MemoryType::Episodic.default_decay_factor();
            episodic.store(entry)?;
            report.working_to_episodic += 1;
        }

        // Step 2: Promote from episodic to semantic
        let episodic_to_promote = episodic.drain_promotable(
            self.config.episodic_to_semantic_importance,
            self.config.episodic_to_semantic_access,
        );

        for mut entry in episodic_to_promote {
            // Check age requirement
            let age_hours = (chrono::Utc::now() - entry.created_at).num_hours() as f64;
            if age_hours >= self.config.min_age_for_semantic_hours {
                entry.memory_type = MemoryType::Semantic;
                entry.decay_factor = MemoryType::Semantic.default_decay_factor();
                semantic.store(entry)?;
                report.promoted_to_semantic += 1;
            } else {
                // Put it back in episodic
                entry.memory_type = MemoryType::Episodic;
                episodic.store(entry)?;
            }
        }

        // Step 3: Prune low importance entries if enabled
        if self.config.prune_low_importance {
            let threshold = self.config.prune_threshold;

            // Prune working memory
            let working_entries: Vec<_> = working
                .all()
                .iter()
                .filter(|e| e.importance_score < threshold)
                .map(|e| e.id.clone())
                .collect();
            for id in working_entries {
                working.delete(&id)?;
                report.removed += 1;
            }
        }

        Ok(report)
    }

    /// Check if consolidation should be triggered
    pub fn should_consolidate(&self, working_count: usize) -> bool {
        working_count >= self.threshold
    }

    /// Get the current configuration
    pub fn config(&self) -> &ConsolidationConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ConsolidationConfig) {
        self.config = config;
    }
}

/// Merge two memory entries into one
pub fn merge_entries(entry1: &MemoryEntry, entry2: &MemoryEntry) -> MemoryEntry {
    // Keep the more important one as the base
    let (base, other) = if entry1.importance_score >= entry2.importance_score {
        (entry1, entry2)
    } else {
        (entry2, entry1)
    };

    let mut merged = base.clone();

    // Combine content if different
    if base.content != other.content {
        merged.content = format!("{}\n\n[Related:]\n{}", base.content, other.content);
    }

    // Take the higher importance
    merged.importance_score = base.importance_score.max(other.importance_score);

    // Sum access counts
    merged.access_count = base.access_count + other.access_count;

    // Keep the oldest creation time
    merged.created_at = base.created_at.min(other.created_at);

    // Keep the newest access time
    merged.accessed_at = base.accessed_at.max(other.accessed_at);

    // Merge tags
    let mut tags = base.metadata.tags.clone();
    for tag in &other.metadata.tags {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }
    merged.metadata.tags = tags;

    // Take the higher confidence
    merged.metadata.confidence = base.metadata.confidence.max(other.metadata.confidence);

    merged
}

/// Calculate text similarity between two entries (for merge detection)
pub fn calculate_entry_similarity(entry1: &MemoryEntry, entry2: &MemoryEntry) -> f64 {
    let lower1 = entry1.content.to_lowercase();
    let lower2 = entry2.content.to_lowercase();
    let words1: std::collections::HashSet<&str> = lower1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = lower2.split_whitespace().collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMetadata;

    fn create_working_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Working, content).with_id(id)
    }

    #[allow(dead_code)]
    fn create_episodic_entry(id: &str, content: &str, session: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Episodic, content)
            .with_id(id)
            .with_metadata(MemoryMetadata::default().session(session))
    }

    #[test]
    fn test_consolidator_creation() {
        let consolidator = MemoryConsolidator::new(50);
        assert_eq!(consolidator.threshold(), 50);
        assert_eq!(consolidator.consolidation_count(), 0);
    }

    #[test]
    fn test_should_consolidate() {
        let consolidator = MemoryConsolidator::new(10);
        assert!(!consolidator.should_consolidate(5));
        assert!(consolidator.should_consolidate(10));
        assert!(consolidator.should_consolidate(15));
    }

    #[test]
    fn test_config_default() {
        let config = ConsolidationConfig::default();
        assert!((config.working_to_episodic_importance - 0.5).abs() < 0.01);
        assert_eq!(config.working_to_episodic_access, 2);
        assert!(config.prune_low_importance);
    }

    #[test]
    fn test_config_aggressive() {
        let config = ConsolidationConfig::aggressive();
        assert!(config.working_to_episodic_importance < 0.5);
        assert_eq!(config.working_to_episodic_access, 1);
    }

    #[test]
    fn test_config_conservative() {
        let config = ConsolidationConfig::conservative();
        assert!(config.working_to_episodic_importance > 0.5);
        assert!(!config.prune_low_importance);
    }

    #[test]
    fn test_consolidate_working_to_episodic() {
        let mut working = WorkingMemory::new(100);
        let mut episodic = EpisodicMemory::new(100);
        let mut semantic = SemanticStore::new(100, 128);

        // Add entry that meets promotion criteria
        let mut entry = create_working_entry("1", "Important memory").with_importance(0.8);
        entry.access_count = 5;
        working.store(entry).unwrap();

        // Add entry that doesn't meet criteria
        working
            .store(create_working_entry("2", "Not important").with_importance(0.1))
            .unwrap();

        // Debug: check what entries are in working memory
        println!("Working memory entries before consolidation:");
        for e in working.all() {
            println!(
                "  Entry {}: importance={}, access_count={}",
                e.id, e.importance_score, e.access_count
            );
        }

        let config = ConsolidationConfig::aggressive();
        println!(
            "Config: importance_threshold={}, access_threshold={}",
            config.working_to_episodic_importance, config.working_to_episodic_access
        );

        let consolidator = MemoryConsolidator::with_config(10, config);
        let report = consolidator
            .consolidate(&mut working, &mut episodic, &mut semantic)
            .unwrap();

        println!(
            "Report: working_to_episodic={}, episodic_len={}",
            report.working_to_episodic,
            episodic.len()
        );

        assert_eq!(report.working_to_episodic, 1);
        assert_eq!(episodic.len(), 1);
    }

    #[test]
    fn test_consolidate_prunes_low_importance() {
        let mut working = WorkingMemory::new(100);
        let mut episodic = EpisodicMemory::new(100);
        let mut semantic = SemanticStore::new(100, 128);

        // Add low importance entry
        working
            .store(create_working_entry("1", "Low").with_importance(0.05))
            .unwrap();

        let consolidator = MemoryConsolidator::new(10);
        let report = consolidator
            .consolidate(&mut working, &mut episodic, &mut semantic)
            .unwrap();

        assert_eq!(report.removed, 1);
        assert_eq!(working.len(), 0);
    }

    #[test]
    fn test_merge_entries() {
        let entry1 = MemoryEntry::new(MemoryType::Semantic, "First content")
            .with_importance(0.8)
            .with_metadata(MemoryMetadata::default().tag("tag1"));

        let mut entry2 = MemoryEntry::new(MemoryType::Semantic, "Second content")
            .with_importance(0.6)
            .with_metadata(MemoryMetadata::default().tag("tag2"));
        entry2.access_count = 3;

        let merged = merge_entries(&entry1, &entry2);

        assert!(merged.content.contains("First content"));
        assert!(merged.content.contains("Second content"));
        assert!((merged.importance_score - 0.8).abs() < 0.01);
        assert_eq!(merged.access_count, 3);
        assert!(merged.metadata.tags.contains(&"tag1".to_string()));
        assert!(merged.metadata.tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_calculate_entry_similarity() {
        let entry1 = MemoryEntry::new(MemoryType::Semantic, "The quick brown fox");
        let entry2 = MemoryEntry::new(MemoryType::Semantic, "The quick brown dog");
        let entry3 = MemoryEntry::new(MemoryType::Semantic, "Something completely different");

        let sim1_2 = calculate_entry_similarity(&entry1, &entry2);
        let sim1_3 = calculate_entry_similarity(&entry1, &entry3);

        // entry1 and entry2 share more words
        assert!(sim1_2 > sim1_3);
        assert!(sim1_2 > 0.5);
    }

    #[test]
    fn test_set_config() {
        let mut consolidator = MemoryConsolidator::new(50);
        let new_config = ConsolidationConfig::aggressive();

        consolidator.set_config(new_config.clone());

        assert!(
            (consolidator.config().working_to_episodic_importance
                - new_config.working_to_episodic_importance)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn test_consolidator_with_custom_config() {
        let config = ConsolidationConfig {
            working_to_episodic_importance: 0.4,
            working_to_episodic_access: 1,
            ..Default::default()
        };

        let consolidator = MemoryConsolidator::with_config(25, config);
        assert_eq!(consolidator.threshold(), 25);
        assert!((consolidator.config().working_to_episodic_importance - 0.4).abs() < 0.01);
    }
}
