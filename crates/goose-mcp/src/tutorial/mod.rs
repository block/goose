use anyhow::Result;
use include_dir::{include_dir, Dir};
use indoc::formatdoc;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, Role, ServerCapabilities,
        ServerInfo,
    },
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};

static TUTORIALS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/tutorial/tutorials");

/// Parameters for the load_tutorial tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LoadTutorialParams {
    /// Name of the tutorial to load, e.g. 'getting-started' or 'developer-mcp'
    pub name: String,
}

/// Tutorial MCP Server using official RMCP SDK
#[derive(Debug)]
pub struct TutorialServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TutorialServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-tutorial".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

impl Default for TutorialServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl TutorialServer {
    pub fn new() -> Self {
        // Get available tutorials
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
            tool_router: Self::tool_router(),
            instructions,
        }
    }

    /// Load a specific tutorial by name. The tutorial will be returned as markdown content that provides step by step instructions.
    #[tool(
        name = "load_tutorial",
        description = "Load a specific tutorial by name. The tutorial will be returned as markdown content that provides step by step instructions."
    )]
    pub async fn load_tutorial(
        &self,
        params: Parameters<LoadTutorialParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let name = &params.name;

        let content = self.load_tutorial_content(name).await?;

        Ok(CallToolResult::success(vec![
            Content::text(content).with_audience(vec![Role::Assistant])
        ]))
    }

    /// Get list of available tutorials with descriptions
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

    /// Load tutorial content by name
    async fn load_tutorial_content(&self, name: &str) -> Result<String, ErrorData> {
        let file_name = format!("{}.md", name);
        let file = TUTORIALS_DIR.get_file(&file_name).ok_or(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Could not locate tutorial '{}'", name),
            None,
        ))?;
        Ok(String::from_utf8_lossy(file.contents()).into_owned())
    }
}

impl Clone for TutorialServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            instructions: self.instructions.clone(),
        }
    }
}
