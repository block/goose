pub mod resource;

use crate::agents::ExtensionManager;
use crate::config::paths::Paths;
use rmcp::model::ErrorData;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use utoipa::ToSchema;

pub use resource::{CspMetadata, McpAppResource, ResourceMetadata, UiMetadata};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WindowProps {
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}

/// A Goose App combining MCP resource data with Goose-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GooseApp {
    #[serde(flatten)]
    pub resource: McpAppResource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub window_props: Option<WindowProps>,
    /// Product requirements document for LLM-based iteration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd: Option<String>,
}

impl GooseApp {
    const METADATA_SCRIPT_TYPE: &'static str = "application/ld+json";
    const PRD_SCRIPT_TYPE: &'static str = "application/x-goose-prd";
    const GOOSE_APP_TYPE: &'static str = "GooseApp";
    const GOOSE_SCHEMA_CONTEXT: &'static str = "https://goose.ai/schema";

    /// Parse a GooseApp from HTML with embedded metadata
    pub fn from_html(html: &str) -> Result<Self, String> {
        use regex::Regex;

        let metadata_re = Regex::new(&format!(
            r#"(?s)<script type="{}"[^>]*>\s*(.*?)\s*</script>"#,
            regex::escape(Self::METADATA_SCRIPT_TYPE)
        ))
        .map_err(|e| format!("Regex error: {}", e))?;

        let prd_re = Regex::new(&format!(
            r#"(?s)<script type="{}"[^>]*>\s*(.*?)\s*</script>"#,
            regex::escape(Self::PRD_SCRIPT_TYPE)
        ))
        .map_err(|e| format!("Regex error: {}", e))?;

        // Extract metadata JSON
        let json_str = metadata_re
            .captures(html)
            .and_then(|cap| cap.get(1))
            .ok_or_else(|| "No GooseApp JSON-LD metadata found in HTML".to_string())?
            .as_str();

        let metadata: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;

        // Extract fields from metadata
        let name = metadata
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'name' in metadata")?
            .to_string();

        let description = metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let width = metadata
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let height = metadata
            .get("height")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let resizable = metadata.get("resizable").and_then(|v| v.as_bool());

        let window_props = if width.is_some() || height.is_some() || resizable.is_some() {
            Some(WindowProps {
                width: width.unwrap_or(800),
                height: height.unwrap_or(600),
                resizable: resizable.unwrap_or(true),
            })
        } else {
            None
        };

        let mcp_server = metadata
            .get("mcpServer")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract PRD
        let prd = prd_re
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string());

        // Strip metadata and PRD scripts from HTML
        let clean_html = metadata_re.replace(html, "");
        let clean_html = prd_re.replace(&clean_html, "").to_string();

        Ok(GooseApp {
            resource: McpAppResource {
                uri: format!("ui://apps/{}", name),
                name,
                description,
                mime_type: "text/html;profile=mcp-app".to_string(),
                text: Some(clean_html),
                blob: None,
                meta: None,
            },
            mcp_server,
            window_props,
            prd,
        })
    }

    /// Convert GooseApp to HTML with embedded metadata
    pub fn to_html(&self) -> Result<String, String> {
        let html = self
            .resource
            .text
            .as_ref()
            .ok_or("App has no HTML content")?;

        // Build metadata JSON
        let mut metadata = serde_json::json!({
            "@context": Self::GOOSE_SCHEMA_CONTEXT,
            "@type": Self::GOOSE_APP_TYPE,
            "name": self.resource.name,
        });

        if let Some(ref desc) = self.resource.description {
            metadata["description"] = serde_json::json!(desc);
        }

        if let Some(ref props) = self.window_props {
            metadata["width"] = serde_json::json!(props.width);
            metadata["height"] = serde_json::json!(props.height);
            metadata["resizable"] = serde_json::json!(props.resizable);
        }

        if let Some(ref server) = self.mcp_server {
            metadata["mcpServer"] = serde_json::json!(server);
        }

        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        let metadata_script = format!(
            "  <script type=\"{}\">\n{}\n  </script>",
            Self::METADATA_SCRIPT_TYPE,
            metadata_json
        );

        let prd_script = if let Some(ref prd) = self.prd {
            if !prd.is_empty() {
                format!(
                    "  <script type=\"{}\">\n{}\n  </script>",
                    Self::PRD_SCRIPT_TYPE,
                    prd
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let scripts = if prd_script.is_empty() {
            format!("{}\n", metadata_script)
        } else {
            format!("{}\n{}\n", metadata_script, prd_script)
        };

        // Insert scripts into HTML
        let result = if let Some(head_pos) = html.find("</head>") {
            let mut result = html.clone();
            result.insert_str(head_pos, &scripts);
            result
        } else if let Some(html_pos) = html.find("<html") {
            let after_html = html
                .get(html_pos..)
                .and_then(|s| s.find('>'))
                .map(|p| html_pos + p + 1);
            if let Some(pos) = after_html {
                let mut result = html.clone();
                result.insert_str(pos, &format!("\n<head>\n{}</head>", scripts));
                result
            } else {
                format!("<head>\n{}</head>\n{}", scripts, html)
            }
        } else {
            format!(
                "<html>\n<head>\n{}</head>\n<body>\n{}\n</body>\n</html>",
                scripts, html
            )
        };

        Ok(result)
    }
}

pub struct McpAppCache {
    cache_dir: PathBuf,
}

impl McpAppCache {
    pub fn new() -> Result<Self, std::io::Error> {
        let config_dir = Paths::config_dir();
        let cache_dir = config_dir.join("mcp-apps-cache");
        Ok(Self { cache_dir })
    }

    fn cache_key(extension_name: &str, resource_uri: &str) -> String {
        let input = format!("{}::{}", extension_name, resource_uri);
        let hash = Sha256::digest(input.as_bytes());
        format!("{}_{:x}", extension_name, hash)
    }

    pub fn list_apps(&self) -> Result<Vec<GooseApp>, std::io::Error> {
        let mut apps = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(apps);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<GooseApp>(&content) {
                        Ok(app) => apps.push(app),
                        Err(e) => warn!("Failed to parse cached app from {:?}: {}", path, e),
                    },
                    Err(e) => warn!("Failed to read cached app from {:?}: {}", path, e),
                }
            }
        }

        Ok(apps)
    }

    pub fn store_app(&self, app: &GooseApp) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_dir)?;

        if let Some(ref extension_name) = app.mcp_server {
            let cache_key = Self::cache_key(extension_name, &app.resource.uri);
            let app_path = self.cache_dir.join(format!("{}.json", cache_key));
            let json = serde_json::to_string_pretty(app).map_err(std::io::Error::other)?;
            fs::write(app_path, json)?;
        }

        Ok(())
    }

    pub fn get_app(&self, extension_name: &str, resource_uri: &str) -> Option<GooseApp> {
        let cache_key = Self::cache_key(extension_name, resource_uri);
        let app_path = self.cache_dir.join(format!("{}.json", cache_key));

        if !app_path.exists() {
            return None;
        }

        fs::read_to_string(&app_path)
            .ok()
            .and_then(|content| serde_json::from_str::<GooseApp>(&content).ok())
    }

    pub fn delete_extension_apps(&self, extension_name: &str) -> Result<usize, std::io::Error> {
        let mut deleted_count = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(app) = serde_json::from_str::<GooseApp>(&content) {
                        if app.mcp_server.as_deref() == Some(extension_name)
                            && fs::remove_file(&path).is_ok()
                        {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

pub async fn fetch_mcp_apps(
    extension_manager: &ExtensionManager,
) -> Result<Vec<GooseApp>, ErrorData> {
    let mut apps = Vec::new();

    let ui_resources = extension_manager.get_ui_resources().await?;

    for (extension_name, resource) in ui_resources {
        match extension_manager
            .read_resource(&resource.uri, &extension_name, CancellationToken::default())
            .await
        {
            Ok(read_result) => {
                let mut html = String::new();
                for content in read_result.contents {
                    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } =
                        content
                    {
                        html = text;
                        break;
                    }
                }

                if !html.is_empty() {
                    let mcp_resource = McpAppResource {
                        uri: resource.uri.clone(),
                        name: resource.name.clone(),
                        description: resource.description.clone(),
                        mime_type: "text/html;profile=mcp-app".to_string(),
                        text: Some(html),
                        blob: None,
                        meta: None,
                    };

                    // Extract window properties from resource meta.window if present
                    let window_props = if let Some(ref meta) = resource.meta {
                        if let Some(window_obj) = meta.get("window").and_then(|v| v.as_object()) {
                            if let (Some(width), Some(height), Some(resizable)) = (
                                window_obj
                                    .get("width")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32),
                                window_obj
                                    .get("height")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32),
                                window_obj.get("resizable").and_then(|v| v.as_bool()),
                            ) {
                                Some(WindowProps {
                                    width,
                                    height,
                                    resizable,
                                })
                            } else {
                                // Window object exists but doesn't have complete props
                                Some(WindowProps {
                                    width: 800,
                                    height: 600,
                                    resizable: true,
                                })
                            }
                        } else {
                            // Meta exists but no window object - use defaults
                            Some(WindowProps {
                                width: 800,
                                height: 600,
                                resizable: true,
                            })
                        }
                    } else {
                        // No meta - use defaults
                        Some(WindowProps {
                            width: 800,
                            height: 600,
                            resizable: true,
                        })
                    };

                    let app = GooseApp {
                        resource: mcp_resource,
                        mcp_server: Some(extension_name),
                        window_props,
                        prd: None,
                    };

                    apps.push(app);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read resource {} from {}: {}",
                    resource.uri, extension_name, e
                );
            }
        }
    }

    Ok(apps)
}
