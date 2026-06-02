use super::helpers::{
    get_tool_meta_value, get_tool_owner, get_tool_resource_uri, insert_trusted_tool_update_meta,
    is_unprefixed_extension, remove_untrusted_mcp_app_meta, ResolvedTool, TOOL_EXTENSION_META_KEY,
};
use super::{ExtensionManager, GooseMcpAppToolAttachment, McpClientBox};
use crate::agents::extension::ExtensionResult;
use crate::agents::tool_execution::{ToolCallContext, ToolCallResult};
use crate::config::extensions::name_to_key;
use anyhow::Result;
use futures::future;
use futures::FutureExt;
use rmcp::model::{CallToolRequestParams, ErrorCode, ErrorData, Tool};
use rmcp::service::ServiceError;
use schemars::_private::NoSerialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::warn;

impl ExtensionManager {
    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(
        &self,
        session_id: &str,
        extension_name: Option<String>,
    ) -> ExtensionResult<Vec<Tool>> {
        let all_tools = self.get_all_tools_cached(session_id).await?;
        Ok(self.filter_tools(&all_tools, extension_name.as_deref(), None))
    }

    pub async fn get_prefixed_tools_excluding(
        &self,
        session_id: &str,
        exclude: &str,
    ) -> ExtensionResult<Vec<Tool>> {
        let all_tools = self.get_all_tools_cached(session_id).await?;
        Ok(self.filter_tools(&all_tools, None, Some(exclude)))
    }

    pub(super) fn filter_tools(
        &self,
        tools: &[Tool],
        extension_name: Option<&str>,
        exclude: Option<&str>,
    ) -> Vec<Tool> {
        let extension_name_normalized = extension_name.map(name_to_key);
        let exclude_normalized = exclude.map(name_to_key);

        tools
            .iter()
            .filter(|tool| {
                let tool_owner = get_tool_owner(tool)
                    .map(|s| name_to_key(&s))
                    .unwrap_or_else(|| tool.name.split("__").next().unwrap_or("").to_string());

                if let Some(ref excluded) = exclude_normalized {
                    if tool_owner == *excluded {
                        return false;
                    }
                }

                if let Some(ref name_filter) = extension_name_normalized {
                    tool_owner == *name_filter
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    pub(super) async fn get_all_tools_cached(
        &self,
        session_id: &str,
    ) -> ExtensionResult<Arc<Vec<Tool>>> {
        {
            let cache = self.tools_cache.lock().await;
            if let Some(ref tools) = *cache {
                return Ok(Arc::clone(tools));
            }
        }

        let version_before = self.tools_cache_version.load(Ordering::SeqCst);
        let tools = Arc::new(self.fetch_all_tools(session_id).await?);

        {
            let mut cache = self.tools_cache.lock().await;
            let version_after = self.tools_cache_version.load(Ordering::SeqCst);
            if version_after == version_before && cache.is_none() {
                *cache = Some(Arc::clone(&tools));
            }
        }

        Ok(tools)
    }

    pub(super) fn host_supports_mcp_apps(&self) -> bool {
        if let Some(host_info) = &self.capabilities.host_info {
            if host_info.explicit_extensions {
                return host_info.mcpui_enabled();
            }
        }

        self.capabilities.mcpui
    }

    pub(super) async fn hydrate_mcp_app_attachment(
        client: &McpClientBox,
        session_id: &str,
        resolved_tool: &ResolvedTool,
        cancellation_token: CancellationToken,
    ) -> Option<GooseMcpAppToolAttachment> {
        let resource_uri = resolved_tool.resource_uri.clone()?;

        let mut attachment = GooseMcpAppToolAttachment {
            tool_name: resolved_tool.tool_name.clone(),
            extension_name: resolved_tool.extension_name.clone(),
            resource_uri: resource_uri.clone(),
            tool_meta: resolved_tool.tool_meta.clone(),
            resource_result: None,
            read_error: None,
        };

        match client
            .read_resource(session_id, &resource_uri, cancellation_token)
            .await
        {
            Ok(resource_result) => {
                attachment.resource_result = serde_json::to_value(&resource_result).ok();
            }
            Err(error) => {
                attachment.read_error = Some(error.to_string());
            }
        }

        Some(attachment)
    }

    pub(super) async fn invalidate_tools_cache_and_bump_version(&self) {
        self.tools_cache_version.fetch_add(1, Ordering::SeqCst);
        *self.tools_cache.lock().await = None;
    }

    pub(super) async fn fetch_all_tools(&self, session_id: &str) -> ExtensionResult<Vec<Tool>> {
        let clients: Vec<_> = self
            .extensions
            .lock()
            .await
            .iter()
            .map(|(name, ext)| (name.clone(), ext.config.clone(), ext.get_client()))
            .collect();

        let cancel_token = CancellationToken::default();
        let client_futures = clients.into_iter().map(|(name, config, client)| {
            let cancel_token = cancel_token.clone();
            let ext_name = name.clone();
            async move {
                let mut tools = Vec::new();
                let mut client_tools = match client
                    .list_tools(session_id, None, cancel_token.clone())
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        warn!(extension = %ext_name, error = %e, "Failed to list tools");
                        return (name, vec![]);
                    }
                };

                let expose_unprefixed = is_unprefixed_extension(&config);

                loop {
                    for mut tool in client_tools.tools {
                        if config.is_tool_available(&tool.name) {
                            let public_name = if expose_unprefixed {
                                tool.name.to_string()
                            } else {
                                format!("{}__{}", name, tool.name)
                            };

                            let mut meta_map = tool
                                .meta
                                .as_ref()
                                .map(|m| m.0.clone())
                                .unwrap_or_default();
                            meta_map.insert(
                                TOOL_EXTENSION_META_KEY.to_string(),
                                serde_json::Value::String(name.clone()),
                            );

                            tool.name = public_name.into();
                            tool.meta = Some(rmcp::model::Meta(meta_map));

                            tools.push(tool);
                        }
                    }

                    if client_tools.next_cursor.is_none() {
                        break;
                    }

                    client_tools = match client
                        .list_tools(session_id, client_tools.next_cursor, cancel_token.clone())
                        .await
                    {
                        Ok(t) => t,
                        Err(e) => {
                            warn!(extension = %ext_name, error = %e, "Failed to list tools (pagination)");
                            break;
                        }
                    };
                }

                (name, tools)
            }
        });

        let results = future::join_all(client_futures).await;

        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut tools = Vec::new();
        for (ext_name, client_tools) in results {
            for tool in client_tools {
                let tool_name = tool.name.to_string();
                if seen_names.contains(&tool_name) {
                    warn!(
                        tool = %tool_name,
                        extension = %ext_name,
                        "Duplicate tool name - skipping"
                    );
                    continue;
                }
                seen_names.insert(tool_name);
                tools.push(tool);
            }
        }

        Ok(tools)
    }

    pub(super) async fn resolve_tool(
        &self,
        session_id: &str,
        tool_name: &str,
    ) -> Result<ResolvedTool, ErrorData> {
        let tools = self.get_all_tools_cached(session_id).await.map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to get tools: {}", e),
                None,
            )
        })?;

        if let Some(tool) = tools.iter().find(|t| *t.name == *tool_name) {
            let owner = get_tool_owner(tool)
                .or_else(|| {
                    tool_name
                        .split_once("__")
                        .map(|(prefix, _)| name_to_key(prefix))
                })
                .ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::RESOURCE_NOT_FOUND,
                        format!("Tool '{}' has no owner", tool_name),
                        None,
                    )
                })?;

            let actual_tool_name = tool_name
                .strip_prefix(&format!("{owner}__"))
                .unwrap_or(tool_name)
                .to_string();

            let client = self.get_server_client(&owner).await.ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::RESOURCE_NOT_FOUND,
                    format!("Extension '{}' not found for tool '{}'", owner, tool_name),
                    None,
                )
            })?;

            return Ok(ResolvedTool {
                tool_name: tool.name.to_string(),
                extension_name: owner,
                actual_tool_name,
                client,
                tool_meta: get_tool_meta_value(tool),
                resource_uri: get_tool_resource_uri(tool),
            });
        }

        if let Some((prefix, actual)) = tool_name.split_once("__") {
            let owner = name_to_key(prefix);
            if let Some(client) = self.get_server_client(&owner).await {
                return Ok(ResolvedTool {
                    tool_name: tool_name.to_string(),
                    extension_name: owner,
                    actual_tool_name: actual.to_string(),
                    client,
                    tool_meta: None,
                    resource_uri: None,
                });
            }
        }

        let available = tools
            .iter()
            .map(|t| t.name.as_ref())
            .collect::<Vec<&str>>()
            .join(", ");

        Err(ErrorData::new(
            ErrorCode::RESOURCE_NOT_FOUND,
            format!(
                "Tool '{}' not found. Available tools: [{}]",
                tool_name, available
            ),
            None,
        ))
    }

    pub async fn dispatch_tool_call(
        &self,
        ctx: &ToolCallContext,
        tool_call: CallToolRequestParams,
        cancellation_token: CancellationToken,
    ) -> Result<ToolCallResult> {
        let tool_name_str = tool_call.name.to_string();
        let resolved = self.resolve_tool(&ctx.session_id, &tool_name_str).await?;

        if let Some(extension) = self.extensions.lock().await.get(&resolved.extension_name) {
            if !extension
                .config
                .is_tool_available(&resolved.actual_tool_name)
            {
                return Err(ErrorData::new(
                    ErrorCode::RESOURCE_NOT_FOUND,
                    format!(
                        "Tool '{}' is not available for extension '{}'",
                        resolved.actual_tool_name, resolved.extension_name
                    ),
                    None,
                )
                .into());
            }
        }

        let arguments = tool_call.arguments.clone();
        let client = resolved.client.clone();
        let hydration_client = client.clone();
        let notifications_receiver = client.subscribe().await;
        let actual_tool_name = resolved.actual_tool_name.clone();
        let resolved_tool = resolved;
        let should_hydrate_mcp_app = self.host_supports_mcp_apps();
        let read_cancellation_token = cancellation_token.clone();
        let session_id = ctx.session_id.clone();
        let owned_ctx = ToolCallContext::new(
            ctx.session_id.clone(),
            ctx.working_dir.clone(),
            ctx.tool_call_request_id.clone(),
        );

        let fut = async move {
            tracing::debug!(
                "dispatch_tool_call: calling client.call_tool tool={} session_id={} working_dir={:?}",
                actual_tool_name,
                owned_ctx.session_id,
                owned_ctx.working_dir,
            );
            let mut result = client
                .call_tool(&owned_ctx, &actual_tool_name, arguments, cancellation_token)
                .await
                .map_err(|e| match e {
                    ServiceError::McpError(error_data) => error_data,
                    _ => {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), e.maybe_to_value())
                    }
                })?;

            remove_untrusted_mcp_app_meta(&mut result);

            if should_hydrate_mcp_app && result.is_error != Some(true) {
                if let Some(attachment) = Self::hydrate_mcp_app_attachment(
                    &hydration_client,
                    &session_id,
                    &resolved_tool,
                    read_cancellation_token,
                )
                .await
                {
                    insert_trusted_tool_update_meta(&mut result, &attachment);
                }
            }

            Ok(result)
        };

        Ok(ToolCallResult {
            result: Box::new(fut.boxed()),
            notification_stream: Some(Box::new(ReceiverStream::new(notifications_receiver))),
        })
    }
}
