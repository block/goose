//! Working Memory Module
//!
//! Implements short-term memory storage for current context and recent interactions.
//! Working memory has fast access times but limited capacity and fast decay.

use std::collections::HashMap;

use super::{MemoryEntry, MemoryError, MemoryResult, MemoryType};

/// Working memory store for short-term context
#[derive(Debug)]
pub struct WorkingMemory {
    /// Memory entries indexed by ID
    entries: HashMap<String, MemoryEntry>,
    /// Maximum capacity
    capacity: usize,
    /// Entry order for LRU eviction
    access_order: Vec<String>,
}

impl WorkingMemory {
    /// Create a new working memory store
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            access_order: Vec::new(),
        }
    }

    /// Store a memory entry
    pub fn store(&mut self, entry: MemoryEntry) -> MemoryResult<String> {
        let id = entry.id.clone();

        // Ensure it's a working memory type
        if entry.memory_type != MemoryType::Working {
            return Err(MemoryError::InvalidMemoryType(format!(
                "Expected Working, got {:?}",
                entry.memory_type
            )));
        }

        // Check capacity and evict if needed
        while self.entries.len() >= self.capacity {
            self.evict_oldest()?;
        }

        // Remove from access order if updating
        self.access_order.retain(|x| x != &id);

        // Store entry
        self.entries.insert(id.clone(), entry);
        self.access_order.push(id.clone());

        Ok(id)
    }

    /// Get a memory entry by ID
    pub fn get(&self, id: &str) -> MemoryResult<Option<MemoryEntry>> {
        Ok(self.entries.get(id).cloned())
    }

    /// Get a memory entry by ID and record access
    pub fn get_mut(&mut self, id: &str) -> MemoryResult<Option<&mut MemoryEntry>> {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.record_access();

            // Move to end of access order
            self.access_order.retain(|x| x != id);
            self.access_order.push(id.to_string());

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Delete a memory entry
    pub fn delete(&mut self, id: &str) -> MemoryResult<bool> {
        self.access_order.retain(|x| x != id);
        Ok(self.entries.remove(id).is_some())
    }

    /// Search for relevant memories
    pub fn search(&self, query: &str, max_results: usize) -> MemoryResult<Vec<MemoryEntry>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<(f64, MemoryEntry)> = self
            .entries
            .values()
            .filter_map(|entry| {
                let score = self.calculate_text_similarity(&entry.content, &query_words);
                if score > 0.0 {
                    Some((score, entry.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results
            .into_iter()
            .take(max_results)
            .map(|(_, e)| e)
            .collect())
    }

    /// Calculate text similarity between content and query words
    fn calculate_text_similarity(&self, content: &str, query_words: &[&str]) -> f64 {
        let content_lower = content.to_lowercase();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        if query_words.is_empty() || content_words.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for qw in query_words {
            let qw_lower = qw.to_lowercase();
            for cw in &content_words {
                // Exact match or word contains the query (but query must be substantial part)
                if *cw == qw_lower || (cw.contains(&qw_lower) && qw_lower.len() >= 3) {
                    matches += 1;
                    break;
                }
            }
        }

        matches as f64 / query_words.len() as f64
    }

    /// Get all entries
    pub fn all(&self) -> Vec<&MemoryEntry> {
        self.entries.values().collect()
    }

    /// Get all entries sorted by access time (most recent first)
    pub fn recent(&self, limit: usize) -> Vec<&MemoryEntry> {
        self.access_order
            .iter()
            .rev()
            .take(limit)
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Apply decay to all entries and remove those below threshold
    pub fn apply_decay(&mut self, hours: f64, threshold: f64) -> MemoryResult<usize> {
        let mut to_remove = Vec::new();

        for (id, entry) in self.entries.iter_mut() {
            entry.apply_decay(hours);
            if entry.importance_score < threshold {
                to_remove.push(id.clone());
            }
        }

        let removed_count = to_remove.len();
        for id in to_remove {
            self.entries.remove(&id);
            self.access_order.retain(|x| x != &id);
        }

        Ok(removed_count)
    }

    /// Clear all entries
    pub fn clear(&mut self) -> MemoryResult<()> {
        self.entries.clear();
        self.access_order.clear();
        Ok(())
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Evict oldest entry
    fn evict_oldest(&mut self) -> MemoryResult<Option<MemoryEntry>> {
        if let Some(oldest_id) = self.access_order.first().cloned() {
            self.access_order.remove(0);
            Ok(self.entries.remove(&oldest_id))
        } else {
            Ok(None)
        }
    }

    /// Get entries that should be promoted to episodic memory
    pub fn get_promotable(&self, min_importance: f64, min_access_count: u64) -> Vec<MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.importance_score >= min_importance && e.access_count >= min_access_count)
            .cloned()
            .collect()
    }

    /// Drain entries that meet promotion criteria
    pub fn drain_promotable(
        &mut self,
        min_importance: f64,
        min_access_count: u64,
    ) -> Vec<MemoryEntry> {
        let promotable_ids: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| {
                e.importance_score >= min_importance && e.access_count >= min_access_count
            })
            .map(|(id, _)| id.clone())
            .collect();

        let mut promoted = Vec::new();
        for id in promotable_ids {
            if let Some(entry) = self.entries.remove(&id) {
                self.access_order.retain(|x| x != &id);
                promoted.push(entry);
            }
        }

        promoted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Working, content).with_id(id)
    }

    #[test]
    fn test_working_memory_creation() {
        let wm = WorkingMemory::new(100);
        assert_eq!(wm.capacity(), 100);
        assert_eq!(wm.len(), 0);
        assert!(wm.is_empty());
    }

    #[test]
    fn test_store_and_get() {
        let mut wm = WorkingMemory::new(10);
        let entry = create_test_entry("test-1", "Hello world");

        let id = wm.store(entry).unwrap();
        assert_eq!(id, "test-1");
        assert_eq!(wm.len(), 1);

        let retrieved = wm.get("test-1").unwrap().unwrap();
        assert_eq!(retrieved.content, "Hello world");
    }

    #[test]
    fn test_store_wrong_type() {
        let mut wm = WorkingMemory::new(10);
        let entry = MemoryEntry::new(MemoryType::Semantic, "test");

        let result = wm.store(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_capacity_eviction() {
        let mut wm = WorkingMemory::new(3);

        wm.store(create_test_entry("1", "first")).unwrap();
        wm.store(create_test_entry("2", "second")).unwrap();
        wm.store(create_test_entry("3", "third")).unwrap();

        assert_eq!(wm.len(), 3);

        // This should evict "1"
        wm.store(create_test_entry("4", "fourth")).unwrap();

        assert_eq!(wm.len(), 3);
        assert!(wm.get("1").unwrap().is_none());
        assert!(wm.get("4").unwrap().is_some());
    }

    #[test]
    fn test_delete() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("test-1", "content")).unwrap();

        assert!(wm.delete("test-1").unwrap());
        assert!(wm.get("test-1").unwrap().is_none());
        assert!(!wm.delete("test-1").unwrap()); // Already deleted
    }

    #[test]
    fn test_search() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "The user prefers dark mode"))
            .unwrap();
        wm.store(create_test_entry("2", "Python is a programming language"))
            .unwrap();
        wm.store(create_test_entry("3", "The dark knight rises"))
            .unwrap();

        let results = wm.search("dark", 10).unwrap();
        assert_eq!(results.len(), 2);

        let ids: Vec<&str> = results.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"1"));
        assert!(ids.contains(&"3"));
    }

    #[test]
    fn test_recent() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "first")).unwrap();
        wm.store(create_test_entry("2", "second")).unwrap();
        wm.store(create_test_entry("3", "third")).unwrap();

        let recent = wm.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "3");
        assert_eq!(recent[1].id, "2");
    }

    #[test]
    fn test_access_updates_order() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "first")).unwrap();
        wm.store(create_test_entry("2", "second")).unwrap();
        wm.store(create_test_entry("3", "third")).unwrap();

        // Access "1" to move it to most recent
        wm.get_mut("1").unwrap();

        let recent = wm.recent(3);
        assert_eq!(recent[0].id, "1"); // Now most recent
    }

    #[test]
    fn test_apply_decay() {
        let mut wm = WorkingMemory::new(10);

        let entry1 = create_test_entry("1", "high importance").with_importance(0.9);
        let entry2 = create_test_entry("2", "low importance").with_importance(0.05);

        wm.store(entry1).unwrap();
        wm.store(entry2).unwrap();

        // Apply decay with threshold 0.1
        let removed = wm.apply_decay(24.0, 0.1).unwrap();

        // Entry 2 should be removed (below threshold)
        assert!(removed >= 1);
        assert!(wm.get("1").unwrap().is_some());
    }

    #[test]
    fn test_clear() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "test")).unwrap();
        wm.store(create_test_entry("2", "test")).unwrap();

        wm.clear().unwrap();
        assert!(wm.is_empty());
    }

    #[test]
    fn test_get_promotable() {
        let mut wm = WorkingMemory::new(10);

        let mut entry1 = create_test_entry("1", "important").with_importance(0.8);
        entry1.access_count = 5;

        let entry2 = create_test_entry("2", "not important").with_importance(0.3);

        wm.store(entry1).unwrap();
        wm.store(entry2).unwrap();

        let promotable = wm.get_promotable(0.5, 3);
        assert_eq!(promotable.len(), 1);
        assert_eq!(promotable[0].id, "1");
    }

    #[test]
    fn test_drain_promotable() {
        let mut wm = WorkingMemory::new(10);

        let mut entry1 = create_test_entry("1", "promote me").with_importance(0.8);
        entry1.access_count = 5;

        let entry2 = create_test_entry("2", "keep me").with_importance(0.3);

        wm.store(entry1).unwrap();
        wm.store(entry2).unwrap();

        let promoted = wm.drain_promotable(0.5, 3);

        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0].id, "1");
        assert_eq!(wm.len(), 1);
        assert!(wm.get("2").unwrap().is_some());
    }

    #[test]
    fn test_all_entries() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "first")).unwrap();
        wm.store(create_test_entry("2", "second")).unwrap();

        let all = wm.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_update_existing() {
        let mut wm = WorkingMemory::new(10);
        wm.store(create_test_entry("1", "original")).unwrap();
        wm.store(create_test_entry("1", "updated")).unwrap();

        assert_eq!(wm.len(), 1);
        let entry = wm.get("1").unwrap().unwrap();
        assert_eq!(entry.content, "updated");
    }

    #[test]
    fn test_text_similarity() {
        let wm = WorkingMemory::new(10);

        let words = vec!["dark", "mode"];
        let score1 = wm.calculate_text_similarity("The user prefers dark mode", &words);
        let score2 = wm.calculate_text_similarity("Python programming", &words);

        assert!(score1 > score2);
        assert!(score1 > 0.5);
        assert!(score2 < 0.1);
    }
}
