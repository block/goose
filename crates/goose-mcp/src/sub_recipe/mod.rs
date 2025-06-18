mod sub_recipe_runner;

use anyhow::Result;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::{JsonRpcMessage, ServerCapabilities},
    resource::Resource,
    role::Role,
    tool::{Tool, ToolAnnotations},
    Content,
};
use mcp_server::{router::CapabilitiesBuilder, Router};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;

use crate::sub_recipe::sub_recipe_runner::{
    run_sub_recipe_command, SubRecipeAttributes, SUB_RECIPE_RUN_DESCRIPTION, SUB_RECIPE_RUN_SCHEMA,
};

pub struct SubRecipeRouter {
    tools: Vec<Tool>,
    sub_recipe_attributes: Option<SubRecipeAttributes>,
}

impl Clone for SubRecipeRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            sub_recipe_attributes: self.sub_recipe_attributes.clone(),
        }
    }
}

impl Default for SubRecipeRouter {
    fn default() -> Self {
        Self::new(None)
    }
}
impl SubRecipeRouter {
    pub fn new(extra_args: Option<String>) -> Self {
        let extra_args_vec: Option<Vec<String>> =
            extra_args.and_then(|args| serde_json::from_str(&args).ok());
        let sub_recipe_attribute_in_json = if let Some(args) = extra_args_vec {
            if !args.is_empty() {
                args.first().cloned()
            } else {
                None
            }
        } else {
            None
        };
        if sub_recipe_attribute_in_json.is_none() {
            return Self {
                tools: vec![],
                sub_recipe_attributes: None,
            };
        }
        let sub_recipe_attributes: Option<SubRecipeAttributes> =
            serde_json::from_str(&sub_recipe_attribute_in_json.unwrap()).ok();
        if sub_recipe_attributes.is_none() {
            return Self {
                tools: vec![],
                sub_recipe_attributes: None,
            };
        }
        let sub_recipe_name = sub_recipe_attributes.as_ref().unwrap().name.clone();
        let sub_recipe_run = Tool::new(
            format!("sub_recipe_run_{}", sub_recipe_name).to_string(),
            SUB_RECIPE_RUN_DESCRIPTION.to_string(),
            serde_json::from_str(SUB_RECIPE_RUN_SCHEMA).unwrap(),
            Some(ToolAnnotations {
                title: Some("Run sub recipe".to_string()),
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );
        Self {
            tools: vec![sub_recipe_run],
            sub_recipe_attributes,
        }
    }

    async fn run_sub_recipe(&self, _params: Value, notifier: mpsc::Sender<JsonRpcMessage>) -> Result<Vec<Content>, ToolError> {
        // let sub_recipe_attributes: SubRecipeAttributes =
        //     serde_json::from_value(params).map_err(|e| {
        //         ToolError::InvalidParameters(format!("Invalid sub-recipe attributes: {}", e))
        //     })?;
        let run_attributes = self.sub_recipe_attributes.as_ref().unwrap();

        run_sub_recipe_command(run_attributes, notifier).await
    }
}

impl Router for SubRecipeRouter {
    fn name(&self) -> String {
        "sub-recipe-router".to_string()
    }

    fn instructions(&self) -> String {
        "Sub recipe MCP to run sub-recipe".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        let arguments = arguments.clone();
        let sub_recipe_name = this.sub_recipe_attributes.as_ref().unwrap().name.clone();
        let tool_name_str = format!("sub_recipe_run_{}", sub_recipe_name);
        Box::pin(async move {
            match tool_name.as_str() {
                t if t == tool_name_str => this.run_sub_recipe(arguments, notifier).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new() // No resources for this MCP
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move {
            Err(ResourceError::NotFound(
                "No resources available".to_string(),
            ))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new() // No prompts for this MCP
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string(); // Clone the string to own it
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt '{}' not found",
                prompt_name
            )))
        })
    }
}
