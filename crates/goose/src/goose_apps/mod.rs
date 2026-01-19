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
                        name: format_resource_name(resource.name.clone()),
                        description: resource.description.clone(),
                        mime_type: "text/html;profile=mcp-app".to_string(),
                        text: Some(html),
                        blob: None,
                        meta: None,
                    };

                    let app = GooseApp {
                        resource: mcp_resource,
                        mcp_server: Some(extension_name),
                        window_props: Some(WindowProps {
                            width: 800,
                            height: 600,
                            resizable: true,
                        }),
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

fn format_resource_name(name: String) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
