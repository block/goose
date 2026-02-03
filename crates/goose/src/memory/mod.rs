//! Memory System Module
//!
//! This module implements a sophisticated memory system for the Goose Enterprise Platform,
//! providing long-term context retention, semantic search, and intelligent recall across
//! sessions and users. Inspired by Mem0 architecture.
//!
//! # Architecture
//!
//! The memory system consists of several specialized memory subsystems:
//!
//! - **Working Memory**: Short-term context for current interactions
//! - **Episodic Memory**: Session and conversation history
//! - **Semantic Memory**: Long-term facts and knowledge
//! - **Procedural Memory**: Learned procedures and patterns
//!
//! # Example
//!
//! ```rust,ignore
//! use goose::memory::{MemoryManager, MemoryConfig, MemoryEntry, MemoryType};
//!
//! // Create memory manager
//! let config = MemoryConfig::default();
//! let manager = MemoryManager::new(config)?;
//!
//! // Store a memory
//! let entry = MemoryEntry::new(
//!     MemoryType::Semantic,
//!     "The user prefers dark mode themes",
//! );
//! manager.store(entry).await?;
//!
//! // Recall relevant memories
//! let context = RecallContext::default();
//! let memories = manager.recall("user preferences", &context).await?;
//! ```

pub mod errors;
pub mod working_memory;
pub mod episodic_memory;
pub mod semantic_store;
pub mod consolidation;
pub mod retrieval;

pub use errors::{MemoryError, MemoryResult};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Re-exports
pub use working_memory::WorkingMemory;
pub use episodic_memory::EpisodicMemory;
pub use semantic_store::SemanticStore;
pub use consolidation::MemoryConsolidator;
pub use retrieval::MemoryRetriever;

/// Memory types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Facts and knowledge (long-term, slow decay)
    Semantic,
    /// Events and conversations (medium-term)
    Episodic,
    /// Skills and procedures (long-term)
    Procedural,
    /// Current context (short-term, fast access)
    Working,
}

impl MemoryType {
    /// Get the default decay factor for this memory type
    pub fn default_decay_factor(&self) -> f64 {
        match self {
            Self::Semantic => 0.99,    // Very slow decay
            Self::Procedural => 0.98,  // Slow decay
            Self::Episodic => 0.90,    // Moderate decay
            Self::Working => 0.70,     // Fast decay
        }
    }

    /// Get the display name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Episodic => "episodic",
            Self::Procedural => "procedural",
            Self::Working => "working",
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Source of the memory entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySource {
    /// User input/message
    UserInput,
    /// Agent response
    AgentResponse,
    /// Tool execution result
    ToolResult,
    /// Observation from environment
    Observation,
    /// Inferred from other memories
    Inference,
    /// External source (API, file, etc.)
    External,
    /// System-generated
    System,
}

impl MemorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserInput => "user_input",
            Self::AgentResponse => "agent_response",
            Self::ToolResult => "tool_result",
            Self::Observation => "observation",
            Self::Inference => "inference",
            Self::External => "external",
            Self::System => "system",
        }
    }
}

/// Type of relationship between memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// Generic relationship
    RelatedTo,
    /// Derived from another memory
    DerivedFrom,
    /// Contradicts another memory
    Contradicts,
    /// Supports another memory
    Supports,
    /// Part of a larger concept
    PartOf,
    /// Follows temporally
    FollowedBy,
    /// Caused by another event
    CausedBy,
    /// Similar to another memory
    SimilarTo,
}

/// A relationship to another memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelation {
    /// Target memory ID
    pub target_id: String,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Strength of the relationship (0.0 - 1.0)
    pub strength: f64,
}

impl MemoryRelation {
    /// Create a new memory relation
    pub fn new(target_id: impl Into<String>, relation_type: RelationType, strength: f64) -> Self {
        Self {
            target_id: target_id.into(),
            relation_type,
            strength: strength.clamp(0.0, 1.0),
        }
    }
}

/// Metadata associated with a memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// User who created/owns this memory
    pub user_id: Option<String>,
    /// Session in which this memory was created
    pub session_id: Option<String>,
    /// Project this memory belongs to
    pub project_id: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Source of the memory
    pub source: MemorySource,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Relationships to other memories
    pub relationships: Vec<MemoryRelation>,
    /// Additional custom data
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        Self {
            user_id: None,
            session_id: None,
            project_id: None,
            tags: Vec::new(),
            source: MemorySource::System,
            confidence: 1.0,
            relationships: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

impl MemoryMetadata {
    /// Create new metadata with a source
    pub fn with_source(source: MemorySource) -> Self {
        Self {
            source,
            ..Default::default()
        }
    }

    /// Set user ID
    pub fn user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session ID
    pub fn session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set project ID
    pub fn project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set confidence
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add a relationship
    pub fn relationship(mut self, relation: MemoryRelation) -> Self {
        self.relationships.push(relation);
        self
    }

    /// Add custom data
    pub fn custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// A memory entry with content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: String,
    /// Type of memory
    pub memory_type: MemoryType,
    /// Content of the memory
    pub content: String,
    /// Vector embedding (optional, generated if not provided)
    pub embedding: Option<Vec<f32>>,
    /// Associated metadata
    pub metadata: MemoryMetadata,
    /// When this memory was created
    pub created_at: DateTime<Utc>,
    /// When this memory was last accessed
    pub accessed_at: DateTime<Utc>,
    /// Number of times accessed
    pub access_count: u64,
    /// Importance score (0.0 - 1.0)
    pub importance_score: f64,
    /// Decay factor (determines how fast importance decreases)
    pub decay_factor: f64,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(memory_type: MemoryType, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            memory_type,
            content: content.into(),
            embedding: None,
            metadata: MemoryMetadata::default(),
            created_at: now,
            accessed_at: now,
            access_count: 0,
            importance_score: 0.5,
            decay_factor: memory_type.default_decay_factor(),
        }
    }

    /// Create with specific ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: MemoryMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set importance score
    pub fn with_importance(mut self, score: f64) -> Self {
        self.importance_score = score.clamp(0.0, 1.0);
        self
    }

    /// Set decay factor
    pub fn with_decay(mut self, decay: f64) -> Self {
        self.decay_factor = decay.clamp(0.0, 1.0);
        self
    }

    /// Record an access to this memory
    pub fn record_access(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count += 1;
        // Increase importance when accessed
        self.importance_score = (self.importance_score + 0.1).min(1.0);
    }

    /// Apply decay to importance score based on time
    pub fn apply_decay(&mut self, hours_elapsed: f64) {
        let decay = self.decay_factor.powf(hours_elapsed / 24.0);
        self.importance_score *= decay;
    }

    /// Calculate current relevance score
    pub fn relevance_score(&self) -> f64 {
        let hours_since_access = (Utc::now() - self.accessed_at).num_hours() as f64;
        let recency_factor = (-0.01 * hours_since_access).exp();
        let access_factor = (self.access_count as f64).ln_1p() / 10.0;

        (self.importance_score * 0.4 + recency_factor * 0.4 + access_factor * 0.2).min(1.0)
    }
}

/// Configuration for the memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable the memory system
    pub enabled: bool,
    /// Maximum entries in working memory
    pub max_working_memory: usize,
    /// Maximum entries in episodic memory per session
    pub max_episodic_per_session: usize,
    /// Maximum total semantic memories
    pub max_semantic_memories: usize,
    /// Auto-consolidate after N working memory entries
    pub consolidation_threshold: usize,
    /// Default embedding dimension
    pub embedding_dimension: usize,
    /// Enable automatic decay
    pub auto_decay: bool,
    /// Decay interval in hours
    pub decay_interval_hours: u64,
    /// Minimum importance to retain
    pub min_importance_threshold: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_working_memory: 100,
            max_episodic_per_session: 1000,
            max_semantic_memories: 100_000,
            consolidation_threshold: 50,
            embedding_dimension: 384, // Common for small models
            auto_decay: true,
            decay_interval_hours: 24,
            min_importance_threshold: 0.1,
        }
    }
}

impl MemoryConfig {
    /// Create a minimal configuration for testing
    pub fn minimal() -> Self {
        Self {
            max_working_memory: 10,
            max_episodic_per_session: 100,
            max_semantic_memories: 1000,
            consolidation_threshold: 5,
            ..Default::default()
        }
    }

    /// Create a high-capacity configuration
    pub fn high_capacity() -> Self {
        Self {
            max_working_memory: 500,
            max_episodic_per_session: 10_000,
            max_semantic_memories: 1_000_000,
            consolidation_threshold: 100,
            ..Default::default()
        }
    }
}

/// Context for memory recall operations
#[derive(Debug, Clone)]
pub struct RecallContext {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Filter by project ID
    pub project_id: Option<String>,
    /// Filter by tags (any match)
    pub tags: Vec<String>,
    /// Include semantic memories
    pub include_semantic: bool,
    /// Include episodic memories
    pub include_episodic: bool,
    /// Include procedural memories
    pub include_procedural: bool,
    /// Include working memory
    pub include_working: bool,
    /// Maximum results to return
    pub max_results: usize,
    /// Minimum relevance score (0.0 - 1.0)
    pub min_relevance: f64,
    /// Weight for semantic similarity
    pub similarity_weight: f64,
    /// Weight for recency
    pub recency_weight: f64,
    /// Weight for importance
    pub importance_weight: f64,
    /// Weight for access frequency
    pub access_weight: f64,
}

impl Default for RecallContext {
    fn default() -> Self {
        Self {
            user_id: None,
            session_id: None,
            project_id: None,
            tags: Vec::new(),
            include_semantic: true,
            include_episodic: true,
            include_procedural: true,
            include_working: true,
            max_results: 10,
            min_relevance: 0.0,
            similarity_weight: 0.4,
            recency_weight: 0.3,
            importance_weight: 0.2,
            access_weight: 0.1,
        }
    }
}

impl RecallContext {
    /// Create context for semantic search only
    pub fn semantic_only() -> Self {
        Self {
            include_semantic: true,
            include_episodic: false,
            include_procedural: false,
            include_working: false,
            ..Default::default()
        }
    }

    /// Create context for working memory only
    pub fn working_only() -> Self {
        Self {
            include_semantic: false,
            include_episodic: false,
            include_procedural: false,
            include_working: true,
            ..Default::default()
        }
    }

    /// Create context for current session
    pub fn current_session(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            include_working: true,
            include_episodic: true,
            include_semantic: false,
            include_procedural: false,
            ..Default::default()
        }
    }

    /// Set user filter
    pub fn for_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session filter
    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set max results
    pub fn limit(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Set minimum relevance threshold
    pub fn min_relevance(mut self, min: f64) -> Self {
        self.min_relevance = min.clamp(0.0, 1.0);
        self
    }
}

/// Memory manager that coordinates all memory subsystems
pub struct MemoryManager {
    /// Working memory store
    working: Arc<RwLock<WorkingMemory>>,
    /// Episodic memory store
    episodic: Arc<RwLock<EpisodicMemory>>,
    /// Semantic memory store
    semantic: Arc<RwLock<SemanticStore>>,
    /// Memory consolidator
    consolidator: Arc<MemoryConsolidator>,
    /// Memory retriever
    retriever: Arc<MemoryRetriever>,
    /// Configuration
    config: MemoryConfig,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> MemoryResult<Self> {
        let working = Arc::new(RwLock::new(WorkingMemory::new(config.max_working_memory)));
        let episodic = Arc::new(RwLock::new(EpisodicMemory::new(config.max_episodic_per_session)));
        let semantic = Arc::new(RwLock::new(SemanticStore::new(
            config.max_semantic_memories,
            config.embedding_dimension,
        )));
        let consolidator = Arc::new(MemoryConsolidator::new(config.consolidation_threshold));
        let retriever = Arc::new(MemoryRetriever::new());

        Ok(Self {
            working,
            episodic,
            semantic,
            consolidator,
            retriever,
            config,
        })
    }

    /// Store a new memory entry
    pub async fn store(&self, entry: MemoryEntry) -> MemoryResult<String> {
        let id = entry.id.clone();

        match entry.memory_type {
            MemoryType::Working => {
                let mut working = self.working.write().await;
                working.store(entry)?;

                // Check if consolidation is needed
                if working.len() >= self.config.consolidation_threshold {
                    drop(working); // Release lock before consolidation
                    self.consolidate().await?;
                }
            }
            MemoryType::Episodic => {
                let mut episodic = self.episodic.write().await;
                episodic.store(entry)?;
            }
            MemoryType::Semantic | MemoryType::Procedural => {
                let mut semantic = self.semantic.write().await;
                semantic.store(entry)?;
            }
        }

        Ok(id)
    }

    /// Recall memories relevant to a query
    pub async fn recall(
        &self,
        query: &str,
        context: &RecallContext,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        let mut results = Vec::new();

        // Collect from working memory
        if context.include_working {
            let working = self.working.read().await;
            let working_results = working.search(query, context.max_results)?;
            results.extend(working_results);
        }

        // Collect from episodic memory
        if context.include_episodic {
            let episodic = self.episodic.read().await;
            let episodic_results = episodic.search(query, context)?;
            results.extend(episodic_results);
        }

        // Collect from semantic memory
        if context.include_semantic || context.include_procedural {
            let semantic = self.semantic.read().await;
            let semantic_results = semantic.search(query, context)?;
            results.extend(semantic_results);
        }

        // Re-rank and filter results
        let results = self.retriever.rerank(results, query, context)?;

        Ok(results)
    }

    /// Get a specific memory by ID
    pub async fn get(&self, id: &str) -> MemoryResult<Option<MemoryEntry>> {
        // Check working memory
        {
            let working = self.working.read().await;
            if let Some(entry) = working.get(id)? {
                return Ok(Some(entry));
            }
        }

        // Check episodic memory
        {
            let episodic = self.episodic.read().await;
            if let Some(entry) = episodic.get(id)? {
                return Ok(Some(entry));
            }
        }

        // Check semantic memory
        {
            let semantic = self.semantic.read().await;
            if let Some(entry) = semantic.get(id)? {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }

    /// Delete a memory by ID
    pub async fn delete(&self, id: &str) -> MemoryResult<bool> {
        // Try working memory
        {
            let mut working = self.working.write().await;
            if working.delete(id)? {
                return Ok(true);
            }
        }

        // Try episodic memory
        {
            let mut episodic = self.episodic.write().await;
            if episodic.delete(id)? {
                return Ok(true);
            }
        }

        // Try semantic memory
        {
            let mut semantic = self.semantic.write().await;
            if semantic.delete(id)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Consolidate working memory to long-term storage
    pub async fn consolidate(&self) -> MemoryResult<ConsolidationReport> {
        let mut working = self.working.write().await;
        let mut episodic = self.episodic.write().await;
        let mut semantic = self.semantic.write().await;

        self.consolidator.consolidate(&mut working, &mut episodic, &mut semantic)
    }

    /// Apply decay to all memories
    pub async fn apply_decay(&self) -> MemoryResult<DecayReport> {
        let hours = self.config.decay_interval_hours as f64;
        let threshold = self.config.min_importance_threshold;

        let mut working_removed = 0;
        let mut episodic_removed = 0;
        let mut semantic_removed = 0;

        // Decay working memory
        {
            let mut working = self.working.write().await;
            working_removed = working.apply_decay(hours, threshold)?;
        }

        // Decay episodic memory
        {
            let mut episodic = self.episodic.write().await;
            episodic_removed = episodic.apply_decay(hours, threshold)?;
        }

        // Decay semantic memory (slower)
        {
            let mut semantic = self.semantic.write().await;
            semantic_removed = semantic.apply_decay(hours, threshold)?;
        }

        Ok(DecayReport {
            working_removed,
            episodic_removed,
            semantic_removed,
        })
    }

    /// Get statistics about memory usage
    pub async fn stats(&self) -> MemoryStats {
        let working = self.working.read().await;
        let episodic = self.episodic.read().await;
        let semantic = self.semantic.read().await;

        MemoryStats {
            working_count: working.len(),
            working_capacity: self.config.max_working_memory,
            episodic_count: episodic.len(),
            episodic_capacity: self.config.max_episodic_per_session,
            semantic_count: semantic.len(),
            semantic_capacity: self.config.max_semantic_memories,
        }
    }

    /// Clear all memories
    pub async fn clear(&self) -> MemoryResult<()> {
        self.working.write().await.clear()?;
        self.episodic.write().await.clear()?;
        self.semantic.write().await.clear()?;
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

/// Report from consolidation operation
#[derive(Debug, Clone)]
pub struct ConsolidationReport {
    /// Memories moved from working to episodic
    pub working_to_episodic: usize,
    /// Memories promoted to semantic
    pub promoted_to_semantic: usize,
    /// Memories merged
    pub merged: usize,
    /// Memories removed (below threshold)
    pub removed: usize,
}

/// Report from decay operation
#[derive(Debug, Clone)]
pub struct DecayReport {
    /// Working memories removed
    pub working_removed: usize,
    /// Episodic memories removed
    pub episodic_removed: usize,
    /// Semantic memories removed
    pub semantic_removed: usize,
}

impl DecayReport {
    /// Total memories removed
    pub fn total_removed(&self) -> usize {
        self.working_removed + self.episodic_removed + self.semantic_removed
    }
}

/// Memory system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Current working memory count
    pub working_count: usize,
    /// Working memory capacity
    pub working_capacity: usize,
    /// Current episodic memory count
    pub episodic_count: usize,
    /// Episodic memory capacity
    pub episodic_capacity: usize,
    /// Current semantic memory count
    pub semantic_count: usize,
    /// Semantic memory capacity
    pub semantic_capacity: usize,
}

impl MemoryStats {
    /// Total memories stored
    pub fn total_count(&self) -> usize {
        self.working_count + self.episodic_count + self.semantic_count
    }

    /// Working memory utilization (0.0 - 1.0)
    pub fn working_utilization(&self) -> f64 {
        if self.working_capacity == 0 {
            0.0
        } else {
            self.working_count as f64 / self.working_capacity as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_decay_factors() {
        assert!(MemoryType::Semantic.default_decay_factor() > MemoryType::Episodic.default_decay_factor());
        assert!(MemoryType::Episodic.default_decay_factor() > MemoryType::Working.default_decay_factor());
    }

    #[test]
    fn test_memory_type_display() {
        assert_eq!(MemoryType::Semantic.as_str(), "semantic");
        assert_eq!(MemoryType::Episodic.as_str(), "episodic");
        assert_eq!(MemoryType::Working.as_str(), "working");
        assert_eq!(MemoryType::Procedural.as_str(), "procedural");
    }

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(MemoryType::Semantic, "test content");
        assert!(!entry.id.is_empty());
        assert_eq!(entry.content, "test content");
        assert_eq!(entry.memory_type, MemoryType::Semantic);
        assert_eq!(entry.access_count, 0);
        assert!(entry.embedding.is_none());
    }

    #[test]
    fn test_memory_entry_builder() {
        let entry = MemoryEntry::new(MemoryType::Working, "test")
            .with_id("custom-id")
            .with_importance(0.8)
            .with_decay(0.95);

        assert_eq!(entry.id, "custom-id");
        assert!((entry.importance_score - 0.8).abs() < 0.001);
        assert!((entry.decay_factor - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_memory_entry_access() {
        let mut entry = MemoryEntry::new(MemoryType::Working, "test");
        let initial_importance = entry.importance_score;

        entry.record_access();

        assert_eq!(entry.access_count, 1);
        assert!(entry.importance_score > initial_importance);
    }

    #[test]
    fn test_memory_entry_decay() {
        let mut entry = MemoryEntry::new(MemoryType::Working, "test")
            .with_importance(1.0)
            .with_decay(0.9);

        entry.apply_decay(24.0); // 24 hours

        assert!(entry.importance_score < 1.0);
        assert!((entry.importance_score - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_memory_metadata_builder() {
        let metadata = MemoryMetadata::with_source(MemorySource::UserInput)
            .user("user-123")
            .session("session-456")
            .project("project-789")
            .tag("important")
            .tags(vec!["ai", "memory"])
            .confidence(0.95);

        assert_eq!(metadata.user_id, Some("user-123".to_string()));
        assert_eq!(metadata.session_id, Some("session-456".to_string()));
        assert_eq!(metadata.project_id, Some("project-789".to_string()));
        assert_eq!(metadata.tags.len(), 3);
        assert!((metadata.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_memory_relation() {
        let relation = MemoryRelation::new("target-id", RelationType::DerivedFrom, 0.8);
        assert_eq!(relation.target_id, "target-id");
        assert_eq!(relation.relation_type, RelationType::DerivedFrom);
        assert!((relation.strength - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_working_memory, 100);
        assert!(config.auto_decay);
    }

    #[test]
    fn test_memory_config_minimal() {
        let config = MemoryConfig::minimal();
        assert_eq!(config.max_working_memory, 10);
        assert_eq!(config.consolidation_threshold, 5);
    }

    #[test]
    fn test_recall_context_default() {
        let context = RecallContext::default();
        assert!(context.include_semantic);
        assert!(context.include_episodic);
        assert!(context.include_working);
        assert_eq!(context.max_results, 10);
    }

    #[test]
    fn test_recall_context_semantic_only() {
        let context = RecallContext::semantic_only();
        assert!(context.include_semantic);
        assert!(!context.include_episodic);
        assert!(!context.include_working);
    }

    #[test]
    fn test_recall_context_working_only() {
        let context = RecallContext::working_only();
        assert!(!context.include_semantic);
        assert!(context.include_working);
    }

    #[test]
    fn test_recall_context_builder() {
        let context = RecallContext::default()
            .for_user("user-123")
            .for_session("session-456")
            .limit(5)
            .min_relevance(0.5);

        assert_eq!(context.user_id, Some("user-123".to_string()));
        assert_eq!(context.session_id, Some("session-456".to_string()));
        assert_eq!(context.max_results, 5);
        assert!((context.min_relevance - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decay_report_total() {
        let report = DecayReport {
            working_removed: 5,
            episodic_removed: 10,
            semantic_removed: 2,
        };
        assert_eq!(report.total_removed(), 17);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            working_count: 50,
            working_capacity: 100,
            episodic_count: 200,
            episodic_capacity: 1000,
            semantic_count: 500,
            semantic_capacity: 10000,
        };

        assert_eq!(stats.total_count(), 750);
        assert!((stats.working_utilization() - 0.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_memory_manager_creation() {
        let config = MemoryConfig::minimal();
        let manager = MemoryManager::new(config).unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 0);
        assert_eq!(stats.episodic_count, 0);
        assert_eq!(stats.semantic_count, 0);
    }

    #[tokio::test]
    async fn test_memory_manager_store_and_recall() {
        let config = MemoryConfig::minimal();
        let manager = MemoryManager::new(config).unwrap();

        // Store a working memory
        let entry = MemoryEntry::new(MemoryType::Working, "The user prefers dark mode");
        let id = manager.store(entry).await.unwrap();

        // Recall
        let context = RecallContext::working_only();
        let results = manager.recall("dark mode preference", &context).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
    }

    #[tokio::test]
    async fn test_memory_manager_get_by_id() {
        let config = MemoryConfig::minimal();
        let manager = MemoryManager::new(config).unwrap();

        let entry = MemoryEntry::new(MemoryType::Working, "test content")
            .with_id("test-id-123");
        manager.store(entry).await.unwrap();

        let retrieved = manager.get("test-id-123").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "test content");
    }

    #[tokio::test]
    async fn test_memory_manager_delete() {
        let config = MemoryConfig::minimal();
        let manager = MemoryManager::new(config).unwrap();

        let entry = MemoryEntry::new(MemoryType::Working, "to be deleted")
            .with_id("delete-me");
        manager.store(entry).await.unwrap();

        let deleted = manager.delete("delete-me").await.unwrap();
        assert!(deleted);

        let retrieved = manager.get("delete-me").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_memory_manager_clear() {
        let config = MemoryConfig::minimal();
        let manager = MemoryManager::new(config).unwrap();

        // Store some memories
        for i in 0..5 {
            let entry = MemoryEntry::new(MemoryType::Working, format!("memory {}", i));
            manager.store(entry).await.unwrap();
        }

        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 5);

        manager.clear().await.unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 0);
    }
}
