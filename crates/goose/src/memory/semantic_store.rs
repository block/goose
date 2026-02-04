//! Semantic Store Module
//!
//! Implements long-term semantic memory with vector-based similarity search.
//! This store is optimized for knowledge retrieval using cosine similarity.

use std::collections::HashMap;

use super::{MemoryEntry, MemoryError, MemoryResult, MemoryType, RecallContext};

/// Semantic memory store with vector search capabilities
#[derive(Debug)]
pub struct SemanticStore {
    /// Memory entries indexed by ID
    entries: HashMap<String, MemoryEntry>,
    /// Embedding vectors indexed by entry ID
    embeddings: HashMap<String, Vec<f32>>,
    /// Maximum capacity
    max_entries: usize,
    /// Embedding dimension
    embedding_dim: usize,
    /// Entry order for LRU-style eviction
    access_order: Vec<String>,
}

impl SemanticStore {
    /// Create a new semantic store
    pub fn new(max_entries: usize, embedding_dim: usize) -> Self {
        Self {
            entries: HashMap::new(),
            embeddings: HashMap::new(),
            max_entries,
            embedding_dim,
            access_order: Vec::new(),
        }
    }

    /// Store a memory entry with optional embedding
    pub fn store(&mut self, mut entry: MemoryEntry) -> MemoryResult<String> {
        let id = entry.id.clone();

        // Convert to semantic type if not already
        if entry.memory_type != MemoryType::Semantic && entry.memory_type != MemoryType::Procedural
        {
            entry.memory_type = MemoryType::Semantic;
            entry.decay_factor = MemoryType::Semantic.default_decay_factor();
        }

        // Check capacity
        while self.entries.len() >= self.max_entries && !self.entries.contains_key(&id) {
            self.evict_least_important()?;
        }

        // Extract or generate embedding
        let embedding = entry
            .embedding
            .take()
            .unwrap_or_else(|| self.generate_embedding(&entry.content));

        // Validate embedding dimension
        if embedding.len() != self.embedding_dim {
            return Err(MemoryError::embedding(format!(
                "Expected {} dimensions, got {}",
                self.embedding_dim,
                embedding.len()
            )));
        }

        // Update access order
        self.access_order.retain(|x| x != &id);

        // Store
        self.entries.insert(id.clone(), entry);
        self.embeddings.insert(id.clone(), embedding);
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

            // Update access order
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
        self.embeddings.remove(id);
        Ok(self.entries.remove(id).is_some())
    }

    /// Search for relevant memories using text matching and optionally vector similarity
    pub fn search(&self, query: &str, context: &RecallContext) -> MemoryResult<Vec<MemoryEntry>> {
        let query_embedding = self.generate_embedding(query);
        self.search_with_embedding(&query_embedding, query, context)
    }

    /// Search using a pre-computed embedding
    pub fn search_with_embedding(
        &self,
        query_embedding: &[f32],
        query_text: &str,
        context: &RecallContext,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        let query_lower = query_text.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<(f64, MemoryEntry)> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                // Type filter
                let type_ok = match entry.memory_type {
                    MemoryType::Semantic => context.include_semantic,
                    MemoryType::Procedural => context.include_procedural,
                    _ => false,
                };
                if !type_ok {
                    return false;
                }

                // Apply context filters
                if let Some(ref user_id) = context.user_id {
                    if entry.metadata.user_id.as_ref() != Some(user_id) {
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
            .filter_map(|(id, entry)| {
                // Get embedding for this entry
                let entry_embedding = self.embeddings.get(id)?;

                // Calculate cosine similarity
                let vector_sim = cosine_similarity(query_embedding, entry_embedding);

                // Calculate text similarity as fallback/boost
                let text_sim = self.calculate_text_similarity(&entry.content, &query_words);

                // Combine scores with weights
                let score = vector_sim * context.similarity_weight
                    + text_sim * 0.3  // Text matching boost
                    + entry.relevance_score() * context.importance_weight;

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

    /// Find k-nearest neighbors by embedding
    pub fn knn(&self, embedding: &[f32], k: usize) -> MemoryResult<Vec<(f64, MemoryEntry)>> {
        let mut results: Vec<(f64, MemoryEntry)> = self
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                let entry_embedding = self.embeddings.get(id)?;
                let similarity = cosine_similarity(embedding, entry_embedding);
                Some((similarity, entry.clone()))
            })
            .collect();

        // Sort by similarity descending
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results.into_iter().take(k).collect())
    }

    /// Get all entries of a specific type
    pub fn get_by_type(&self, memory_type: MemoryType) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.memory_type == memory_type)
            .collect()
    }

    /// Get entries matching tags
    pub fn get_by_tags(&self, tags: &[String]) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| tags.iter().any(|t| e.metadata.tags.contains(t)))
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
            self.delete(&id)?;
        }

        Ok(removed_count)
    }

    /// Clear all entries
    pub fn clear(&mut self) -> MemoryResult<()> {
        self.entries.clear();
        self.embeddings.clear();
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

    /// Get the embedding dimension
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    /// Get embedding for an entry
    pub fn get_embedding(&self, id: &str) -> Option<&Vec<f32>> {
        self.embeddings.get(id)
    }

    /// Update embedding for an entry
    pub fn update_embedding(&mut self, id: &str, embedding: Vec<f32>) -> MemoryResult<bool> {
        if embedding.len() != self.embedding_dim {
            return Err(MemoryError::embedding(format!(
                "Expected {} dimensions, got {}",
                self.embedding_dim,
                embedding.len()
            )));
        }

        if self.entries.contains_key(id) {
            self.embeddings.insert(id.to_string(), embedding);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Generate a simple embedding from text
    /// This is a basic implementation - production would use a proper model
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.embedding_dim];

        // Simple hash-based embedding for deterministic results
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            // Hash word to indices
            let hash = simple_hash(word);
            let idx1 = (hash % self.embedding_dim as u64) as usize;
            let idx2 = ((hash / 7) % self.embedding_dim as u64) as usize;
            let idx3 = ((hash / 13) % self.embedding_dim as u64) as usize;

            // Position weighting
            let position_weight = 1.0 / (1.0 + i as f32 * 0.1);

            // Word length factor
            let length_factor = (word.len() as f32).sqrt() / 3.0;

            embedding[idx1] += position_weight * length_factor;
            embedding[idx2] += position_weight * 0.5;
            embedding[idx3] -= position_weight * 0.3;
        }

        // Normalize to unit vector
        normalize(&mut embedding);

        embedding
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

    /// Evict least important entry
    fn evict_least_important(&mut self) -> MemoryResult<Option<MemoryEntry>> {
        // Find entry with lowest importance that was accessed longest ago
        let to_evict = self
            .entries
            .iter()
            .min_by(|a, b| {
                let score_a = a.1.relevance_score();
                let score_b = b.1.relevance_score();
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id.clone());

        if let Some(id) = to_evict {
            self.delete(&id)?;
            // Note: we can't return the entry since delete removes it
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (x, y) in a.iter().zip(b.iter()) {
        dot_product += (*x as f64) * (*y as f64);
        norm_a += (*x as f64) * (*x as f64);
        norm_b += (*y as f64) * (*y as f64);
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

/// Normalize a vector to unit length
fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Simple hash function for deterministic embedding generation
fn simple_hash(s: &str) -> u64 {
    let mut hash = 5381u64;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMetadata;

    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry::new(MemoryType::Semantic, content).with_id(id)
    }

    #[test]
    fn test_semantic_store_creation() {
        let store = SemanticStore::new(1000, 384);
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert_eq!(store.embedding_dim(), 384);
    }

    #[test]
    fn test_store_and_get() {
        let mut store = SemanticStore::new(100, 128);
        let entry = create_test_entry("test-1", "The user prefers dark mode");

        let id = store.store(entry).unwrap();
        assert_eq!(id, "test-1");
        assert_eq!(store.len(), 1);

        let retrieved = store.get("test-1").unwrap().unwrap();
        assert_eq!(retrieved.content, "The user prefers dark mode");
    }

    #[test]
    fn test_embedding_storage() {
        let mut store = SemanticStore::new(100, 128);
        let entry = create_test_entry("test-1", "Test content");

        store.store(entry).unwrap();

        let embedding = store.get_embedding("test-1").unwrap();
        assert_eq!(embedding.len(), 128);

        // Check it's normalized (approximately unit length)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_store_with_custom_embedding() {
        let mut store = SemanticStore::new(100, 4);

        let mut embedding = vec![1.0, 0.0, 0.0, 0.0];
        let entry = create_test_entry("test-1", "Test").with_embedding(embedding.clone());

        store.store(entry).unwrap();

        let stored_embedding = store.get_embedding("test-1").unwrap();
        assert_eq!(stored_embedding, &embedding);
    }

    #[test]
    fn test_invalid_embedding_dimension() {
        let mut store = SemanticStore::new(100, 128);
        let entry = create_test_entry("test-1", "Test").with_embedding(vec![1.0, 0.0]); // Wrong dimension

        let result = store.store(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_search() {
        let mut store = SemanticStore::new(100, 128);

        store
            .store(create_test_entry("1", "The user prefers dark mode"))
            .unwrap();
        store
            .store(create_test_entry("2", "Python is a programming language"))
            .unwrap();
        store
            .store(create_test_entry("3", "Dark theme for editors"))
            .unwrap();

        let context = RecallContext::default();
        let results = store.search("dark mode preference", &context).unwrap();

        // Should find entries mentioning "dark"
        assert!(!results.is_empty());
    }

    #[test]
    fn test_knn() {
        let mut store = SemanticStore::new(100, 4);

        // Store with known embeddings
        let entry1 = create_test_entry("1", "Similar A").with_embedding(vec![1.0, 0.0, 0.0, 0.0]);
        let entry2 = create_test_entry("2", "Different").with_embedding(vec![0.0, 1.0, 0.0, 0.0]);
        let entry3 = create_test_entry("3", "Similar B").with_embedding(vec![0.9, 0.1, 0.0, 0.0]);

        store.store(entry1).unwrap();
        store.store(entry2).unwrap();
        store.store(entry3).unwrap();

        // Query similar to entry1 and entry3
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = store.knn(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        // Entry 1 should be first (exact match)
        assert_eq!(results[0].1.id, "1");
        assert!((results[0].0 - 1.0).abs() < 0.01); // Cosine similarity ~1.0
    }

    #[test]
    fn test_delete() {
        let mut store = SemanticStore::new(100, 128);
        store.store(create_test_entry("test-1", "content")).unwrap();

        assert!(store.delete("test-1").unwrap());
        assert!(store.get("test-1").unwrap().is_none());
        assert!(store.get_embedding("test-1").is_none());
    }

    #[test]
    fn test_capacity_eviction() {
        let mut store = SemanticStore::new(3, 64);

        store.store(create_test_entry("1", "first")).unwrap();
        store.store(create_test_entry("2", "second")).unwrap();
        store.store(create_test_entry("3", "third")).unwrap();
        store.store(create_test_entry("4", "fourth")).unwrap();

        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_get_by_type() {
        let mut store = SemanticStore::new(100, 128);

        let semantic = MemoryEntry::new(MemoryType::Semantic, "knowledge").with_id("1");
        let procedural = MemoryEntry::new(MemoryType::Procedural, "skill").with_id("2");

        store.store(semantic).unwrap();
        store.store(procedural).unwrap();

        let semantic_entries = store.get_by_type(MemoryType::Semantic);
        assert_eq!(semantic_entries.len(), 1);
        assert_eq!(semantic_entries[0].id, "1");

        let procedural_entries = store.get_by_type(MemoryType::Procedural);
        assert_eq!(procedural_entries.len(), 1);
        assert_eq!(procedural_entries[0].id, "2");
    }

    #[test]
    fn test_get_by_tags() {
        let mut store = SemanticStore::new(100, 128);

        let entry1 = MemoryEntry::new(MemoryType::Semantic, "tagged content")
            .with_id("1")
            .with_metadata(MemoryMetadata::default().tag("important").tag("ai"));

        let entry2 = MemoryEntry::new(MemoryType::Semantic, "untagged").with_id("2");

        store.store(entry1).unwrap();
        store.store(entry2).unwrap();

        let tagged = store.get_by_tags(&["important".to_string()]);
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].id, "1");
    }

    #[test]
    fn test_apply_decay() {
        let mut store = SemanticStore::new(100, 128);

        let entry1 = create_test_entry("1", "high importance").with_importance(0.9);
        let entry2 = create_test_entry("2", "low importance").with_importance(0.05);

        store.store(entry1).unwrap();
        store.store(entry2).unwrap();

        let removed = store.apply_decay(24.0, 0.1).unwrap();
        assert!(removed >= 1);
        assert!(store.get("1").unwrap().is_some());
    }

    #[test]
    fn test_update_embedding() {
        let mut store = SemanticStore::new(100, 4);
        store.store(create_test_entry("1", "test")).unwrap();

        let new_embedding = vec![0.5, 0.5, 0.5, 0.5];
        assert!(store.update_embedding("1", new_embedding.clone()).unwrap());

        let stored = store.get_embedding("1").unwrap();
        assert_eq!(stored, &new_embedding);
    }

    #[test]
    fn test_update_embedding_invalid_dimension() {
        let mut store = SemanticStore::new(100, 4);
        store.store(create_test_entry("1", "test")).unwrap();

        let result = store.update_embedding("1", vec![1.0, 0.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear() {
        let mut store = SemanticStore::new(100, 128);
        store.store(create_test_entry("1", "test")).unwrap();
        store.store(create_test_entry("2", "test")).unwrap();

        store.clear().unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        // Identical vectors
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 0.001);

        // Orthogonal vectors
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 0.001);

        // Opposite vectors
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 0.001);

        // Similar vectors
        let sim = cosine_similarity(&[1.0, 0.1], &[0.9, 0.2]);
        assert!(sim > 0.9);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_generate_embedding_deterministic() {
        let store = SemanticStore::new(100, 128);

        let emb1 = store.generate_embedding("test content");
        let emb2 = store.generate_embedding("test content");

        assert_eq!(emb1, emb2);
    }

    #[test]
    fn test_search_with_filters() {
        let mut store = SemanticStore::new(100, 128);

        let entry1 = MemoryEntry::new(MemoryType::Semantic, "User data")
            .with_id("1")
            .with_metadata(MemoryMetadata::default().user("user-1"));

        let entry2 = MemoryEntry::new(MemoryType::Semantic, "Other user data")
            .with_id("2")
            .with_metadata(MemoryMetadata::default().user("user-2"));

        store.store(entry1).unwrap();
        store.store(entry2).unwrap();

        let context = RecallContext::default().for_user("user-1");
        let results = store.search("data", &context).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }
}
