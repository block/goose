//! goose Apps module
//!
//! This module contains types and utilities for working with goose Apps,
//! which are UI resources that can be rendered in an MCP server or native
//! goose apps, or something in between.

pub mod resource;

use crate::agents::ExtensionManager;
use rmcp::model::ErrorData;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use utoipa::ToSchema;

pub use resource::{CspMetadata, McpAppResource, ResourceMetadata, UiMetadata};

/// GooseApp represents an app that can be launched in a standalone window
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GooseApp {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resizable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    pub resource_uri: String,
    pub html: String,
}

/// List all MCP apps from loaded extensions
pub async fn list_mcp_apps(
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
                // Extract HTML from the first text content
                let mut html = String::new();
                for content in read_result.contents {
                    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content
                    {
                        html = text;
                        break;
                    }
                }

                if !html.is_empty() {
                    apps.push(GooseApp {
                        name: format_resource_name(resource.name.clone()),
                        description: resource.description.clone(),
                        resource_uri: resource.uri.clone(),
                        html,
                        width: None,
                        height: None,
                        resizable: Some(true),
                        mcp_server: Some(extension_name),
                    });
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
