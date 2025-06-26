use mcp_core::content::TextContent;
use mcp_core::tool::Tool;
use mcp_core::{Content, ToolError};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::tool_vectordb::ToolVectorDB;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::{self, base::Provider};

#[derive(Debug, Clone, PartialEq)]
pub enum RouterToolSelectionStrategy {
    Vector,
    VectorWithExtension,
    Llm,
    VectorPassthrough,
    VectorWithExtensionPassthrough,
    LlmPassthrough,
    LlmParallel,
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError>;
    async fn select_tools_with_context(
        &self,
        params: Value,
        _user_message: Option<&str>,
    ) -> Result<Vec<Content>, ToolError> {
        // Default implementation just calls select_tools (for backward compatibility)
        self.select_tools(params).await
    }
    async fn index_tools(
        &self,
        tools: &[Tool],
        extension_name: &str,
        extension_description: Option<&str>,
    ) -> Result<(), ToolError>;
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ToolError>;
    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ToolError>;
    fn selector_type(&self) -> RouterToolSelectionStrategy;
}

pub struct VectorToolSelector {
    vector_db: Arc<RwLock<ToolVectorDB>>,
    embedding_provider: Arc<dyn Provider>,
    recent_tool_calls: Arc<RwLock<VecDeque<String>>>,
    strategy: RouterToolSelectionStrategy,
}

impl VectorToolSelector {
    pub async fn new(
        provider: Arc<dyn Provider>,
        table_name: String,
        strategy: RouterToolSelectionStrategy,
    ) -> Result<Self> {
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
            strategy,
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

        // Extract extension_name from params if present (optional for backward compatibility)
        let extension_name = params.get("extension_name").and_then(|v| v.as_str());

        // Log the incoming query
        tracing::warn!(
            "Vector search query received: query='{}', k={}, extension_name={:?}",
            query,
            k,
            extension_name
        );

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
            .search_tools(query_embedding, k, extension_name)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to search tools: {}", e)))?;

        // Log the vector search results
        let tool_names: Vec<&str> = tools.iter().map(|t| t.tool_name.as_str()).collect();
        tracing::warn!(
            "Vector search returned {} tools for query '{}' with extension filter {:?}: {:?}",
            tools.len(),
            query,
            extension_name,
            tool_names
        );

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

    async fn select_tools_with_context(
        &self,
        params: Value,
        user_message: Option<&str>,
    ) -> Result<Vec<Content>, ToolError> {
        let mut params = params;

        // For passthrough strategies, use the actual user message from session context
        let is_passthrough = matches!(
            self.strategy,
            RouterToolSelectionStrategy::VectorPassthrough
                | RouterToolSelectionStrategy::VectorWithExtensionPassthrough
        );

        if is_passthrough {
            // Use the actual user message from session context if available,
            // otherwise fall back to user_query from tool parameters
            let user_query = if let Some(msg) = user_message {
                msg.to_string()
            } else {
                params
                    .get("user_query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };

            let additional_context = params
                .get("additional_context")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Combine user_query and additional_context, prioritizing the user's actual message
            let combined_query = if user_message.is_some() {
                // When we have the actual user message, weight it more heavily
                format!("{} {} {}", user_query, user_query, additional_context)
                    .trim()
                    .to_string()
            } else {
                // Fallback for when we only have tool parameters
                format!("{} {}", user_query, additional_context)
                    .trim()
                    .to_string()
            };

            // Update the params with the combined query for the underlying search
            if let Some(obj) = params.as_object_mut() {
                obj.insert("query".to_string(), Value::String(combined_query.clone()));
            }

            tracing::warn!(
                "Vector passthrough search: using user_message '{}' with additional_context '{}' into final query '{}'",
                user_query,
                additional_context,
                combined_query
            );
        }

        // Call the regular select_tools with potentially modified params
        self.select_tools(params).await
    }

    async fn index_tools(
        &self,
        tools: &[Tool],
        extension_name: &str,
        extension_description: Option<&str>,
    ) -> Result<(), ToolError> {
        let extension_context = extension_description
            .map(|desc| {
                format!(
                    " Extension Name: {}\nExtension Description: {}",
                    extension_name, desc
                )
            })
            .unwrap_or_else(|| format!(" Extension: {}", extension_name));

        let texts_to_embed: Vec<String> = tools
            .iter()
            .map(|tool| {
                let schema_str = serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string());
                format!(
                    "{}\nTool Name:{}\nTool Description:{}\nTool Schema:{}\n",
                    extension_context, tool.name, tool.description, schema_str,
                )
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
                    extension_name: extension_name.to_string(),
                }
            })
            .collect();

        // Get vector_db lock
        let vector_db = self.vector_db.read().await;

        // Filter out tools that already exist in the database
        let mut new_tool_records = Vec::new();
        for record in tool_records {
            // Check if tool exists by searching for it
            let existing_tools = vector_db
                .search_tools(record.vector.clone(), 1, Some(&record.extension_name))
                .await
                .map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to search for existing tools: {}", e))
                })?;

            // Only add if no exact match found
            if !existing_tools
                .iter()
                .any(|t| t.tool_name == record.tool_name)
            {
                new_tool_records.push(record);
            }
        }

        // Only index if there are new tools to add
        if !new_tool_records.is_empty() {
            vector_db
                .index_tools(new_tool_records)
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to index tools: {}", e)))?;
        }

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
        self.strategy.clone()
    }
}

pub struct LLMToolSelector {
    llm_provider: Arc<dyn Provider>,
    tool_strings: Arc<RwLock<HashMap<String, String>>>, // extension_name -> tool_string
    recent_tool_calls: Arc<RwLock<VecDeque<String>>>,
}

impl LLMToolSelector {
    pub async fn new(provider: Arc<dyn Provider>) -> Result<Self> {
        Ok(Self {
            llm_provider: provider.clone(),
            tool_strings: Arc::new(RwLock::new(HashMap::new())),
            recent_tool_calls: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        })
    }
}

#[async_trait]
impl RouterToolSelector for LLMToolSelector {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'query' parameter".to_string()))?;

        let extension_name = params
            .get("extension_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let k = params.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        // Log the incoming query
        tracing::warn!(
            "LLM search query received: query='{}', k={}, extension_name={:?}",
            query,
            k,
            extension_name
        );

        // Get relevant tool strings based on extension_name
        let tool_strings = self.tool_strings.read().await;
        let relevant_tools = if let Some(ext) = &extension_name {
            tool_strings.get(ext).cloned()
        } else {
            // If no extension specified, use all tools
            Some(
                tool_strings
                    .values()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        };

        if let Some(tools) = relevant_tools {
            // Use LLM to search through tools
            let prompt = format!(
                "Given the following tools:\n{}\n\nFind the most relevant tools for the query: {}\n\nReturn the tools in this exact format for each tool:\nTool: <tool_name>\nDescription: <tool_description>\nSchema: <tool_schema>",
                tools, query
            );
            let system_message = Message::user().with_text("You are a tool selection assistant. Your task is to find the most relevant tools based on the user's query.");
            let response = self
                .llm_provider
                .complete(&prompt, &[system_message], &[])
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to search tools: {}", e)))?;

            // Extract just the message content from the response
            let (message, _usage) = response;
            let text = message.content[0].as_text().unwrap_or_default();

            // Split the response into individual tool entries
            let tool_entries: Vec<Content> = text
                .split("\n\n")
                .filter(|entry| entry.trim().starts_with("Tool:"))
                .map(|entry| {
                    Content::Text(TextContent {
                        text: entry.trim().to_string(),
                        annotations: None,
                    })
                })
                .collect();

            // Log the LLM search results
            let tool_names: Vec<String> = tool_entries
                .iter()
                .filter_map(|content| {
                    if let Content::Text(text_content) = content {
                        text_content
                            .text
                            .lines()
                            .next()
                            .and_then(|line| line.strip_prefix("Tool: "))
                    } else {
                        None
                    }
                })
                .map(|s| s.to_string())
                .collect();
            tracing::warn!(
                "LLM search returned {} tools for query '{}' with extension filter {:?}: {:?}",
                tool_entries.len(),
                query,
                extension_name,
                tool_names
            );

            Ok(tool_entries)
        } else {
            Ok(vec![])
        }
    }

    async fn select_tools_with_context(
        &self,
        params: Value,
        user_message: Option<&str>,
    ) -> Result<Vec<Content>, ToolError> {
        let mut params = params;

        // For LLM passthrough strategy, use the actual user message from session context
        // Use the actual user message from session context if available,
        // otherwise fall back to user_query from tool parameters
        let user_query = if let Some(msg) = user_message {
            msg.to_string()
        } else {
            params
                .get("user_query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let additional_context = params
            .get("additional_context")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combine user_query and additional_context, prioritizing the user's actual message
        let combined_query = if user_message.is_some() {
            // When we have the actual user message, weight it more heavily
            format!("{} {} {}", user_query, user_query, additional_context)
                .trim()
                .to_string()
        } else {
            // Fallback for when we only have tool parameters
            format!("{} {}", user_query, additional_context)
                .trim()
                .to_string()
        };

        // Update the params with the combined query for the underlying search
        if let Some(obj) = params.as_object_mut() {
            obj.insert("query".to_string(), Value::String(combined_query.clone()));
        }

        tracing::warn!(
            "LLM passthrough search: using user_message '{}' with additional_context '{}' into final query '{}'",
            user_query,
            additional_context,
            combined_query
        );

        // Call the regular select_tools with potentially modified params
        self.select_tools(params).await
    }

    async fn index_tools(
        &self,
        tools: &[Tool],
        extension_name: &str,
        extension_description: Option<&str>,
    ) -> Result<(), ToolError> {
        let mut tool_strings = self.tool_strings.write().await;

        let extension_context = extension_description
            .map(|desc| format!("\nExtension: {} - {}", extension_name, desc))
            .unwrap_or_else(|| format!("\nExtension: {}", extension_name));

        for tool in tools {
            let tool_string = format!(
                "Tool: {}\nDescription: {}\nSchema: {}{}",
                tool.name,
                tool.description,
                serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string()),
                extension_context
            );

            // Use the provided extension_name instead of parsing from tool name
            let entry = tool_strings.entry(extension_name.to_string()).or_default();

            // Check if this tool already exists in the entry
            if !entry.contains(&format!("Tool: {}", tool.name)) {
                if !entry.is_empty() {
                    entry.push_str("\n\n");
                }
                entry.push_str(&tool_string);
            }
        }

        Ok(())
    }
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError> {
        let mut tool_strings = self.tool_strings.write().await;
        if let Some(extension_name) = tool_name.split("__").next() {
            tool_strings.remove(extension_name);
        }
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
        RouterToolSelectionStrategy::Llm
    }
}

pub struct LlmParallelToolSelector {
    llm_provider: Arc<dyn Provider>,
    tool_strings: Arc<RwLock<HashMap<String, String>>>, // extension_name -> tool_string
    extension_descriptions: Arc<RwLock<HashMap<String, String>>>, // extension_name -> description
    recent_tool_calls: Arc<RwLock<VecDeque<String>>>,
}

impl LlmParallelToolSelector {
    pub async fn new(provider: Arc<dyn Provider>) -> Result<Self> {
        Ok(Self {
            llm_provider: provider.clone(),
            tool_strings: Arc::new(RwLock::new(HashMap::new())),
            extension_descriptions: Arc::new(RwLock::new(HashMap::new())),
            recent_tool_calls: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        })
    }
}

#[async_trait]
impl RouterToolSelector for LlmParallelToolSelector {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'query' parameter".to_string()))?;

        let extension_names = params
            .get("extension_names")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ToolError::InvalidParameters("Missing 'extension_names' parameter".to_string())
            })?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let k = params.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        // Log the incoming query
        tracing::warn!(
            "LLM parallel search query received: query='{}', k={}, extension_names={:?}",
            query,
            k,
            extension_names
        );

        // Get relevant tool strings and descriptions for all specified extensions
        let tool_strings = self.tool_strings.read().await;
        let extension_descriptions = self.extension_descriptions.read().await;
        let mut extension_tools = Vec::new();

        for ext_name in &extension_names {
            if let Some(tools) = tool_strings.get(ext_name) {
                let description = extension_descriptions.get(ext_name).cloned();
                extension_tools.push((ext_name.clone(), tools.clone(), description));
            }
        }

        if extension_tools.is_empty() {
            return Ok(vec![Content::Text(TextContent {
                text: "no tools relevant".to_string(),
                annotations: None,
            })]);
        }

        // Fire off parallel LLM requests for each extension
        let futures: Vec<_> = extension_tools
            .into_iter()
            .map(|(ext_name, tools, description)| {
                let provider = self.llm_provider.clone();
                let query = query.to_string();
                async move {
                    let extension_context = if let Some(desc) = description {
                        format!("Extension '{}' - {}:\n", ext_name, desc)
                    } else {
                        format!("Extension '{}':\n", ext_name)
                    };
                    
                    let prompt = format!(
                        "Given the following tools from {}{}

Find the most relevant tools for the query: {}

Return the tools in this exact format for each tool:
Tool: <tool_name>
Description: <tool_description>
Schema: <tool_schema>

If none of the tools are appropriate, then respond with 'no tools relevant'",
                        extension_context, tools, query
                    );
                    let system_message = Message::user().with_text("You are a tool selection assistant. Your task is to find the most relevant tools based on the user's query.");
                    let response = provider
                        .complete(&prompt, &[system_message], &[])
                        .await
                        .map_err(|e| ToolError::ExecutionError(format!("Failed to search tools for extension {}: {}", ext_name, e)))?;

                    let (message, _usage) = response;
                    let text = message.content[0].as_text().unwrap_or_default();
                    Ok::<(String, String), ToolError>((ext_name.to_string(), text.to_string()))
                }
            })
            .collect();

        // Await all parallel requests
        let results = futures::future::try_join_all(futures).await?;

        // Collect all tool entries from all extensions
        let mut all_tool_entries = Vec::new();
        let mut found_relevant_tools = false;

        for (ext_name, response_text) in results {
            // Check if the response indicates no relevant tools
            if response_text
                .trim()
                .to_lowercase()
                .contains("no tools relevant")
            {
                tracing::warn!(
                    "Extension '{}' returned no relevant tools for query '{}'",
                    ext_name,
                    query
                );
                continue;
            }

            // Split the response into individual tool entries
            let tool_entries: Vec<Content> = response_text
                .split("\n\n")
                .filter(|entry| entry.trim().starts_with("Tool:"))
                .map(|entry| {
                    found_relevant_tools = true;
                    Content::Text(TextContent {
                        text: entry.trim().to_string(),
                        annotations: None,
                    })
                })
                .collect();

            all_tool_entries.extend(tool_entries);
        }

        // If no relevant tools were found across all extensions, return the fallback message
        if !found_relevant_tools {
            return Ok(vec![Content::Text(TextContent {
                text: "no tools relevant".to_string(),
                annotations: None,
            })]);
        }

        // Log the parallel LLM search results
        let tool_names: Vec<String> = all_tool_entries
            .iter()
            .filter_map(|content| {
                if let Content::Text(text_content) = content {
                    text_content
                        .text
                        .lines()
                        .next()
                        .and_then(|line| line.strip_prefix("Tool: "))
                } else {
                    None
                }
            })
            .map(|s| s.to_string())
            .collect();
        tracing::warn!(
            "LLM parallel search returned {} tools for query '{}' across extensions {:?}: {:?}",
            all_tool_entries.len(),
            query,
            extension_names,
            tool_names
        );

        Ok(all_tool_entries)
    }

    async fn select_tools_with_context(
        &self,
        params: Value,
        user_message: Option<&str>,
    ) -> Result<Vec<Content>, ToolError> {
        let mut params = params;

        // Use the actual user message from session context if available,
        // otherwise fall back to user_query from tool parameters
        let user_query = if let Some(msg) = user_message {
            msg.to_string()
        } else {
            params
                .get("user_query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let additional_context = params
            .get("additional_context")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Combine user_query and additional_context, prioritizing the user's actual message
        let combined_query = if user_message.is_some() {
            // When we have the actual user message, weight it more heavily
            format!("{} {} {}", user_query, user_query, additional_context)
                .trim()
                .to_string()
        } else {
            // Fallback for when we only have tool parameters
            format!("{} {}", user_query, additional_context)
                .trim()
                .to_string()
        };

        // Update the params with the combined query for the underlying search
        if let Some(obj) = params.as_object_mut() {
            obj.insert("query".to_string(), Value::String(combined_query.clone()));
        }

        tracing::warn!(
            "LLM parallel passthrough search: using user_message '{}' with additional_context '{}' into final query '{}'",
            user_query,
            additional_context,
            combined_query
        );

        // Call the regular select_tools with potentially modified params
        self.select_tools(params).await
    }

    async fn index_tools(
        &self,
        tools: &[Tool],
        extension_name: &str,
        extension_description: Option<&str>,
    ) -> Result<(), ToolError> {
        let mut tool_strings = self.tool_strings.write().await;
        let mut extension_descriptions = self.extension_descriptions.write().await;

        // Store the extension description
        if let Some(desc) = extension_description {
            extension_descriptions.insert(extension_name.to_string(), desc.to_string());
        }

        let extension_context = extension_description
            .map(|desc| format!("\nExtension: {} - {}", extension_name, desc))
            .unwrap_or_else(|| format!("\nExtension: {}", extension_name));

        for tool in tools {
            let tool_string = format!(
                "Tool: {}\nDescription: {}\nSchema: {}{}",
                tool.name,
                tool.description,
                serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string()),
                extension_context
            );

            // Use the provided extension_name instead of parsing from tool name
            let entry = tool_strings.entry(extension_name.to_string()).or_default();

            // Check if this tool already exists in the entry
            if !entry.contains(&format!("Tool: {}", tool.name)) {
                if !entry.is_empty() {
                    entry.push_str("\n\n");
                }
                entry.push_str(&tool_string);
            }
        }

        Ok(())
    }

    async fn remove_tool(&self, tool_name: &str) -> Result<(), ToolError> {
        let mut tool_strings = self.tool_strings.write().await;
        let mut extension_descriptions = self.extension_descriptions.write().await;
        if let Some(extension_name) = tool_name.split("__").next() {
            tool_strings.remove(extension_name);
            extension_descriptions.remove(extension_name);
        }
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
        RouterToolSelectionStrategy::LlmParallel
    }
}

// Helper function to create a boxed tool selector
pub async fn create_tool_selector(
    strategy: Option<RouterToolSelectionStrategy>,
    provider: Arc<dyn Provider>,
    table_name: Option<String>,
) -> Result<Box<dyn RouterToolSelector>> {
    match strategy {
        Some(RouterToolSelectionStrategy::Vector) => {
            let selector = VectorToolSelector::new(
                provider,
                table_name.unwrap(),
                RouterToolSelectionStrategy::Vector,
            )
            .await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::VectorWithExtension) => {
            let selector = VectorToolSelector::new(
                provider,
                table_name.unwrap(),
                RouterToolSelectionStrategy::VectorWithExtension,
            )
            .await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::Llm) => {
            let selector = LLMToolSelector::new(provider).await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::VectorPassthrough) => {
            let selector = VectorToolSelector::new(
                provider,
                table_name.unwrap(),
                RouterToolSelectionStrategy::VectorPassthrough,
            )
            .await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::VectorWithExtensionPassthrough) => {
            let selector = VectorToolSelector::new(
                provider,
                table_name.unwrap(),
                RouterToolSelectionStrategy::VectorWithExtensionPassthrough,
            )
            .await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::LlmPassthrough) => {
            let selector = LLMToolSelector::new(provider).await?;
            Ok(Box::new(selector))
        }
        Some(RouterToolSelectionStrategy::LlmParallel) => {
            let selector = LlmParallelToolSelector::new(provider).await?;
            Ok(Box::new(selector))
        }
        None => {
            let selector = LLMToolSelector::new(provider).await?;
            Ok(Box::new(selector))
        }
    }
}
