use chrono::{DateTime, Utc};
use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub memory_type: String,
    pub category: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub content: MemoryContent,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    pub title: String,
    pub description: String,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreMemoryRequest {
    #[schemars(description = "Type of memory (user_preference, context, fact, instruction, note)")]
    #[allow(dead_code)]
    pub memory_type: String,
    #[schemars(
        description = "Optional category classification (personal, work, project, general)"
    )]
    #[allow(dead_code)]
    pub category: Option<String>,
    #[schemars(description = "Short title for the memory")]
    #[allow(dead_code)]
    pub title: String,
    #[schemars(description = "Detailed description or content")]
    #[allow(dead_code)]
    pub description: String,
    #[schemars(description = "Optional tags for categorization")]
    #[allow(dead_code)]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Optional priority level (high, medium, low)")]
    #[allow(dead_code)]
    pub priority: Option<String>,
    #[schemars(description = "Optional expiry date (ISO 8601 format)")]
    #[allow(dead_code)]
    pub expiry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveMemoryRequest {
    #[schemars(description = "Search query to match in title or description")]
    pub query: Option<String>,
    #[schemars(
        description = "Filter by memory type (user_preference, context, fact, instruction, note)"
    )]
    pub memory_type: Option<String>,
    #[schemars(description = "Filter by category (personal, work, project, general)")]
    pub category: Option<String>,
    #[schemars(description = "Filter by tags (must contain all specified tags)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Maximum number of results to return (default 10)")]
    pub limit: Option<u32>,
    #[schemars(description = "Return memories created since this date (ISO 8601)")]
    pub since: Option<String>,
    #[schemars(description = "Return memories created until this date (ISO 8601)")]
    pub until: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateMemoryRequest {
    #[schemars(description = "UUID of the memory to update")]
    #[allow(dead_code)]
    pub id: String,
    #[schemars(description = "New title (optional)")]
    pub title: Option<String>,
    #[schemars(description = "New description (optional)")]
    pub description: Option<String>,
    #[schemars(description = "New tags (optional, replaces existing)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "New priority (optional, high, medium, low)")]
    pub priority: Option<String>,
    #[schemars(description = "New expiry date (optional, ISO 8601)")]
    pub expiry: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteMemoryRequest {
    #[schemars(description = "UUID of the memory to delete")]
    #[allow(dead_code)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub memories: Vec<MemoryEntry>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
    pub oldest: Option<DateTime<Utc>>,
    pub newest: Option<DateTime<Utc>>,
}

impl MemoryEntry {
    pub fn new(
        memory_type: String,
        category: Option<String>,
        title: String,
        description: String,
        tags: Vec<String>,
        priority: Option<String>,
        expiry: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: format!("mem_{}", Utc::now().timestamp()),
            memory_type,
            category,
            timestamp: Utc::now(),
            content: MemoryContent {
                title,
                description,
                metadata: MemoryMetadata {
                    tags: tags.clone(),
                    priority,
                    expiry,
                },
            },
            tags,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.content.metadata.expiry {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.content.title.to_lowercase().contains(&query_lower)
            || self
                .content
                .description
                .to_lowercase()
                .contains(&query_lower)
            || self
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower))
            || self
                .category
                .as_ref()
                .is_some_and(|cat| cat.to_lowercase().contains(&query_lower))
    }
}
