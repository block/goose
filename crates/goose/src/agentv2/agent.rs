use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

use crate::message::Message;
use crate::providers::base::{Provider, ProviderUsage};
use crate::providers::errors::ProviderError;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::agents::extension::{ExtensionConfig, ExtensionResult};
use crate::agents::extension_manager::ExtensionManager;
use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::providers::toolshim::modify_system_prompt_for_tool_json;
use mcp_core::{prompt::Prompt, tool::Tool, Content, ToolError};

use crate::agents::platform_tools;

/// The main goose Agent
pub struct AgentV2 {
    pub(super) provider: Mutex<Arc<dyn Provider>>,
    pub(super) extension_manager: Mutex<ExtensionManager>,
    pub(super) prompt_manager: Mutex<PromptManager>,
}

impl AgentV2 {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            provider: Mutex::new(provider),
            extension_manager: Mutex::new(ExtensionManager::new()),
            prompt_manager: Mutex::new(PromptManager::new()),
        }
    }
}

impl AgentV2 {
    /// Get a reference count clone to the provider
    pub async fn provider(&self) -> Arc<dyn Provider> {
        self.provider.lock().await.clone()
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&self) -> ExtensionResult<Vec<Tool>> {
        let tools = self
            .extension_manager
            .lock()
            .await
            .get_prefixed_tools(None)
            .await?;

        Ok(tools)
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call, request_id), fields(input, output))]
    pub(super) async fn dispatch_tool_call(
        &self,
        tool_call: mcp_core::tool::ToolCall,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let extension_manager = self.extension_manager.lock().await;
        let result = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            extension_manager
                .read_resource(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            extension_manager
                .list_resources(tool_call.arguments.clone())
                .await
        } else {
            extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        (request_id, result)
    }

    pub async fn add_extension(&self, extension: ExtensionConfig) -> ExtensionResult<()> {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager.add_extension(extension).await?;

        Ok(())
    }

    pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<Tool> {
        let extension_manager = self.extension_manager.lock().await;
        let mut prefixed_tools = extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            // Add resource tools if supported
            if extension_manager.supports_resources() {
                prefixed_tools.push(platform_tools::read_resource_tool());
                prefixed_tools.push(platform_tools::list_resources_tool());
            }
        }

        prefixed_tools
    }

    pub async fn remove_extension(&self, name: &str) {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    /// Prepares tools and system prompt for a provider request
    async fn prepare_tools_and_prompt(&self) -> anyhow::Result<(Vec<Tool>, Vec<Tool>, String)> {
        // Get tools from extension manager
        let mut tools = self.list_tools(None).await;

        // Prepare system prompt
        let extension_manager = self.extension_manager.lock().await;
        let extensions_info = extension_manager.get_extensions_info().await;

        // Get model name from provider
        let provider = self.provider().await;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;

        let prompt_manager = self.prompt_manager.lock().await;
        let mut system_prompt = prompt_manager.build_system_prompt(
            extensions_info,
            None,
            extension_manager.suggest_disable_extensions_prompt().await,
            Some(model_name),
        );

        // Handle toolshim if enabled
        let mut toolshim_tools = vec![];
        if model_config.toolshim {
            // If tool interpretation is enabled, modify the system prompt
            system_prompt = modify_system_prompt_for_tool_json(&system_prompt, &tools);
            // Make a copy of tools before emptying
            toolshim_tools = tools.clone();
            // Empty the tools vector for provider completion
            tools = vec![];
        }

        Ok((tools, toolshim_tools, system_prompt))
    }

    /// This replaced the previous 'reply' method
    /// Generate a response from the LLM provider
    /// Handles toolshim transformations if needed
    pub async fn provider_complete(
        &self,
        messages: &[Message],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let messages = messages.to_vec();

        // Setup tools and prompt
        // Not using the toolshim tools in any way
        let (tools, mut _toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;

        let provider = self.provider().await;

        // Call the provider to get a response
        let (response, usage) = provider.complete(&system_prompt, &messages, &tools).await?;

        // Store the model information in the global store
        crate::providers::base::set_current_model(&usage.model);

        Ok((response, usage))
    }

    /// Extend the system prompt with one line of additional instruction
    pub async fn extend_system_prompt(&self, instruction: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.add_system_prompt_extra(instruction);
    }

    /// Update the provider used by this agent
    pub async fn update_provider(&self, provider: Arc<dyn Provider>) -> Result<()> {
        *self.provider.lock().await = provider;
        Ok(())
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&self, template: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.set_system_prompt_override(template);
    }

    pub async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_prompts()
            .await
            .expect("Failed to list prompts")
    }
}
