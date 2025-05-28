use mcp_core::content::TextContent;
use mcp_core::{Content, ToolError};

use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::embeddings::{create_embedding_provider, EmbeddingProviderTrait};
use crate::agents::tool_vectordb::ToolVectorDB;
use crate::providers::base::Provider;

#[derive(Debug, Clone)]
pub enum RouterToolSelectionStrategy {
    Vector,
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError>;
    async fn index_tool(
        &self,
        tool_name: String,
        description: String,
        schema: String,
    ) -> Result<(), ToolError>;
    async fn clear_tools(&self) -> Result<(), ToolError>;
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ToolError>;
}

pub struct VectorToolSelector {
    vector_db: Arc<RwLock<ToolVectorDB>>,
    embedding_provider: Arc<Box<dyn EmbeddingProviderTrait>>,
    recent_tool_calls: Arc<RwLock<VecDeque<String>>>,
}

impl VectorToolSelector {
    pub async fn new(provider: Arc<dyn Provider>, table_name: String) -> Result<Self, ToolError> {
        let vector_db = ToolVectorDB::new(Some(table_name))
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to create vector DB: {}", e)))?;

        let embedding_provider = create_embedding_provider(provider.clone()).await;

        Ok(Self {
            vector_db: Arc::new(RwLock::new(vector_db)),
            embedding_provider: Arc::new(embedding_provider),
            recent_tool_calls: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        })
    }
}

#[async_trait]
impl RouterToolSelector for VectorToolSelector {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        eprintln!("[DEBUG] Received params: {:?}", params);

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'query' parameter".to_string()))?;

        eprintln!("[DEBUG] Extracted query: {}", query);

        let k = params.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        eprintln!("[DEBUG] Using k value: {}", k);

        // Generate embedding for the query
        eprintln!("[DEBUG] Generating embedding for query...");
        let query_embedding = self
            .embedding_provider
            .embed_single(query.to_string())
            .await
            .map_err(|e| {
                eprintln!("[DEBUG] Embedding generation failed: {}", e);
                ToolError::ExecutionError(format!("Failed to generate query embedding: {}", e))
            })?;
        eprintln!("[DEBUG] Successfully generated embedding");

        // Search for similar tools
        eprintln!("[DEBUG] Starting vector search...");
        let vector_db = self.vector_db.read().await;
        let tools = vector_db
            .search_tools(query_embedding, k)
            .await
            .map_err(|e| {
                eprintln!("[DEBUG] Vector search failed: {}", e);
                ToolError::ExecutionError(format!("Failed to search tools: {}", e))
            })?;
        eprintln!(
            "[DEBUG] Vector search completed, found {} tools",
            tools.len()
        );

        // Convert tool records to Content
        let selected_tools: Vec<Content> = tools
            .into_iter()
            .map(|tool| {
                let text = format!(
                    "Tool: {}\nDescription: {}\nSchema: {}",
                    tool.tool_name, tool.description, tool.schema
                );
                Content::Text(TextContent {
                    text,
                    annotations: None,
                })
            })
            .collect();

        eprintln!(
            "[DEBUG] Successfully converted {} tools to Content",
            selected_tools.len()
        );
        Ok(selected_tools)
    }

    async fn index_tool(
        &self,
        tool_name: String,
        description: String,
        schema: String,
    ) -> Result<(), ToolError> {
        // Create text to embed
        let text_to_embed = format!("{} {} {}", tool_name, description, schema);

        // Generate embedding
        let embedding = self
            .embedding_provider
            .embed_single(text_to_embed)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!("Failed to generate tool embedding: {}", e))
            })?;

        // Index the tool
        let vector_db = self.vector_db.read().await;
        let tool_record = crate::agents::tool_vectordb::ToolRecord {
            tool_name,
            description,
            schema,
            vector: embedding,
        };

        vector_db
            .index_tools(vec![tool_record])
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to index tool: {}", e)))?;

        Ok(())
    }

    async fn clear_tools(&self) -> Result<(), ToolError> {
        let vector_db = self.vector_db.write().await;
        vector_db
            .clear_tools()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to clear tools: {}", e)))?;
        Ok(())
    }

    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError> {
        let vector_db = self.vector_db.read().await;
        vector_db.remove_tool(tool_name).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to remove tool {}: {}", tool_name, e))
        })?;
        Ok(())
    }

    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ToolError> {
        let mut recent_calls = self.recent_tool_calls.write().await;
        if recent_calls.len() >= 100 {
            recent_calls.pop_front();
        }
        recent_calls.push_back(tool_name.to_string());
        Ok(())
    }

    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ToolError> {
        let recent_calls = self.recent_tool_calls.read().await;
        Ok(recent_calls.iter().rev().take(limit).cloned().collect())
    }
}

// Helper function to create a boxed tool selector
pub async fn create_tool_selector(
    strategy: Option<RouterToolSelectionStrategy>,
    provider: Arc<dyn Provider>,
    table_name: String,
) -> Result<Box<dyn RouterToolSelector>, ToolError> {
    match strategy {
        Some(RouterToolSelectionStrategy::Vector) => {
            let selector = VectorToolSelector::new(provider, table_name).await?;
            Ok(Box::new(selector))
        }
        None => {
            let selector = VectorToolSelector::new(provider, table_name).await?;
            Ok(Box::new(selector))
        }
    }
}
