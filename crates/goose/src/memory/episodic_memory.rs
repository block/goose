//! Episodic Memory Module
//!
//! Implements session-based memory storage for conversation history and events.
//! Episodic memory has medium-term retention with moderate decay.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

use super::{MemoryEntry, MemoryResult, MemoryType, RecallContext};

/// Session information for grouping memories
#[derive(Debug, Clone)]
pub struct Session {
    /// Session identifier
    pub id: String,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session was last active
    pub last_active: DateTime<Utc>,
    /// Number of entries in this session
    pub entry_count: usize,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            started_at: now,
            last_active: now,
            entry_count: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn touch(&mut self) {
        self.last_active = Utc::now();
    }

    pub fn increment(&mut self) {
        self.entry_count += 1;
        self.touch();
    }

    pub fn decrement(&mut self) {
        if self.entry_count > 0 {
            self.entry_count -= 1;
        }
    }

    /// Check if session is stale (no activity for given duration)
    pub fn is_stale(&self, max_idle: Duration) -> bool {
        Utc::now() - self.last_active > max_idle
    }
}

/// Episodic memory store for conversation and event history
#[derive(Debug)]
pub struct EpisodicMemory {
    /// Memory entries indexed by ID
    entries: HashMap<String, MemoryEntry>,
    /// Entries grouped by session
    sessions: HashMap<String, Session>,
    /// Session to entry mapping
    session_entries: HashMap<String, Vec<String>>,
    /// Maximum entries per session
    max_per_session: usize,
    /// Maximum total entries
    max_total: usize,
    /// Session idle timeout for cleanup (hours)
    session_idle_hours: i64,
}

impl EpisodicMemory {
    /// Create a new episodic memory store
    pub fn new(max_per_session: usize) -> Self {
        Self {
            entries: HashMap::new(),
            sessions: HashMap::new(),
            session_entries: HashMap::new(),
            max_per_session,
            max_total: max_per_session * 100, // Allow up to 100 sessions
            session_idle_hours: 24 * 7,       // 7 days default
        }
    }

    /// Create with custom total limit
    pub fn with_max_total(mut self, max: usize) -> Self {
        self.max_total = max;
        self
    }

    /// Create with custom idle timeout
    pub fn with_idle_timeout(mut self, hours: i64) -> Self {
        self.session_idle_hours = hours;
        self
    }

    /// Store a memory entry
    pub fn store(&mut self, mut entry: MemoryEntry) -> MemoryResult<String> {
        let id = entry.id.clone();

        // Convert to episodic type if needed
        if entry.memory_type != MemoryType::Episodic {
            entry.memory_type = MemoryType::Episodic;
            entry.decay_factor = MemoryType::Episodic.default_decay_factor();
        }

        // Get or create session
        let session_id = entry
            .metadata
            .session_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // Check total capacity
        if self.entries.len() >= self.max_total && !self.entries.contains_key(&id) {
            self.evict_oldest_session()?;
        }

        // Get or create session tracking
        let session = self
            .sessions
            .entry(session_id.clone())
            .or_insert_with(|| Session::new(&session_id));

        let session_entry_list = self
            .session_entries
            .entry(session_id.clone())
            .or_default();

        // Check per-session capacity
        if session_entry_list.len() >= self.max_per_session && !self.entries.contains_key(&id) {
            // Evict oldest in session
            if let Some(oldest_id) = session_entry_list.first().cloned() {
                self.entries.remove(&oldest_id);
                session_entry_list.remove(0);
                session.decrement();
            }
        }

        // Update session metadata
        entry.metadata.session_id = Some(session_id.clone());

        // Remove from session list if updating existing entry
        if self.entries.contains_key(&id) {
            session_entry_list.retain(|x| x != &id);
        } else {
            session.increment();
        }

        // Store entry
        self.entries.insert(id.clone(), entry);
        session_entry_list.push(id.clone());
        session.touch();

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

            // Update session activity
            if let Some(session_id) = &entry.metadata.session_id {
                if let Some(session) = self.sessions.get_mut(session_id) {
                    session.touch();
                }
            }

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Delete a memory entry
    pub fn delete(&mut self, id: &str) -> MemoryResult<bool> {
        if let Some(entry) = self.entries.remove(id) {
            // Update session tracking
            if let Some(session_id) = &entry.metadata.session_id {
                if let Some(entries) = self.session_entries.get_mut(session_id) {
                    entries.retain(|x| x != id);
                }
                if let Some(session) = self.sessions.get_mut(session_id) {
                    session.decrement();
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Search for relevant memories
    pub fn search(&self, query: &str, context: &RecallContext) -> MemoryResult<Vec<MemoryEntry>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<(f64, MemoryEntry)> = self
            .entries
            .values()
            .filter(|entry| {
                // Apply context filters
                if let Some(ref user_id) = context.user_id {
                    if entry.metadata.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }
                if let Some(ref session_id) = context.session_id {
                    if entry.metadata.session_id.as_ref() != Some(session_id) {
                        return false;
                    }
                }
                if let Some(ref project_id) = context.project_id {
                    if entry.metadata.project_id.as_ref() != Some(project_id) {
                        return false;
                    }
                }
                if !context.tags.is_empty() {
                    let has_tag = context.tags.iter().any(|t| entry.metadata.tags.contains(t));
                    if !has_tag {
                        return false;
                    }
                }
                true
            })
            .filter_map(|entry| {
                let text_score = self.calculate_text_similarity(&entry.content, &query_words);
                let relevance = entry.relevance_score();

                // Weighted score
                let score =
                    text_score * context.similarity_weight + relevance * context.importance_weight;

                if score > context.min_relevance {
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
            .take(context.max_results)
            .map(|(_, e)| e)
            .collect())
    }

    /// Get entries for a specific session
    pub fn get_session(&self, session_id: &str) -> MemoryResult<Vec<MemoryEntry>> {
        if let Some(entry_ids) = self.session_entries.get(session_id) {
            let entries: Vec<MemoryEntry> = entry_ids
                .iter()
                .filter_map(|id| self.entries.get(id).cloned())
                .collect();
            Ok(entries)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get recent entries from a session
    pub fn get_session_recent(
        &self,
        session_id: &str,
        limit: usize,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        if let Some(entry_ids) = self.session_entries.get(session_id) {
            let entries: Vec<MemoryEntry> = entry_ids
                .iter()
                .rev()
                .take(limit)
                .filter_map(|id| self.entries.get(id).cloned())
                .collect();
            Ok(entries)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all session info
    pub fn get_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// Get session by ID
    pub fn get_session_info(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
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
            self.delete(&id)?;
        }

        // Also clean up stale sessions
        self.cleanup_stale_sessions()?;

        Ok(removed_count)
    }

    /// Clean up sessions with no activity
    fn cleanup_stale_sessions(&mut self) -> MemoryResult<usize> {
        let max_idle = Duration::hours(self.session_idle_hours);
        let stale_sessions: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_stale(max_idle) && s.entry_count == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let count = stale_sessions.len();
        for session_id in stale_sessions {
            self.sessions.remove(&session_id);
            self.session_entries.remove(&session_id);
        }

        Ok(count)
    }

    /// Clear all entries
    pub fn clear(&mut self) -> MemoryResult<()> {
        self.entries.clear();
        self.sessions.clear();
        self.session_entries.clear();
        Ok(())
    }

    /// Clear a specific session
    pub fn clear_session(&mut self, session_id: &str) -> MemoryResult<usize> {
        if let Some(entry_ids) = self.session_entries.remove(session_id) {
            let count = entry_ids.len();
            for id in entry_ids {
                self.entries.remove(&id);
            }
            self.sessions.remove(session_id);
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get number of sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get entries that should be promoted to semantic memory
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
            // Remove from entries first
            if let Some(entry) = self.entries.remove(&id) {
                // Update session tracking
                if let Some(session_id) = &entry.metadata.session_id {
                    if let Some(entries) = self.session_entries.get_mut(session_id) {
                        entries.retain(|x| x != &id);
                    }
                    if let Some(session) = self.sessions.get_mut(session_id) {
                        session.decrement();
                    }
                }
                promoted.push(entry);
            }
        }

        promoted
    }

    /// Evict oldest session when at capacity
    fn evict_oldest_session(&mut self) -> MemoryResult<()> {
        // Find session with oldest last_active
        let oldest = self
            .sessions
            .iter()
            .min_by_key(|(_, s)| s.last_active)
            .map(|(id, _)| id.clone());

        if let Some(session_id) = oldest {
            self.clear_session(&session_id)?;
        }

        Ok(())
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
            for cw in &content_words {
                if cw.contains(qw) || qw.contains(cw) {
                    matches += 1;
                    break;
                }
            }
        }

        matches as f64 / query_words.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMetadata;

    fn create_test_entry(id: &str, content: &str, session: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Episodic, content)
            .with_id(id)
            .with_metadata(MemoryMetadata::default().session(session))
    }

    #[test]
    fn test_episodic_memory_creation() {
        let em = EpisodicMemory::new(100);
        assert_eq!(em.len(), 0);
        assert!(em.is_empty());
        assert_eq!(em.session_count(), 0);
    }

    #[test]
    fn test_store_and_get() {
        let mut em = EpisodicMemory::new(100);
        let entry = create_test_entry("test-1", "Hello world", "session-1");

        let id = em.store(entry).unwrap();
        assert_eq!(id, "test-1");
        assert_eq!(em.len(), 1);
        assert_eq!(em.session_count(), 1);

        let retrieved = em.get("test-1").unwrap().unwrap();
        assert_eq!(retrieved.content, "Hello world");
    }

    #[test]
    fn test_session_tracking() {
        let mut em = EpisodicMemory::new(100);

        em.store(create_test_entry("1", "first", "session-A"))
            .unwrap();
        em.store(create_test_entry("2", "second", "session-A"))
            .unwrap();
        em.store(create_test_entry("3", "third", "session-B"))
            .unwrap();

        assert_eq!(em.session_count(), 2);

        let session_a = em.get_session("session-A").unwrap();
        assert_eq!(session_a.len(), 2);

        let session_b = em.get_session("session-B").unwrap();
        assert_eq!(session_b.len(), 1);
    }

    #[test]
    fn test_per_session_capacity() {
        let mut em = EpisodicMemory::new(3);

        em.store(create_test_entry("1", "first", "session-A"))
            .unwrap();
        em.store(create_test_entry("2", "second", "session-A"))
            .unwrap();
        em.store(create_test_entry("3", "third", "session-A"))
            .unwrap();
        em.store(create_test_entry("4", "fourth", "session-A"))
            .unwrap();

        assert_eq!(em.len(), 3);
        assert!(em.get("1").unwrap().is_none()); // Evicted
        assert!(em.get("4").unwrap().is_some());
    }

    #[test]
    fn test_delete() {
        let mut em = EpisodicMemory::new(100);
        em.store(create_test_entry("test-1", "content", "session-A"))
            .unwrap();

        assert!(em.delete("test-1").unwrap());
        assert!(em.get("test-1").unwrap().is_none());

        let session_info = em.get_session_info("session-A").unwrap();
        assert_eq!(session_info.entry_count, 0);
    }

    #[test]
    fn test_search_with_filters() {
        let mut em = EpisodicMemory::new(100);

        let entry1 = MemoryEntry::new(MemoryType::Episodic, "User prefers dark mode")
            .with_id("1")
            .with_metadata(
                MemoryMetadata::default()
                    .session("session-A")
                    .user("user-1"),
            );

        let entry2 = MemoryEntry::new(MemoryType::Episodic, "Dark theme settings")
            .with_id("2")
            .with_metadata(
                MemoryMetadata::default()
                    .session("session-A")
                    .user("user-2"),
            );

        em.store(entry1).unwrap();
        em.store(entry2).unwrap();

        // Search with user filter
        let context = RecallContext::default().for_user("user-1");
        let results = em.search("dark", &context).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_session_recent() {
        let mut em = EpisodicMemory::new(100);

        em.store(create_test_entry("1", "first", "session-A"))
            .unwrap();
        em.store(create_test_entry("2", "second", "session-A"))
            .unwrap();
        em.store(create_test_entry("3", "third", "session-A"))
            .unwrap();

        let recent = em.get_session_recent("session-A", 2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "3");
        assert_eq!(recent[1].id, "2");
    }

    #[test]
    fn test_clear_session() {
        let mut em = EpisodicMemory::new(100);

        em.store(create_test_entry("1", "first", "session-A"))
            .unwrap();
        em.store(create_test_entry("2", "second", "session-A"))
            .unwrap();
        em.store(create_test_entry("3", "third", "session-B"))
            .unwrap();

        let cleared = em.clear_session("session-A").unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(em.len(), 1);
        assert_eq!(em.session_count(), 1);
    }

    #[test]
    fn test_apply_decay() {
        let mut em = EpisodicMemory::new(100);

        let entry1 = create_test_entry("1", "high importance", "session-A").with_importance(0.9);
        let entry2 = create_test_entry("2", "low importance", "session-A").with_importance(0.05);

        em.store(entry1).unwrap();
        em.store(entry2).unwrap();

        let removed = em.apply_decay(24.0, 0.1).unwrap();
        assert!(removed >= 1);
        assert!(em.get("1").unwrap().is_some());
    }

    #[test]
    fn test_get_promotable() {
        let mut em = EpisodicMemory::new(100);

        let mut entry1 = create_test_entry("1", "promote me", "session-A").with_importance(0.8);
        entry1.access_count = 5;

        let entry2 = create_test_entry("2", "keep me", "session-A").with_importance(0.3);

        em.store(entry1).unwrap();
        em.store(entry2).unwrap();

        let promotable = em.get_promotable(0.5, 3);
        assert_eq!(promotable.len(), 1);
        assert_eq!(promotable[0].id, "1");
    }

    #[test]
    fn test_session_struct() {
        let mut session = Session::new("test-session");
        assert_eq!(session.id, "test-session");
        assert_eq!(session.entry_count, 0);

        session.increment();
        assert_eq!(session.entry_count, 1);

        session.decrement();
        assert_eq!(session.entry_count, 0);
    }

    #[test]
    fn test_session_staleness() {
        let mut session = Session::new("test");

        // Should not be stale immediately
        assert!(!session.is_stale(Duration::hours(1)));

        // Manually set last_active to past
        session.last_active = Utc::now() - Duration::hours(2);
        assert!(session.is_stale(Duration::hours(1)));
    }

    #[test]
    fn test_clear_all() {
        let mut em = EpisodicMemory::new(100);
        em.store(create_test_entry("1", "test", "session-A"))
            .unwrap();
        em.store(create_test_entry("2", "test", "session-B"))
            .unwrap();

        em.clear().unwrap();
        assert!(em.is_empty());
        assert_eq!(em.session_count(), 0);
    }

    #[test]
    fn test_update_existing_entry() {
        let mut em = EpisodicMemory::new(100);
        em.store(create_test_entry("1", "original", "session-A"))
            .unwrap();
        em.store(create_test_entry("1", "updated", "session-A"))
            .unwrap();

        assert_eq!(em.len(), 1);
        let entry = em.get("1").unwrap().unwrap();
        assert_eq!(entry.content, "updated");
    }

    #[test]
    fn test_auto_convert_to_episodic() {
        let mut em = EpisodicMemory::new(100);

        // Store a working memory entry (wrong type)
        let entry = MemoryEntry::new(MemoryType::Working, "test content")
            .with_id("1")
            .with_metadata(MemoryMetadata::default().session("session-A"));

        em.store(entry).unwrap();

        let retrieved = em.get("1").unwrap().unwrap();
        assert_eq!(retrieved.memory_type, MemoryType::Episodic);
    }
}
