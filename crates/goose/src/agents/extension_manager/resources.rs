use super::helpers::require_str_parameter;
use super::{ExtensionManager, McpClientBox};
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use rmcp::model::{Content, ErrorCode, ErrorData, Resource, ResourceContents};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use tracing::warn;

impl ExtensionManager {
    // Function that gets executed for read_resource tool
    pub async fn read_resource_tool(
        &self,
        session_id: &str,
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ErrorData> {
        let uri = require_str_parameter(&params, "uri")?;
        let extension_name = require_str_parameter(&params, "extension_name")?;

        let read_result = self
            .read_resource(session_id, uri, extension_name, cancellation_token)
            .await?;

        let mut result = Vec::new();
        for content in read_result.contents {
            if let ResourceContents::TextResourceContents { text, .. } = content {
                result.push(Content::text(format!("{}\n\n{}", uri, text)));
            }
        }
        Ok(result)
    }

    pub async fn read_resource(
        &self,
        session_id: &str,
        uri: &str,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<rmcp::model::ReadResourceResult, ErrorData> {
        let available_extensions = self
            .extensions
            .lock()
            .await
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "Extension '{}' not found. Here are the available extensions: {}",
            extension_name, available_extensions
        );

        let client = self
            .get_server_client(extension_name)
            .await
            .ok_or(ErrorData::new(ErrorCode::INVALID_PARAMS, error_msg, None))?;

        client
            .read_resource(session_id, uri, cancellation_token)
            .await
            .map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Could not read resource with uri: {}", uri),
                    None,
                )
            })
    }

    pub async fn get_ui_resources(
        &self,
        session_id: &str,
    ) -> Result<Vec<(String, Resource)>, ErrorData> {
        let mut ui_resources = Vec::new();

        let extensions_to_check: Vec<(String, McpClientBox)> = {
            let extensions = self.extensions.lock().await;
            extensions
                .iter()
                .map(|(name, ext)| (name.clone(), ext.get_client()))
                .collect()
        };

        for (extension_name, client) in extensions_to_check {
            match client
                .list_resources(session_id, None, CancellationToken::default())
                .await
            {
                Ok(list_response) => {
                    for resource in list_response.resources {
                        if resource.uri.starts_with("ui://") {
                            ui_resources.push((extension_name.clone(), resource));
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list resources for {}: {:?}", extension_name, e);
                }
            }
        }

        Ok(ui_resources)
    }

    async fn list_resources_from_extension(
        &self,
        session_id: &str,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ErrorData> {
        let client = self
            .get_server_client(extension_name)
            .await
            .ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Extension {} is not valid", extension_name),
                    None,
                )
            })?;

        client
            .list_resources(session_id, None, cancellation_token)
            .await
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Unable to list resources for {}, {:?}", extension_name, e),
                    None,
                )
            })
            .map(|lr| {
                let resource_list = lr
                    .resources
                    .into_iter()
                    .map(|r| format!("{} - {}, uri: ({})", extension_name, r.name, r.uri))
                    .collect::<Vec<String>>()
                    .join("\n");

                vec![Content::text(resource_list)]
            })
    }

    pub async fn list_resources(
        &self,
        session_id: &str,
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ErrorData> {
        let extension = params.get("extension_name").and_then(|v| v.as_str());

        match extension {
            Some(extension_name) => {
                // Handle single extension case
                self.list_resources_from_extension(session_id, extension_name, cancellation_token)
                    .await
            }
            None => {
                // Handle all extensions case using FuturesUnordered
                let mut futures = FuturesUnordered::new();

                // Create futures for each resource_capable_extension
                self.extensions
                    .lock()
                    .await
                    .iter()
                    .filter(|(_name, ext)| ext.supports_resources())
                    .map(|(name, _ext)| name.clone())
                    .for_each(|name| {
                        let token = cancellation_token.clone();
                        futures.push(async move {
                            self.list_resources_from_extension(session_id, name.as_str(), token)
                                .await
                        });
                    });

                let mut all_resources = Vec::new();
                let mut errors = Vec::new();

                // Process results as they complete
                while let Some(result) = futures.next().await {
                    match result {
                        Ok(content) => {
                            all_resources.extend(content);
                        }
                        Err(tool_error) => {
                            errors.push(tool_error);
                        }
                    }
                }

                if !errors.is_empty() {
                    tracing::error!(
                        errors = ?errors
                            .into_iter()
                            .map(|e| format!("{:?}", e))
                            .collect::<Vec<_>>(),
                        "errors from listing resources"
                    );
                }

                Ok(all_resources)
            }
        }
    }
}
