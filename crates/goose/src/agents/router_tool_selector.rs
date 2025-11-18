use rmcp::model::{Content, ErrorCode, ErrorData};
use rmcp::model::{JsonObject, Tool};

use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::conversation::message::Message;
use crate::prompt_template::render_global_file;
use crate::providers::base::Provider;

#[derive(Serialize)]
struct ToolSelectorContext {
    tools: String,
    query: String,
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(&self, params: JsonObject) -> Result<Vec<Content>, ErrorData>;
    async fn index_tools(&self, tools: &[Tool], extension_name: &str) -> Result<(), ErrorData>;
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ErrorData>;
    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ErrorData>;
    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ErrorData>;
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
    async fn select_tools(&self, params: JsonObject) -> Result<Vec<Content>, ErrorData> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from("Missing 'query' parameter"),
                data: None,
            })?;

        let extension_name = params
            .get("extension_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

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
            // Use template to generate the prompt
            let context = ToolSelectorContext {
                tools: tools.clone(),
                query: query.to_string(),
            };

            let user_prompt =
                render_global_file("router_tool_selector.md", &context).map_err(|e| ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: Cow::from(format!("Failed to render prompt template: {}", e)),
                    data: None,
                })?;

            let user_message = Message::user().with_text(&user_prompt);
            let response = self
                .llm_provider
                .complete("system", &[user_message], &[])
                .await
                .map_err(|e| ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: Cow::from(format!("Failed to search tools: {}", e)),
                    data: None,
                })?;

            // Extract just the message content from the response
            let (message, _usage) = response;
            let text = message.content[0].as_text().unwrap_or_default();

            // Split the response into individual tool entries
            let tool_entries: Vec<Content> = text
                .split("\n\n")
                .filter(|entry| entry.trim().starts_with("Tool:"))
                .map(|entry| Content::text(entry.trim().to_string()))
                .collect();

            Ok(tool_entries)
        } else {
            Ok(vec![])
        }
    }

    async fn index_tools(&self, tools: &[Tool], extension_name: &str) -> Result<(), ErrorData> {
        let mut tool_strings = self.tool_strings.write().await;

        for tool in tools {
            let tool_string = format!(
                "Tool: {}\nDescription: {}\nSchema: {}",
                tool.name,
                tool.description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default(),
                serde_json::to_string_pretty(&tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string())
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
    async fn remove_tool(&self, tool_name: &str) -> Result<(), ErrorData> {
        let mut tool_strings = self.tool_strings.write().await;
        if let Some(extension_name) = tool_name.split("__").next() {
            tool_strings.remove(extension_name);
        }
        Ok(())
    }

    async fn record_tool_call(&self, tool_name: &str) -> Result<(), ErrorData> {
        let mut recent_calls = self.recent_tool_calls.write().await;
        if recent_calls.len() >= 100 {
            recent_calls.pop_front();
        }
        recent_calls.push_back(tool_name.to_string());
        Ok(())
    }

    async fn get_recent_tool_calls(&self, limit: usize) -> Result<Vec<String>, ErrorData> {
        let recent_calls = self.recent_tool_calls.read().await;
        Ok(recent_calls.iter().rev().take(limit).cloned().collect())
    }
}

// Helper function to create a boxed tool selector
pub async fn create_tool_selector(
    provider: Arc<dyn Provider>,
) -> Result<Box<dyn RouterToolSelector>> {
    let selector = LLMToolSelector::new(provider).await?;
    Ok(Box::new(selector))
}
