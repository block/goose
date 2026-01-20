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
    ListToolsResult, Meta, ProtocolVersion, RawResource, ReadResourceResult, Resource,
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

/// Parameters for list_apps tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ListAppsParams {
    // No parameters needed - lists all apps
}

/// Response from create_app_content tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateAppContentResponse {
    /// App name (lowercase, hyphens allowed, no spaces)
    name: String,
    /// Brief description of what the app does (1-2 sentences, max 100 chars)
    description: String,
    /// Complete HTML code for the app, from <!DOCTYPE html> to </html>
    html: String,
    /// Window width in pixels (recommended: 400-1600)
    width: Option<u32>,
    /// Window height in pixels (recommended: 300-1200)
    height: Option<u32>,
    /// Whether the window should be resizable
    resizable: Option<bool>,
}

/// Response from update_app_content tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UpdateAppContentResponse {
    /// Updated description of what the app does (1-2 sentences, max 100 chars)
    description: String,
    /// Complete updated HTML code for the app, from <!DOCTYPE html> to </html>
    html: String,
    /// Updated PRD reflecting the current state of the app after this iteration
    prd: String,
    /// Updated window width in pixels (optional - only if size should change)
    width: Option<u32>,
    /// Updated window height in pixels (optional - only if size should change)
    height: Option<u32>,
    /// Updated resizable property (optional - only if it should change)
    resizable: Option<bool>,
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
                "Use this extension to create, manage, and iterate on custom HTML/CSS/JavaScript apps."
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

        // Always ensure the clock app exists
        if !apps.contains(&"clock".to_string()) {
            let clock_html = include_str!("../goose_apps/clock.html");
            let clock_app = GooseApp {
                resource: McpAppResource {
                    uri: "ui://apps/clock".to_string(),
                    name: "clock".to_string(),
                    description: Some("Swiss Railway Clock".to_string()),
                    mime_type: "text/html;profile=mcp-app".to_string(),
                    text: Some(clock_html.to_string()),
                    blob: None,
                    meta: None,
                },
                mcp_server: Some("apps".to_string()),
                window_props: None,
                prd: Some("An analog clock widget inspired by the iconic Swiss railway clock (Hans Hilfiker design). Features smooth-sweeping hands with the characteristic pause-and-jump behavior at 12 o'clock.".to_string()),
            };
            self.save_app(&clock_app)?;
            tracing::info!("Created default clock app");
        }

        Ok(())
    }

    fn list_stored_apps(&self) -> Result<Vec<String>, String> {
        let mut apps = Vec::new();

        let entries = fs::read_dir(&self.apps_dir)
            .map_err(|e| format!("Failed to read apps directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("html") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    apps.push(stem.to_string());
                }
            }
        }

        apps.sort();
        Ok(apps)
    }

    fn load_app(&self, name: &str) -> Result<GooseApp, String> {
        let path = self.apps_dir.join(format!("{}.html", name));

        let html =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read app file: {}", e))?;

        GooseApp::from_html(&html)
    }

    fn save_app(&self, app: &GooseApp) -> Result<(), String> {
        let path = self.apps_dir.join(format!("{}.html", app.resource.name));

        let html_content = app.to_html()?;

        fs::write(&path, html_content).map_err(|e| format!("Failed to write app file: {}", e))?;

        Ok(())
    }

    fn delete_app(&self, name: &str) -> Result<(), String> {
        let path = self.apps_dir.join(format!("{}.html", name));

        fs::remove_file(&path).map_err(|e| format!("Failed to delete app file: {}", e))?;

        Ok(())
    }

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

    fn schema<T: JsonSchema>() -> JsonObject {
        serde_json::to_value(schema_for!(T))
            .map(|v| v.as_object().unwrap().clone())
            .expect("valid schema")
    }

    fn create_app_content_tool() -> rmcp::model::Tool {
        rmcp::model::Tool::new(
            "create_app_content".to_string(),
            "Generate content for a new Goose app. Returns the HTML code, app name, description, and window properties.".to_string(),
            Self::schema::<CreateAppContentResponse>(),
        )
    }

    fn update_app_content_tool() -> rmcp::model::Tool {
        rmcp::model::Tool::new(
            "update_app_content".to_string(),
            "Generate updated content for an existing Goose app. Returns the improved HTML code, updated description, and optionally updated window properties.".to_string(),
            Self::schema::<UpdateAppContentResponse>(),
        )
    }

    async fn generate_new_app_content(
        &self,
        prd: &str,
    ) -> Result<CreateAppContentResponse, String> {
        let provider = self.get_provider().await?;

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

WINDOW SIZING:
- Choose appropriate width and height based on the app's content and layout
- Typical sizes: small utilities (400x300), standard apps (800x600), large apps (1200x800)
- Set resizable to false for fixed-size apps, true for flexible layouts

You must call the create_app_content tool to return the app name, description, HTML, and window properties."#;

        let user_prompt = format!(
            "REQUESTED APP:\n{}\n\nEXISTING APPS: {}\n\nGenerate a unique name (lowercase with hyphens, not in existing apps), a brief description, complete HTML, and appropriate window size for this app.",
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
                        let params = tool_call
                            .arguments
                            .as_ref()
                            .ok_or("Missing tool call parameters")?;

                        let response: CreateAppContentResponse =
                            serde_json::from_value(serde_json::Value::Object(params.clone()))
                                .map_err(|e| format!("Failed to parse tool response: {}", e))?;

                        return Ok(response);
                    }
                }
            }
        }

        Err("LLM did not call the required tool".to_string())
    }

    /// Generate updated content for an existing app using the LLM with tool calling
    async fn generate_updated_app_content(
        &self,
        existing_html: &str,
        existing_prd: &str,
        feedback: &str,
    ) -> Result<UpdateAppContentResponse, String> {
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

WINDOW SIZING:
- Optionally update width/height if the changes warrant a different window size
- Only include size properties if they should change
- Set resizable to false for fixed-size apps, true for flexible layouts

PRD UPDATE:
- Update the PRD to reflect the current state of the app after implementing the feedback
- Keep the core requirements but add/update sections based on what was actually changed
- Document new features, changed behavior, or updated requirements
- Keep the PRD concise and focused on what the app should do, not implementation details

You must call the update_app_content tool to return the updated description, HTML, updated PRD, and optionally updated window properties."#;

        let user_prompt = format!(
            "ORIGINAL PRD:\n{}\n\nCURRENT APP:\n```html\n{}\n```\n\nFEEDBACK: {}\n\nImplement the requested changes and return:\n1. Updated description\n2. Updated HTML implementing the feedback\n3. Updated PRD reflecting the current state of the app\n4. Optionally updated window size if appropriate",
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
                        let params = tool_call
                            .arguments
                            .as_ref()
                            .ok_or("Missing tool call parameters")?;

                        let response: UpdateAppContentResponse =
                            serde_json::from_value(serde_json::Value::Object(params.clone()))
                                .map_err(|e| format!("Failed to parse tool response: {}", e))?;

                        return Ok(response);
                    }
                }
            }
        }

        Err("LLM did not call the required tool".to_string())
    }

    /// Handle list_apps tool call
    async fn handle_list_apps(
        &self,
        _arguments: Option<JsonObject>,
        _meta: McpMeta,
    ) -> Result<CallToolResult, String> {
        let app_names = self.list_stored_apps()?;

        if app_names.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No apps found. Create your first app with the create_app tool!".to_string(),
            )]));
        }

        let mut apps_info = vec![format!("Found {} app(s):\n", app_names.len())];

        for name in app_names {
            match self.load_app(&name) {
                Ok(app) => {
                    let description = app
                        .resource
                        .description
                        .as_deref()
                        .unwrap_or("No description");

                    let size = if let Some(ref props) = app.window_props {
                        format!(" ({}x{})", props.width, props.height)
                    } else {
                        String::new()
                    };

                    apps_info.push(format!("- {}{}: {}", name, size, description));
                }
                Err(e) => {
                    apps_info.push(format!("- {}: (error loading: {})", name, e));
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            apps_info.join("\n"),
        )]))
    }

    async fn handle_create_app(
        &self,
        arguments: Option<JsonObject>,
        _meta: McpMeta,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let prd = extract_string(&args, "prd")?;

        // Generate app content using LLM with tool calling
        let content = self.generate_new_app_content(&prd).await?;
        tracing::info!("LLM generated app name: '{}'", content.name);

        // Check if app already exists
        if self.load_app(&content.name).is_ok() {
            return Err(format!(
                "App '{}' already exists (generated name conflicts with existing app).",
                content.name
            ));
        }

        let app = GooseApp {
            resource: McpAppResource {
                uri: format!("ui://apps/{}", content.name),
                name: content.name.clone(),
                description: Some(content.description),
                mime_type: "text/html;profile=mcp-app".to_string(),
                text: Some(content.html),
                blob: None,
                meta: None,
            },
            mcp_server: Some("apps".to_string()),
            window_props: Some(crate::goose_apps::WindowProps {
                width: content.width.unwrap_or(800),
                height: content.height.unwrap_or(600),
                resizable: content.resizable.unwrap_or(true),
            }),
            prd: Some(prd),
        };

        self.save_app(&app)?;
        tracing::info!("Saved app with name: '{}'", content.name);

        let result = CallToolResult::success(vec![Content::text(format!(
            "Created app '{}'! It should have automatically opened in a new window. You can always find it again in the [Apps] tab.",
            content.name
        ))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(content.name));
        tracing::info!("Sending platform notification for app: '{}'", content.name);

        let result =
            self.context
                .result_with_platform_notification(result, "apps", "app_created", params);

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
        let content = self
            .generate_updated_app_content(existing_html, existing_prd, &feedback)
            .await?;

        // Update app with new content
        app.resource.text = Some(content.html);
        app.resource.description = Some(content.description);

        // Update PRD from LLM response - keeps HTML and PRD in sync
        app.prd = Some(content.prd);

        // Update window properties if provided
        if content.width.is_some() || content.height.is_some() || content.resizable.is_some() {
            let current_props = app.window_props.as_ref();
            let default_width = current_props.map(|p| p.width).unwrap_or(800);
            let default_height = current_props.map(|p| p.height).unwrap_or(600);
            let default_resizable = current_props.map(|p| p.resizable).unwrap_or(true);

            app.window_props = Some(crate::goose_apps::WindowProps {
                width: content.width.unwrap_or(default_width),
                height: content.height.unwrap_or(default_height),
                resizable: content.resizable.unwrap_or(default_resizable),
            });
        }

        self.save_app(&app)?;

        let result = CallToolResult::success(vec![Content::text(format!(
            "Updated app '{}' based on your feedback",
            name
        ))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(name));

        let result =
            self.context
                .result_with_platform_notification(result, "apps", "app_updated", params);

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

        let result =
            CallToolResult::success(vec![Content::text(format!("Deleted app '{}'", name))]);

        // Add platform notification
        let mut params = serde_json::Map::new();
        params.insert("app_name".to_string(), json!(name));

        let result =
            self.context
                .result_with_platform_notification(result, "apps", "app_deleted", params);

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
                "list_apps".to_string(),
                "List all available Goose apps with their names and descriptions. Use this to see what apps exist before creating or modifying apps.".to_string(),
                schema::<ListAppsParams>(),
            ),
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
            "list_apps" => self.handle_list_apps(arguments, meta).await,
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
                // Build meta with window properties if available
                let meta = if let Some(ref window_props) = app.window_props {
                    let mut meta_obj = Meta::new();
                    meta_obj.insert(
                        "window".to_string(),
                        json!({
                            "width": window_props.width,
                            "height": window_props.height,
                            "resizable": window_props.resizable,
                        }),
                    );
                    Some(meta_obj)
                } else {
                    None
                };

                let raw_resource = RawResource {
                    uri: app.resource.uri.clone(),
                    name: app.resource.name.clone(),
                    title: None,
                    description: app.resource.description.clone(),
                    mime_type: Some(app.resource.mime_type.clone()),
                    size: None,
                    icons: None,
                    meta,
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

        let app = self
            .load_app(app_name)
            .map_err(|_| Error::TransportClosed)?;

        // Return the clean HTML without embedded metadata
        // The metadata (window props, PRD) is exposed via list_resources meta field
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

/// Extract a string from JSON arguments
fn extract_string(args: &JsonObject, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing or invalid '{}'", key))
}
