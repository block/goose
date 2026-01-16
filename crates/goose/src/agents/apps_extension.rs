use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait, McpMeta};
use crate::config::paths::Paths;
use crate::conversation::message::Message;
use crate::goose_apps::GooseApp;
use crate::goose_apps::McpAppResource;
use crate::providers::base::Provider;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListResourcesResult,
    ListToolsResult, ProtocolVersion, RawResource, ReadResourceResult, Resource,
    ResourceContents, ResourcesCapability, ServerCapabilities, Tool as McpTool, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "apps";

/// Parameters for create_app tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateAppParams {
    /// What the app should do - a description or PRD that will be used to generate the app
    prd: String,
}

/// Parameters for iterate_app tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct IterateAppParams {
    /// Name of the app to iterate on
    name: String,
    /// Feedback or requested changes to improve the app
    feedback: String,
}

/// Parameters for delete_app tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DeleteAppParams {
    /// Name of the app to delete
    name: String,
}

pub struct AppsManagerClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    apps_dir: PathBuf,
}

impl AppsManagerClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self, String> {
        let apps_dir = Paths::in_data_dir("apps");

        // Ensure apps directory exists
        fs::create_dir_all(&apps_dir)
            .map_err(|e| format!("Failed to create apps directory: {}", e))?;

        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Apps Manager".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Use this extension to create, manage, and iterate on custom HTML/CSS/JavaScript apps. \
                Apps are stored locally and can be viewed in the Apps section or launched in standalone windows. \
                \n\nTools:\n\
                - create_app: Create a new app\n\
                - update_app: Update an existing app's HTML or metadata\n\
                - delete_app: Delete an app\n\n\
                All apps are automatically exposed as ui://apps/{name} resources."
                    .to_string(),
            ),
        };

        let mut client = Self {
            info,
            context,
            apps_dir,
        };

        if let Err(e) = client.ensure_default_apps() {
            tracing::warn!("Failed to create default apps: {}", e);
        }

        Ok(client)
    }

    /// Ensure default apps exist (like the clock)
    fn ensure_default_apps(&mut self) -> Result<(), String> {
        let apps = self.list_stored_apps()?;

        // If no apps exist, create the default clock app
        if apps.is_empty() {
            let clock_html = include_str!("../../resources/clock.html");
            let clock_app = GooseApp {
                resource: McpAppResource {
                    uri: "ui://apps/clock".to_string(),
                    name: "clock".to_string(),
                    description: Some("A beautiful clock with multiple design themes (Digital, Analog, Swiss Railway)".to_string()),
                    mime_type: "text/html;profile=mcp-app".to_string(),
                    text: Some(clock_html.to_string()),
                    blob: None,
                    meta: None,
                },
                mcp_server: Some("apps".to_string()),
                window_props: None,
                prd: Some("A clock app with three iconic design themes: Casio digital, Braun analog, and Swiss Railway. Users can switch between themes.".to_string()),
            };
            self.save_app(&clock_app)?;
            tracing::info!("Created default clock app");
        }

        Ok(())
    }

    /// List all stored apps
    fn list_stored_apps(&self) -> Result<Vec<String>, String> {
        let mut apps = Vec::new();

        let entries = fs::read_dir(&self.apps_dir)
            .map_err(|e| format!("Failed to read apps directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    apps.push(stem.to_string());
                }
            }
        }

        apps.sort();
        Ok(apps)
    }

    /// Load an app from disk
    fn load_app(&self, name: &str) -> Result<GooseApp, String> {
        let path = self.apps_dir.join(format!("{}.json", name));

        if !path.exists() {
            return Err(format!("App '{}' not found", name));
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read app file: {}", e))?;

        let app: GooseApp = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse app JSON: {}", e))?;

        Ok(app)
    }

    /// Save an app to disk
    fn save_app(&self, app: &GooseApp) -> Result<(), String> {
        // Validate app name
        let app_name = &app.resource.name;
        if !is_valid_app_name(app_name) {
            return Err(format!(
                "Invalid app name '{}'. Use lowercase letters, numbers, and hyphens only.",
                app_name
            ));
        }

        let path = self.apps_dir.join(format!("{}.json", app_name));

        let json = serde_json::to_string_pretty(app)
            .map_err(|e| format!("Failed to serialize app: {}", e))?;

        fs::write(&path, json)
            .map_err(|e| format!("Failed to write app file: {}", e))?;

        Ok(())
    }

    /// Delete an app from disk
    fn delete_app(&self, name: &str) -> Result<(), String> {
        let path = self.apps_dir.join(format!("{}.json", name));

        if !path.exists() {
            return Err(format!("App '{}' not found", name));
        }

        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete app file: {}", e))?;

        Ok(())
    }

    /// Get provider from extension manager
    async fn get_provider(&self) -> Result<Arc<dyn Provider>, String> {
        let extension_manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or("Extension manager not available")?;

        let provider_guard = extension_manager.get_provider().lock().await;

        let provider = provider_guard
            .as_ref()
            .ok_or("Provider not available")?
            .clone();

        Ok(provider)
    }

    /// Tool schema for creating a new app (returns name, description, HTML)
    fn create_app_content_tool() -> rmcp::model::Tool {
        use rmcp::object;

        rmcp::model::Tool::new(
            "create_app_content".to_string(),
            "Generate content for a new Goose app. Returns the HTML code, app name, and description.".to_string(),
            object!({
                "type": "object",
                "required": ["name", "description", "html"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "App name (lowercase, hyphens allowed, no spaces). Must be unique and not in the existing apps list."
                    },
                    "description": {
                        "type": "string",
                        "description": "Brief description of what the app does (1-2 sentences, max 100 chars)"
                    },
                    "html": {
                        "type": "string",
                        "description": "Complete HTML code for the app, from <!DOCTYPE html> to </html>"
                    }
                }
            }),
        )
    }

    /// Tool schema for updating an existing app (returns description, HTML)
    fn update_app_content_tool() -> rmcp::model::Tool {
        use rmcp::object;

        rmcp::model::Tool::new(
            "update_app_content".to_string(),
            "Generate updated content for an existing Goose app. Returns the improved HTML code and updated description.".to_string(),
            object!({
                "type": "object",
                "required": ["description", "html"],
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Updated description of what the app does (1-2 sentences, max 100 chars)"
                    },
                    "html": {
                        "type": "string",
                        "description": "Complete updated HTML code for the app, from <!DOCTYPE html> to </html>"
                    }
                }
            }),
        )
    }

    /// Generate content for a new app using the LLM with tool calling
    async fn generate_new_app_content(&self, prd: &str) -> Result<(String, String, String), String> {
        let provider = self.get_provider().await?;

        // Get list of existing app names
        let existing_apps = self.list_stored_apps().unwrap_or_default();
        let existing_names = existing_apps.join(", ");

        let system_prompt = r#"You are an expert HTML/CSS/JavaScript developer. Generate standalone, single-file HTML applications.

REQUIREMENTS:
- Create a complete, self-contained HTML file with embedded CSS and JavaScript
- Use modern, clean design with good UX
- Make it responsive and work well in different window sizes
- Use semantic HTML5
- Add appropriate error handling
- Make the app interactive and functional
- Use vanilla JavaScript (no external dependencies unless absolutely necessary)
- If you need external resources (fonts, icons), use CDN links
- The app will be sandboxed with strict CSP, so all scripts must be inline or from trusted CDNs

You must call the create_app_content tool to return the app name, description, and HTML."#;

        let user_prompt = format!(
            "REQUESTED APP:\n{}\n\nEXISTING APPS: {}\n\nGenerate a unique name (lowercase with hyphens, not in existing apps), a brief description, and complete HTML for this app.",
            prd,
            if existing_names.is_empty() { "none" } else { &existing_names }
        );

        let messages = vec![Message::user().with_text(&user_prompt)];
        let tools = vec![Self::create_app_content_tool()];

        let (response, _usage) = provider
            .complete(system_prompt, &messages, &tools)
            .await
            .map_err(|e| format!("LLM call failed: {}", e))?;

        // Extract tool call from response
        for content in &response.content {
            if let crate::conversation::message::MessageContent::ToolRequest(tool_req) = content {
                if let Ok(tool_call) = &tool_req.tool_call {
                    if tool_call.name == "create_app_content" {
                        let params = tool_call.arguments
                            .as_ref()
                            .ok_or("Missing tool call parameters")?;

                        let name = params.get("name")
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .ok_or("Missing 'name' in tool call")?
                            .to_string();

                        let description = params.get("description")
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .ok_or("Missing 'description' in tool call")?
                            .to_string();

                        let html = params.get("html")
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .ok_or("Missing 'html' in tool call")?
                            .to_string();

                        return Ok((name, description, html));
                    }
                }
            }
        }

        Err("LLM did not call the required tool".to_string())
    }

    /// Generate updated content for an existing app using the LLM with tool calling
    async fn generate_updated_app_content(&self, existing_html: &str, existing_prd: &str, feedback: &str) -> Result<(String, String), String> {
        let provider = self.get_provider().await?;

        let system_prompt = r#"You are an expert HTML/CSS/JavaScript developer. Generate standalone, single-file HTML applications.

REQUIREMENTS:
- Create a complete, self-contained HTML file with embedded CSS and JavaScript
- Use modern, clean design with good UX
- Make it responsive and work well in different window sizes
- Use semantic HTML5
- Add appropriate error handling
- Make the app interactive and functional
- Use vanilla JavaScript (no external dependencies unless absolutely necessary)
- If you need external resources (fonts, icons), use CDN links
- The app will be sandboxed with strict CSP, so all scripts must be inline or from trusted CDNs

You must call the update_app_content tool to return the updated description and HTML."#;

        let user_prompt = format!(
            "ORIGINAL PRD:\n{}\n\nCURRENT APP:\n```html\n{}\n```\n\nFEEDBACK: {}\n\nGenerate an improved version with an updated description and HTML that addresses the feedback while preserving the app's core functionality.",
            existing_prd,
            existing_html,
            feedback
        );

        let messages = vec![Message::user().with_text(&user_prompt)];
        let tools = vec![Self::update_app_content_tool()];

        let (response, _usage) = provider
            .complete(system_prompt, &messages, &tools)
            .await
            .map_err(|e| format!("LLM call failed: {}", e))?;

        // Extract tool call from response
        for content in &response.content {
            if let crate::conversation::message::MessageContent::ToolRequest(tool_req) = content {
                if let Ok(tool_call) = &tool_req.tool_call {
                    if tool_call.name == "update_app_content" {
                        let params = tool_call.arguments
                            .as_ref()
                            .ok_or("Missing tool call parameters")?;

                        let description = params.get("description")
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .ok_or("Missing 'description' in tool call")?
                            .to_string();

                        let html = params.get("html")
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .ok_or("Missing 'html' in tool call")?
                            .to_string();

                        return Ok((description, html));
                    }
                }
            }
        }

        Err("LLM did not call the required tool".to_string())
    }

    /// Handle create_app tool call
    async fn handle_create_app(
        &self,
        arguments: Option<JsonObject>,
        _meta: McpMeta,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let prd = extract_string(&args, "prd")?;

        // Generate app content using LLM with tool calling
        let (name, description, html) = self.generate_new_app_content(&prd).await?;

        // Validate the generated name
        if !is_valid_app_name(&name) {
            return Err(format!(
                "LLM generated invalid app name '{}'. App names must be lowercase with hyphens only.",
                name
            ));
        }

        // Check if app already exists
        if self.load_app(&name).is_ok() {
            return Err(format!(
                "App '{}' already exists (generated name conflicts with existing app).",
                name
            ));
        }

        let app = GooseApp {
            resource: McpAppResource {
                uri: format!("ui://apps/{}", name),
                name: name.clone(),
                description: Some(description),
                mime_type: "text/html;profile=mcp-app".to_string(),
                text: Some(html),
                blob: None,
                meta: None,
            },
            mcp_server: Some("apps".to_string()),
            window_props: None,
            prd: Some(prd),
        };

        self.save_app(&app)?;

        let result = CallToolResult::success(vec![Content::text(format!(
            "Created app '{}'. You can view it in the Apps section or open it with the resource uri: ui://apps/{}",
            name, name
        ))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(name));

        let result = self.context.result_with_platform_notification(
            result,
            "apps",
            "app_created",
            params,
        );

        Ok(result)
    }

    /// Handle iterate_app tool call
    async fn handle_iterate_app(
        &self,
        arguments: Option<JsonObject>,
        _meta: McpMeta,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;

        let name = extract_string(&args, "name")?;
        let feedback = extract_string(&args, "feedback")?;

        let mut app = self.load_app(&name)?;

        // Get existing HTML
        let existing_html = app
            .resource
            .text
            .as_deref()
            .ok_or("App has no HTML content")?;

        // Get existing PRD
        let existing_prd = app.prd.as_deref().unwrap_or("");

        // Generate updated content using LLM with tool calling
        let (description, html) = self
            .generate_updated_app_content(existing_html, existing_prd, &feedback)
            .await?;

        // Update app with new content
        app.resource.text = Some(html);
        app.resource.description = Some(description);

        // Optionally update PRD with feedback
        if let Some(ref mut prd) = app.prd {
            prd.push_str(&format!("\n\nIteration feedback: {}", feedback));
        }

        self.save_app(&app)?;

        let result = CallToolResult::success(vec![Content::text(format!(
            "Updated app '{}' based on your feedback",
            name
        ))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(name));

        let result = self.context.result_with_platform_notification(
            result,
            "apps",
            "app_updated",
            params,
        );

        Ok(result)
    }

    /// Handle delete_app tool call
    async fn handle_delete_app(
        &self,
        arguments: Option<JsonObject>,
        _meta: McpMeta,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;

        let name = extract_string(&args, "name")?;

        self.delete_app(&name)?;

        let result = CallToolResult::success(vec![Content::text(format!("Deleted app '{}'", name))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(name));

        let result = self.context.result_with_platform_notification(
            result,
            "apps",
            "app_deleted",
            params,
        );

        Ok(result)
    }
}

#[async_trait]
impl McpClientTrait for AppsManagerClient {
    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        fn schema<T: JsonSchema>() -> JsonObject {
            serde_json::to_value(schema_for!(T))
                .map(|v| v.as_object().unwrap().clone())
                .expect("valid schema")
        }

        let tools = vec![
            McpTool::new(
                "create_app".to_string(),
                "Create a new Goose app based on a description or PRD. The extension will use an LLM to generate the HTML/CSS/JavaScript. Apps are sandboxed and run in standalone windows.".to_string(),
                schema::<CreateAppParams>(),
            ),
            McpTool::new(
                "iterate_app".to_string(),
                "Improve an existing app based on feedback. The extension will use an LLM to update the HTML while preserving the app's intent.".to_string(),
                schema::<IterateAppParams>(),
            ),
            McpTool::new(
                "delete_app".to_string(),
                "Delete an app permanently".to_string(),
                schema::<DeleteAppParams>(),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        meta: McpMeta,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            "create_app" => self.handle_create_app(arguments, meta).await,
            "iterate_app" => self.handle_iterate_app(arguments, meta).await,
            "delete_app" => self.handle_delete_app(arguments, meta).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match result {
            Ok(result) => Ok(result),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        let app_names = self
            .list_stored_apps()
            .map_err(|_| Error::TransportClosed)?;

        let mut resources = Vec::new();

        for name in app_names {
            if let Ok(app) = self.load_app(&name) {
                let raw_resource = RawResource {
                    uri: app.resource.uri.clone(),
                    name: app.resource.name.clone(),
                    title: None,
                    description: app.resource.description.clone(),
                    mime_type: Some(app.resource.mime_type.clone()),
                    size: None,
                    icons: None,
                    meta: None,
                };
                resources.push(Resource {
                    raw: raw_resource,
                    annotations: None,
                });
            }
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        uri: &str,
        _cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        // Parse app name from URI (ui://apps/{name})
        let app_name = uri
            .strip_prefix("ui://apps/")
            .ok_or(Error::TransportClosed)?;

        let app = self.load_app(app_name).map_err(|_| Error::TransportClosed)?;

        let html = app
            .resource
            .text
            .unwrap_or_else(|| String::from("No content"));

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(html, uri)],
        })
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}

/// Validate app name (lowercase, numbers, hyphens only)
fn is_valid_app_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// Extract a string from JSON arguments
fn extract_string(args: &JsonObject, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing or invalid '{}'", key))
}
