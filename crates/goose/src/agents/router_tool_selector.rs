use mcp_core::content::TextContent;
use mcp_core::tool::Tool;
use mcp_core::{Content, ToolError};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::tool_vectordb::ToolVectorDB;
use crate::model::ModelConfig;
use crate::providers::{self, base::Provider};

#[derive(Debug, Clone, PartialEq)]
pub enum RouterToolSelectionStrategy {
    Vector,
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError>;
    async fn index_tools(&self, tools: &[Tool]) -> Result<(), ToolError>;
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ToolError>;
    fn selector_type(&self) -> RouterToolSelectionStrategy;
}

pub struct VectorToolSelector {
    vector_db: Arc<RwLock<ToolVectorDB>>,
    embedding_provider: Arc<dyn Provider>,
    recent_tool_calls: Arc<RwLock<VecDeque<String>>>,
}

impl VectorToolSelector {
    pub async fn new(provider: Arc<dyn Provider>, table_name: String) -> Result<Self> {
        let vector_db = ToolVectorDB::new(Some(table_name)).await?;

        let embedding_provider = if env::var("GOOSE_EMBEDDING_MODEL_PROVIDER").is_ok() {
            // If env var is set, create a new provider for embeddings
            // Get embedding model and provider from environment variables
            let embedding_model = env::var("GOOSE_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string());
            let embedding_provider_name =
                env::var("GOOSE_EMBEDDING_MODEL_PROVIDER").unwrap_or_else(|_| "openai".to_string());

            // Create the provider using the factory
            let model_config = ModelConfig::new(embedding_model);
            providers::create(&embedding_provider_name, model_config).context(format!(
                "Failed to create {} provider for embeddings. If using OpenAI, make sure OPENAI_API_KEY env var is set or that you have configured the OpenAI provider via Goose before.",
                embedding_provider_name
            ))?
        } else {
            // Otherwise fall back to using the same provider instance as used for base goose model
            provider.clone()
        };

        Ok(Self {
            vector_db: Arc::new(RwLock::new(vector_db)),
            embedding_provider,
            recent_tool_calls: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        })
    }
}

#[async_trait]
impl RouterToolSelector for VectorToolSelector {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'query' parameter".to_string()))?;

        let k = params.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        // Check if provider supports embeddings
        if !self.embedding_provider.supports_embeddings() {
            return Err(ToolError::ExecutionError(
                "Embedding provider does not support embeddings".to_string(),
            ));
        }

        let embeddings = self
            .embedding_provider
            .create_embeddings(vec![query.to_string()])
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!("Failed to generate query embedding: {}", e))
            })?;

        let query_embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| ToolError::ExecutionError("No embedding returned".to_string()))?;

        let vector_db = self.vector_db.read().await;
        let tools = vector_db
            .search_tools(query_embedding, k)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to search tools: {}", e)))?;

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

        Ok(selected_tools)
    }

    async fn index_tools(&self, tools: &[Tool]) -> Result<(), ToolError> {
        let texts_to_embed: Vec<String> = tools
            .iter()
            .map(|tool| {
                let schema_str = serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string());
                format!("{} {} {}", tool.name, tool.description, schema_str)
            })
            .collect();

        if !self.embedding_provider.supports_embeddings() {
            return Err(ToolError::ExecutionError(
                "Embedding provider does not support embeddings".to_string(),
            ));
        }

        let embeddings = self
            .embedding_provider
            .create_embeddings(texts_to_embed)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!("Failed to generate tool embeddings: {}", e))
            })?;

        // Create tool records
        let tool_records: Vec<crate::agents::tool_vectordb::ToolRecord> = tools
            .iter()
            .zip(embeddings.into_iter())
            .map(|(tool, vector)| {
                let schema_str = serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string());
                crate::agents::tool_vectordb::ToolRecord {
                    tool_name: tool.name.clone(),
                    description: tool.description.clone(),
                    schema: schema_str,
                    vector,
                }
            })
            .collect();

        // Index all tools at once
        let vector_db = self.vector_db.read().await;
        vector_db
            .index_tools(tool_records)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to index tools: {}", e)))?;

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

    fn selector_type(&self) -> RouterToolSelectionStrategy {
        RouterToolSelectionStrategy::Vector
    }
}

// Helper function to create a boxed tool selector
pub async fn create_tool_selector(
    strategy: Option<RouterToolSelectionStrategy>,
    provider: Arc<dyn Provider>,
    table_name: String,
) -> Result<Box<dyn RouterToolSelector>> {
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
