use anyhow::Result;
use include_dir::{include_dir, Dir};
use indoc::formatdoc;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{Content, JsonRpcMessage, Prompt, Resource, Role, Tool, ToolAnnotations};
use rmcp::object;
use serde_json::Value;
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;

static TUTORIALS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/tutorial/tutorials");

pub struct TutorialRouter {
    tools: Vec<Tool>,
    instructions: String,
}

impl Default for TutorialRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl TutorialRouter {
    pub fn new() -> Self {
        let load_tutorial = Tool::new(
            "load_tutorial".to_string(),
            "Load a specific tutorial by name. The tutorial will be returned as markdown content that provides step by step instructions.".to_string(),
            object!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the tutorial to load, e.g. 'getting-started' or 'developer-mcp'"
                    }
                }
            })
        ).annotate(ToolAnnotations {
            title: Some("Load Tutorial".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        // Get base instructions and available tutorials
        let available_tutorials = Self::get_available_tutorials();

        let instructions = formatdoc! {r#"
            Because the tutorial extension is enabled, be aware that the user may be new to using Goose
            or looking for help with specific features. Proactively offer relevant tutorials when appropriate.

            Available tutorials:
            {tutorials}

            The specific content of the tutorial are available in by running load_tutorial.
            To run through a tutorial, make sure to be interactive with the user. Don't run more than
            a few related tool calls in a row. Make sure to prompt the user for understanding and participation.

            **Important**: Make sure that you provide guidance or info *before* you run commands, as the command will
            run immediately for the user. For example while running a game tutorial, let the user know what to expect
            before you run a command to start the game itself.
            "#,
            tutorials=available_tutorials,
        };

        Self {
            tools: vec![load_tutorial],
            instructions,
        }
    }

    fn get_available_tutorials() -> String {
        let mut tutorials = String::new();
        for file in TUTORIALS_DIR.files() {
            // Use first line for additional context
            let first_line = file
                .contents_utf8()
                .and_then(|s| s.lines().next().map(|line| line.to_string()))
                .unwrap_or_else(String::new);

            if let Some(name) = file.path().file_stem() {
                tutorials.push_str(&format!("- {}: {}\n", name.to_string_lossy(), first_line));
            }
        }
        tutorials
    }

    async fn load_tutorial(&self, name: &str) -> Result<String, ToolError> {
        let file_name = format!("{}.md", name);
        let file = TUTORIALS_DIR
            .get_file(&file_name)
            .ok_or(ToolError::ExecutionError(format!(
                "Could not locate tutorial '{}'",
                name
            )))?;
        Ok(String::from_utf8_lossy(file.contents()).into_owned())
    }
}

impl Router for TutorialRouter {
    fn name(&self) -> String {
        "tutorial".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new().with_tools(false).build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tool_name.as_str() {
                "load_tutorial" => {
                    let name = arguments
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidParameters("Missing 'name' parameter".to_string())
                        })?;

                    let content = this.load_tutorial(name).await?;
                    Ok(vec![
                        Content::text(content).with_audience(vec![Role::Assistant])
                    ])
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
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
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}

impl Clone for TutorialRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            instructions: self.instructions.clone(),
        }
    }
}
