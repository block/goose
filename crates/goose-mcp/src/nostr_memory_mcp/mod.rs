use chrono::{DateTime, Utc};
use indoc::formatdoc;
use mcp_core::{
    handler::{PromptError, ResourceError},
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{
    Content, ErrorCode, ErrorData, JsonRpcMessage, Prompt, Resource, Tool, ToolAnnotations,
};
use rmcp::object;
use serde_json::Value;
use std::{env, future::Future, pin::Pin};
use tokio::sync::mpsc;

mod client;
mod types;

use client::NostrMemoryClient;
use nostr_sdk::prelude::*;
use types::*;

#[derive(Clone)]
pub struct NostrMcpRouter {
    tools: Vec<Tool>,
    instructions: String,
    nsec: Option<String>,
    nostr_client: Option<NostrMemoryClient>,
}

impl Default for NostrMcpRouter {
    fn default() -> Self {
        Self::new(None)
    }
}

impl NostrMcpRouter {
    pub fn new(nsec: Option<String>) -> Self {
        let nsec = nsec.or_else(|| env::var("NOSTR_NSEC").ok());

        let store_memory_tool = Tool::new(
            "store_memory",
            "Store a new memory entry in Nostr with encryption",
            object!({
                "type": "object",
                "required": ["memory_type", "title", "description"],
                "properties": {
                    "memory_type": {
                        "type": "string",
                        "enum": ["user_preference", "context", "fact", "instruction", "note"],
                        "description": "Type of memory to store"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["personal", "work", "project", "general"],
                        "description": "Optional category classification"
                    },
                    "title": {
                        "type": "string",
                        "description": "Short title for the memory"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed description or content"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional tags for categorization"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["high", "medium", "low"],
                        "description": "Optional priority level"
                    },
                    "expiry": {
                        "type": "string",
                        "description": "Optional expiry date (ISO 8601 format)"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Store Memory".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        });

        let retrieve_memory_tool = Tool::new(
            "retrieve_memory",
            "Retrieve and search memory entries with filtering",
            object!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query to match in title or description"
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["user_preference", "context", "fact", "instruction", "note"],
                        "description": "Filter by memory type"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["personal", "work", "project", "general"],
                        "description": "Filter by category"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Filter by tags (must contain all specified tags)"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 10,
                        "description": "Maximum number of results to return"
                    },
                    "since": {
                        "type": "string",
                        "description": "Return memories created since this date (ISO 8601)"
                    },
                    "until": {
                        "type": "string",
                        "description": "Return memories created until this date (ISO 8601)"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Retrieve Memory".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let update_memory_tool = Tool::new(
            "update_memory",
            "Update an existing memory entry",
            object!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "UUID of the memory to update"
                    },
                    "title": {
                        "type": "string",
                        "description": "New title (optional)"
                    },
                    "description": {
                        "type": "string",
                        "description": "New description (optional)"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "New tags (optional, replaces existing)"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["high", "medium", "low"],
                        "description": "New priority (optional)"
                    },
                    "expiry": {
                        "type": "string",
                        "description": "New expiry date (optional, ISO 8601)"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Update Memory".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let delete_memory_tool = Tool::new(
            "delete_memory",
            "Delete a memory entry by ID",
            object!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "UUID of the memory to delete"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Delete Memory".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let memory_stats_tool = Tool::new(
            "memory_stats",
            "Get statistics about stored memories",
            object!({
                "type": "object",
                "properties": {}
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Memory Stats".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let cleanup_expired_tool = Tool::new(
            "cleanup_expired_memories",
            "Clean up expired memories",
            object!({
                "type": "object",
                "properties": {}
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Cleanup Expired".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let instructions = formatdoc! {r#"
            You are a helpful assistant with access to persistent memory storage using Nostr protocol.

            🧠 **MEMORY OPERATIONS**:

            store_memory
              - Store new memory entries with type, category, tags, and optional expiry
              - All memories are encrypted using Nostr NIP-17 private direct messages
              - Memories are sent as encrypted DMs to yourself for maximum privacy
              - Uses gift wrap encryption protocol for perfect forward secrecy
              - Each memory gets a unique UUID for precise identification

            retrieve_memory
              - Search and filter memories by query, type, category, tags, or date range
              - Supports full-text search across titles and descriptions
              - Powerful filtering capabilities for organized memory retrieval

            update_memory
              - Modify existing memory entries by UUID
              - Update any field including title, description, tags, priority, or expiry

            delete_memory
              - Remove memory entries by UUID
              - Permanently deletes the memory from Nostr storage

            memory_stats
              - Get comprehensive statistics about stored memories
              - Shows totals by type, category, and date ranges

            cleanup_expired_memories
              - Remove expired memory entries automatically
              - Helps maintain optimal storage and performance

            🔐 **PRIVACY & SECURITY**:
            • All memories are encrypted using Nostr NIP-17 private direct messages
            • Memories are sent as encrypted DMs to yourself for maximum privacy
            • Uses gift wrap encryption with perfect forward secrecy
            • Content is end-to-end encrypted and only you can decrypt it
            • Each memory has a unique UUID for precise identification
            • Memories can have expiry dates for automatic cleanup
            • Deletion markers are also encrypted for privacy

            📋 **MEMORY TYPES**:
            • user_preference: User preferences and settings
            • context: Contextual information about conversations
            • fact: Important facts to remember
            • instruction: Instructions or commands to remember
            • note: General notes and observations

            📂 **CATEGORIES**:
            • personal: Personal information
            • work: Work-related memories
            • project: Project-specific information
            • general: General purpose memories

            🏷️ **FEATURES**:
            • Full-text search across titles and descriptions
            • Tag-based organization and filtering
            • Priority levels (high, medium, low)
            • Date range filtering
            • Automatic expiry handling
            • Comprehensive statistics

            💡 **USAGE TIPS**:
            • Use descriptive titles for easy searching
            • Add relevant tags for better organization
            • Set expiry dates for temporary information
            • Use appropriate types and categories for filtering
            • Regular cleanup of expired memories keeps storage optimal

            {}
            "#,
            if nsec.is_some() {
                "🔑 **CONFIGURATION**: Nostr private key (nsec) is configured for memory storage."
            } else {
                "⚠️ **CONFIGURATION**: No Nostr private key configured. Memory operations may not work properly."
            }
        };

        let nostr_client = if let Some(ref nsec_str) = nsec {
            Self::create_nostr_client(nsec_str).ok()
        } else {
            None
        };

        Self {
            tools: vec![
                store_memory_tool,
                retrieve_memory_tool,
                update_memory_tool,
                delete_memory_tool,
                memory_stats_tool,
                cleanup_expired_tool,
            ],
            instructions,
            nsec,
            nostr_client,
        }
    }

    pub fn get_nsec(&self) -> Option<&str> {
        self.nsec.as_deref()
    }

    fn create_nostr_client(
        nsec: &str,
    ) -> Result<NostrMemoryClient, Box<dyn std::error::Error + Send + Sync>> {
        let secret_key = SecretKey::from_bech32(nsec)?;
        let keys = Keys::new(secret_key);
        let client = Client::new(keys.clone());

        Ok(NostrMemoryClient::new(client, keys))
    }

    async fn store_memory(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let memory_type = params
            .get("memory_type")
            .and_then(|v| v.as_str())
            .unwrap_or("note");
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let category = params.get("category").and_then(|v| v.as_str());
        let tags = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        let priority = params.get("priority").and_then(|v| v.as_str());

        let expiry = params
            .get("expiry")
            .and_then(|v| v.as_str())
            .and_then(|expiry_str| DateTime::parse_from_rfc3339(expiry_str).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let memory_entry = MemoryEntry::new(
            memory_type.to_string(),
            category.map(|s| s.to_string()),
            title.to_string(),
            description.to_string(),
            tags.clone(),
            priority.map(|s| s.to_string()),
            expiry,
        );

        match nostr_client.store_memory(&memory_entry).await {
            Ok(_) => {
                let memory_summary = format!(
                    "🧠 **Memory Stored Successfully!**\n\n\
                    📝 **Title:** {}\n\
                    🆔 **ID:** {}\n\
                    📅 **Created:** {}\n\
                    🏷️ **Type:** {}\n\
                    {}{}{}📄 **Description:** {}\n\
                    🔑 **nsec:** {}...configured ✅\n\n\
                    ✅ *Note: Memory encrypted and stored on Nostr network!*",
                    memory_entry.content.title,
                    &memory_entry.id[..8],
                    memory_entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    memory_entry.memory_type,
                    memory_entry
                        .category
                        .as_ref()
                        .map(|c| format!("📂 **Category:** {c}\n"))
                        .unwrap_or_default(),
                    if memory_entry.content.metadata.tags.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "🏷️ **Tags:** {}\n",
                            memory_entry.content.metadata.tags.join(", ")
                        )
                    },
                    memory_entry
                        .content
                        .metadata
                        .priority
                        .as_ref()
                        .map(|p| format!("⭐ **Priority:** {p}\n"))
                        .unwrap_or_default(),
                    memory_entry.content.description,
                    self.nsec
                        .as_ref()
                        .unwrap()
                        .chars()
                        .take(10)
                        .collect::<String>()
                );

                Ok(vec![Content::text(memory_summary)])
            }
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Store Memory**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                Ok(vec![Content::text(error_message)])
            }
        }
    }

    async fn retrieve_memory(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let query = params.get("query").and_then(|v| v.as_str());
        let memory_type = params.get("memory_type").and_then(|v| v.as_str());
        let category = params.get("category").and_then(|v| v.as_str());
        let tags = params.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let _since = params.get("since").and_then(|v| v.as_str());
        let _until = params.get("until").and_then(|v| v.as_str());

        let retrieve_filter = RetrieveMemoryRequest {
            query: query.map(|s| s.to_string()),
            memory_type: memory_type.map(|s| s.to_string()),
            category: category.map(|s| s.to_string()),
            tags,
            limit: Some(limit as u32),
            since: _since.map(|s| s.to_string()),
            until: _until.map(|s| s.to_string()),
        };

        let filtered_memories = match nostr_client.retrieve_memories(&retrieve_filter).await {
            Ok(memories) => memories,
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Retrieve Memories**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                return Ok(vec![Content::text(error_message)]);
            }
        };

        let result_message = if filtered_memories.is_empty() {
            "🔍 **No Memories Found**\n\nNo memories match your search criteria.".to_string()
        } else {
            let mut message = format!("🧠 **Found {} Memories**\n\n", filtered_memories.len());

            for (i, memory) in filtered_memories.iter().enumerate() {
                message.push_str(&format!(
                    "{}. **{}**\n\
                    🆔 **ID:** {}\n\
                    📅 **Created:** {}\n\
                    🏷️ **Type:** {}\n\
                    {}{}{}📄 **Description:** {}\n\
                    🔑 **nsec:** {}...configured ✅\n\n",
                    i + 1,
                    memory.content.title,
                    &memory.id[..8],
                    memory.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    memory.memory_type,
                    memory
                        .category
                        .as_ref()
                        .map(|c| format!("📂 **Category:** {c}\n"))
                        .unwrap_or_default(),
                    if memory.content.metadata.tags.is_empty() {
                        String::new()
                    } else {
                        format!("🏷️ **Tags:** {}\n", memory.content.metadata.tags.join(", "))
                    },
                    memory
                        .content
                        .metadata
                        .priority
                        .as_ref()
                        .map(|p| format!("⭐ **Priority:** {p}\n"))
                        .unwrap_or_default(),
                    memory.content.description,
                    self.nsec
                        .as_ref()
                        .unwrap()
                        .chars()
                        .take(10)
                        .collect::<String>()
                ));
            }

            message.push_str("✅ *Note: Retrieved from Nostr network!*");
            message
        };

        Ok(vec![Content::text(result_message)])
    }

    async fn update_memory(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let memory_id = params
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: "Missing required parameter: id".into(),
                data: None,
            })?;

        let new_title = params.get("title").and_then(|v| v.as_str());
        let new_description = params.get("description").and_then(|v| v.as_str());
        let new_tags = params.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });
        let new_priority = params.get("priority").and_then(|v| v.as_str());
        let new_expiry = params.get("expiry").and_then(|v| v.as_str());

        let update_request = UpdateMemoryRequest {
            id: memory_id.to_string(),
            title: new_title.map(|s| s.to_string()),
            description: new_description.map(|s| s.to_string()),
            tags: new_tags,
            priority: new_priority.map(|s| s.to_string()),
            expiry: new_expiry.map(|s| s.to_string()),
        };

        let updated_memory = match nostr_client.update_memory(memory_id, &update_request).await {
            Ok(memory) => memory,
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Update Memory**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                return Ok(vec![Content::text(error_message)]);
            }
        };

        let update_summary = format!(
            "✅ **Memory Updated Successfully!**\n\n\
            📝 **Title:** {}\n\
            🆔 **ID:** {}\n\
            📅 **Updated:** {}\n\
            🏷️ **Type:** {}\n\
            {}{}{}📄 **Description:** {}\n\
            🔑 **nsec:** {}...configured ✅\n\n\
            ✅ *Note: Memory updated on Nostr network!*",
            updated_memory.content.title,
            &updated_memory.id[..8],
            updated_memory.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            updated_memory.memory_type,
            updated_memory
                .category
                .as_ref()
                .map(|c| format!("📂 **Category:** {c}\n"))
                .unwrap_or_default(),
            if updated_memory.content.metadata.tags.is_empty() {
                String::new()
            } else {
                format!(
                    "🏷️ **Tags:** {}\n",
                    updated_memory.content.metadata.tags.join(", ")
                )
            },
            updated_memory
                .content
                .metadata
                .priority
                .as_ref()
                .map(|p| format!("⭐ **Priority:** {p}\n"))
                .unwrap_or_default(),
            updated_memory.content.description,
            self.nsec
                .as_ref()
                .unwrap()
                .chars()
                .take(10)
                .collect::<String>()
        );

        Ok(vec![Content::text(update_summary)])
    }

    async fn delete_memory(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let memory_id = params
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: "Missing required parameter: id".into(),
                data: None,
            })?;

        match nostr_client.delete_memory(memory_id).await {
            Ok(_) => {
                let timestamp = Utc::now();
                let deletion_summary = format!(
                    "🗑️ **Memory Deleted Successfully!**\n\n\
                    🆔 **Deleted ID:** {}\n\
                    📅 **Deleted At:** {}\n\
                    🔑 **nsec:** {}...configured ✅\n\n\
                    ✅ *Note: Deletion marker posted to Nostr network.*",
                    &memory_id[..8.min(memory_id.len())],
                    timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    self.nsec
                        .as_ref()
                        .unwrap()
                        .chars()
                        .take(10)
                        .collect::<String>()
                );
                Ok(vec![Content::text(deletion_summary)])
            }
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Delete Memory**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                Ok(vec![Content::text(error_message)])
            }
        }
    }

    async fn memory_stats(&self, _params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let memory_stats = match nostr_client.get_memory_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Get Memory Statistics**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                return Ok(vec![Content::text(error_message)]);
            }
        };

        let mut stats_summary = format!(
            "📊 **Memory Statistics**\n\n\
            🧠 **Total Memories:** {}\n\n",
            memory_stats.total_memories
        );

        if !memory_stats.by_type.is_empty() {
            stats_summary.push_str("📋 **By Type:**\n");
            for (type_name, count) in &memory_stats.by_type {
                stats_summary.push_str(&format!("• {type_name}: {count}\n"));
            }
            stats_summary.push('\n');
        }

        if !memory_stats.by_category.is_empty() {
            stats_summary.push_str("📂 **By Category:**\n");
            for (category_name, count) in &memory_stats.by_category {
                stats_summary.push_str(&format!("• {category_name}: {count}\n"));
            }
            stats_summary.push('\n');
        }

        if let (Some(oldest_dt), Some(newest_dt)) = (memory_stats.oldest, memory_stats.newest) {
            stats_summary.push_str(&format!(
                "📅 **Date Range:**\n\
                • Oldest: {}\n\
                • Newest: {}\n\n",
                oldest_dt.format("%Y-%m-%d %H:%M:%S UTC"),
                newest_dt.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        stats_summary.push_str(&format!(
            "🔑 **nsec:** {}...configured ✅\n\n\
            ✅ *Note: Real-time statistics from Nostr network!*",
            self.nsec
                .as_ref()
                .unwrap()
                .chars()
                .take(10)
                .collect::<String>()
        ));

        Ok(vec![Content::text(stats_summary)])
    }

    async fn cleanup_expired_memories(&self, _params: Value) -> Result<Vec<Content>, ErrorData> {
        let nostr_client = match &self.nostr_client {
            Some(client) => client,
            None => {
                return Ok(vec![Content::text(
                    "❌ Error: Nostr client not initialized. Please configure your nsec properly."
                        .to_string(),
                )]);
            }
        };

        let retrieve_filter = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(10000),
            since: None,
            until: None,
        };

        let all_memories = match nostr_client.retrieve_memories(&retrieve_filter).await {
            Ok(memories) => memories,
            Err(e) => {
                let error_message = format!(
                    "❌ **Failed to Retrieve Memories for Cleanup**\n\n\
                    Error: {e}\n\n\
                    🔧 This might be due to network connectivity or relay issues."
                );
                return Ok(vec![Content::text(error_message)]);
            }
        };

        let mut expired_memories = Vec::new();
        for memory in all_memories {
            if memory.is_expired() {
                expired_memories.push(memory);
            }
        }

        let mut deleted_count = 0;
        for memory in &expired_memories {
            if (nostr_client.delete_memory(&memory.id).await).is_ok() {
                deleted_count += 1;
            }
        }

        let timestamp = Utc::now();
        let expired_count = expired_memories.len();

        let cleanup_summary = if expired_count == 0 {
            format!(
                "✅ **No Expired Memories Found**\n\n\
                📅 **Checked At:** {}\n\
                🧹 All memories are current and valid.\n\
                🔑 **nsec:** {}...configured ✅\n\n\
                ✅ *Note: Real-time cleanup from Nostr network.*",
                timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                self.nsec
                    .as_ref()
                    .unwrap()
                    .chars()
                    .take(10)
                    .collect::<String>()
            )
        } else {
            let mut details = String::new();
            for memory in &expired_memories {
                details.push_str(&format!(
                    "• {}: \"{}\" (expired {})\n",
                    &memory.id[..8],
                    memory.content.title,
                    memory
                        .content
                        .metadata
                        .expiry
                        .map(|exp| exp.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }

            format!(
                "🧹 **Cleanup Completed Successfully!**\n\n\
                📅 **Cleaned At:** {}\n\
                🗑️ **Found Expired:** {}\n\
                ✅ **Successfully Deleted:** {}\n\
                📝 **Details:**\n\
                {}\n\
                🔑 **nsec:** {}...configured ✅\n\n\
                ✅ *Note: Expired memories marked as deleted on Nostr network.*",
                timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                expired_count,
                deleted_count,
                details,
                self.nsec
                    .as_ref()
                    .unwrap()
                    .chars()
                    .take(10)
                    .collect::<String>()
            )
        };

        Ok(vec![Content::text(cleanup_summary)])
    }
}

impl Router for NostrMcpRouter {
    fn name(&self) -> String {
        "NostrMemoryMcpExtension".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ErrorData>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "store_memory" => this.store_memory(arguments).await,
                "retrieve_memory" => this.retrieve_memory(arguments).await,
                "update_memory" => this.update_memory(arguments).await,
                "delete_memory" => this.delete_memory(arguments).await,
                "memory_stats" => this.memory_stats(arguments).await,
                "cleanup_expired_memories" => this.cleanup_expired_memories(arguments).await,
                _ => Err(ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: format!("Tool {tool_name} not found").into(),
                    data: None,
                }),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!(
                "Resource not found: {uri}"
            )))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {prompt_name} not found"
            )))
        })
    }
}
