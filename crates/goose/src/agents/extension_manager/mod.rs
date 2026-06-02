use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use rmcp::transport::ConfigureCommandExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::container::Container;
use super::extension::{
    ExtensionConfig, ExtensionError, ExtensionInfo, ExtensionResult, PlatformExtensionContext,
    ToolInfo, PLATFORM_EXTENSIONS,
};
use super::types::SharedProvider;
use crate::agents::extension_malware_check;
use crate::agents::mcp_client::{
    GooseMcpClientCapabilities, GooseMcpHostInfo, McpClient, McpClientTrait,
};
use crate::builtin_extension::get_builtin_extension;
use crate::config::extensions::name_to_key;
use crate::config::{get_all_extensions, Config};
use crate::prompt_template;
use rmcp::model::{Content, ErrorCode, ErrorData, GetPromptResult, Prompt, ServerInfo, Tool};
use serde_json::Value;

type McpClientBox = Arc<dyn McpClientTrait>;

mod connection;
mod helpers;
mod resources;
mod tools;

use connection::{child_process_client, create_streamable_http_client};
pub use helpers::{
    get_parameter_names, get_tool_owner, is_first_class_extension, is_hidden_extension,
    merge_environments, substitute_env_vars,
};
use helpers::{resolve_command, resolve_timeout};

struct Extension {
    pub config: ExtensionConfig,
    /// Resolved config snapshot (with secrets from keyring substituted)
    /// captured at client-creation time. Used to detect secret rotation
    /// without re-reading the keyring on every comparison. Only held in
    /// memory — never serialized to disk.
    resolved_config: ExtensionConfig,

    client: McpClientBox,
    server_info: Option<ServerInfo>,
    _temp_dir: Option<tempfile::TempDir>,
}

impl Extension {
    fn new(
        config: ExtensionConfig,
        resolved_config: ExtensionConfig,
        client: McpClientBox,
        server_info: Option<ServerInfo>,
        temp_dir: Option<tempfile::TempDir>,
    ) -> Self {
        Self {
            client,
            config,
            resolved_config,
            server_info,
            _temp_dir: temp_dir,
        }
    }

    fn supports_resources(&self) -> bool {
        self.server_info
            .as_ref()
            .and_then(|info| info.capabilities.resources.as_ref())
            .is_some()
    }

    fn get_instructions(&self) -> Option<String> {
        self.client.get_instructions()
    }

    fn get_client(&self) -> McpClientBox {
        self.client.clone()
    }
}

pub struct ExtensionManagerCapabilities {
    pub mcpui: bool,
    pub host_info: Option<GooseMcpHostInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooseMcpAppToolAttachment {
    pub tool_name: String,
    pub extension_name: String,
    pub resource_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_meta: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<String>,
}

pub(crate) const TRUSTED_TOOL_UPDATE_META_KEY: &str = "__goose_tool_update_meta";

/// A flattened representation of a resource used by the agent to prepare inference
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub extension_name: String, // The name of the extension that owns the resource
    pub uri: String,            // The URI of the resource
    pub name: String,           // The name of the resource
    pub content: String,        // The content of the resource
    pub timestamp: DateTime<Utc>, // The timestamp of the resource
    pub priority: f32,          // The priority of the resource
    pub token_count: Option<u32>, // The token count of the resource (filled in by the agent)
}

impl ResourceItem {
    pub fn new(
        extension_name: String,
        uri: String,
        name: String,
        content: String,
        timestamp: DateTime<Utc>,
        priority: f32,
    ) -> Self {
        Self {
            extension_name,
            uri,
            name,
            content,
            timestamp,
            priority,
            token_count: None,
        }
    }
}

/// Manages goose extensions / MCP clients and their interactions
pub struct ExtensionManager {
    extensions: Mutex<HashMap<String, Extension>>,
    context: PlatformExtensionContext,
    provider: SharedProvider,
    tools_cache: Mutex<Option<Arc<Vec<Tool>>>>,
    tools_cache_version: AtomicU64,
    client_name: String,
    capabilities: ExtensionManagerCapabilities,
}

impl ExtensionManager {
    fn mcp_client_capabilities(&self) -> GooseMcpClientCapabilities {
        GooseMcpClientCapabilities {
            mcpui: self.capabilities.mcpui,
            host_info: self.capabilities.host_info.clone(),
        }
    }

    pub fn new(
        provider: SharedProvider,
        session_manager: Arc<crate::session::SessionManager>,
        client_name: String,
        capabilities: ExtensionManagerCapabilities,
        use_login_shell_path: bool,
    ) -> Self {
        Self {
            extensions: Mutex::new(HashMap::new()),
            context: PlatformExtensionContext {
                extension_manager: None,
                session_manager,
                session: None,
                use_login_shell_path,
            },
            provider,
            tools_cache: Mutex::new(None),
            tools_cache_version: AtomicU64::new(0),
            client_name,
            capabilities,
        }
    }

    pub fn new_without_provider(data_dir: std::path::PathBuf) -> Self {
        let session_manager = Arc::new(crate::session::SessionManager::new(data_dir));
        Self::new(
            Arc::new(Mutex::new(None)),
            session_manager,
            "goose-cli".to_string(),
            ExtensionManagerCapabilities {
                mcpui: false,
                host_info: None,
            },
            false,
        )
    }

    pub fn get_context(&self) -> &PlatformExtensionContext {
        &self.context
    }

    pub fn get_provider(&self) -> &SharedProvider {
        &self.provider
    }

    pub async fn supports_resources(&self) -> bool {
        self.extensions
            .lock()
            .await
            .values()
            .any(|ext| ext.supports_resources())
    }

    /// Add an extension with an optional working directory.
    /// If working_dir is None, falls back to current_dir.
    #[allow(clippy::too_many_lines)]
    pub async fn add_extension(
        self: &Arc<Self>,
        config: ExtensionConfig,
        working_dir: Option<PathBuf>,
        container: Option<&Container>,
        session_id: Option<&str>,
    ) -> ExtensionResult<()> {
        let sanitized_name = config.key();

        // Compare both the unresolved config (to detect structural changes like
        // migrating from plaintext envs to env_keys) and the resolved config (to
        // detect secret rotation where only keyring values changed). Only skip
        // restart if both match.
        let resolved_config = config.clone().resolve(Config::global()).await?;

        if let Some(existing) = self.extensions.lock().await.get(&sanitized_name) {
            if existing.config == config && existing.resolved_config == resolved_config {
                return Ok(());
            }
            tracing::debug!(
                name = sanitized_name,
                "extension config changed, restarting with updated config"
            );
        }

        let mut temp_dir = None;

        let effective_working_dir = working_dir
            .clone()
            .or_else(|| std::env::var("GOOSE_WORKING_DIR").ok().map(PathBuf::from))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let client: Box<dyn McpClientTrait> = match &config {
            ExtensionConfig::Sse { .. } => {
                return Err(ExtensionError::ConfigError(
                    "SSE is unsupported, migrate to streamable_http".to_string(),
                ));
            }
            ExtensionConfig::StreamableHttp {
                uri,
                timeout,
                headers,
                name,
                envs,
                env_keys,
                socket,
                ..
            } => {
                let config = Config::global();
                let all_envs = merge_environments(envs, env_keys, &sanitized_name, config).await?;
                let resolved_uri = substitute_env_vars(uri, &all_envs);
                let resolved_headers = headers
                    .iter()
                    .map(|(k, v)| (k.clone(), substitute_env_vars(v, &all_envs)))
                    .collect();
                let resolved_socket = socket.as_ref().map(|s| substitute_env_vars(s, &all_envs));
                create_streamable_http_client(
                    &resolved_uri,
                    *timeout,
                    &resolved_headers,
                    name,
                    resolved_socket.as_deref(),
                    self.provider.clone(),
                    self.client_name.clone(),
                    self.mcp_client_capabilities(),
                    &effective_working_dir,
                )
                .await?
            }
            ExtensionConfig::Builtin { ref name, .. }
            | ExtensionConfig::Platform { ref name, .. } => {
                let timeout = if let ExtensionConfig::Builtin { timeout, .. } = &config {
                    *timeout
                } else {
                    None
                };
                let normalized_name = name_to_key(name);

                if let Some(def) = PLATFORM_EXTENSIONS.get(normalized_name.as_str()) {
                    // Platform extension: create via in-process client factory
                    let mut context = self.context.clone();
                    context.extension_manager = Some(Arc::downgrade(self));
                    if let Some(id) = session_id {
                        if let Ok(session) =
                            self.context.session_manager.get_session(id, false).await
                        {
                            context.session = Some(Arc::new(session));
                        }
                    }
                    (def.client_factory)(context)
                } else {
                    // Builtin MCP server extension
                    let timeout_secs = resolve_timeout(timeout);
                    let extension_fn =
                        get_builtin_extension(normalized_name.as_str()).ok_or_else(|| {
                            ExtensionError::ConfigError(format!("Unknown extension: {}", name))
                        })?;

                    if let Some(container) = container {
                        let container_id = container.id();
                        tracing::info!(
                            container = %container_id,
                            builtin = %name,
                            "Starting builtin extension inside Docker container"
                        );
                        let command = Command::new("docker").configure(|command| {
                            command
                                .arg("exec")
                                .arg("-i")
                                .arg(container_id)
                                .arg("goose")
                                .arg("mcp")
                                .arg(&normalized_name);
                        });

                        let client = child_process_client(
                            command,
                            &Some(timeout_secs),
                            self.provider.clone(),
                            &effective_working_dir,
                            Some(container_id.to_string()),
                            self.client_name.clone(),
                            self.mcp_client_capabilities(),
                        )
                        .await?;
                        Box::new(client)
                    } else {
                        let (server_read, client_write) = tokio::io::duplex(65536);
                        let (client_read, server_write) = tokio::io::duplex(65536);
                        extension_fn(server_read, server_write);

                        Box::new(
                            McpClient::connect(
                                (client_read, client_write),
                                Duration::from_secs(timeout_secs),
                                self.provider.clone(),
                                self.client_name.clone(),
                                self.mcp_client_capabilities(),
                                effective_working_dir.clone(),
                            )
                            .await?,
                        )
                    }
                }
            }
            ExtensionConfig::Stdio {
                cmd,
                args,
                envs,
                env_keys,
                timeout,
                ..
            } => {
                let config = Config::global();
                let mut all_envs =
                    merge_environments(envs, env_keys, &sanitized_name, config).await?;

                if let Some(sid) = session_id {
                    all_envs.insert("AGENT_SESSION_ID".to_string(), sid.to_string());
                }

                // Check for malicious packages before launching the process
                extension_malware_check::deny_if_malicious_cmd_args(cmd, args).await?;

                let command = if let Some(container) = container {
                    let container_id = container.id();
                    tracing::info!(
                        container = %container_id,
                        cmd = %cmd,
                        "Starting stdio extension inside Docker container"
                    );
                    Command::new("docker").configure(|command| {
                        command.arg("exec").arg("-i");
                        for (key, value) in &all_envs {
                            command.arg("-e").arg(format!("{}={}", key, value));
                        }
                        command.arg(container_id);
                        command.arg(cmd);
                        command.args(args);
                    })
                } else {
                    let cmd = resolve_command(cmd);
                    Command::new(cmd).configure(|command| {
                        command.args(args).envs(all_envs);
                    })
                };

                let client = child_process_client(
                    command,
                    timeout,
                    self.provider.clone(),
                    &effective_working_dir,
                    container.map(|c| c.id().to_string()),
                    self.client_name.clone(),
                    self.mcp_client_capabilities(),
                )
                .await?;
                Box::new(client)
            }
            ExtensionConfig::InlinePython {
                name,
                code,
                timeout,
                dependencies,
                ..
            } => {
                let dir = tempdir()?;
                let file_path = dir.path().join(format!("{}.py", name));
                temp_dir = Some(dir);
                std::fs::write(&file_path, code)?;

                let command = Command::new("uvx").configure(|command| {
                    command.arg("--with").arg("mcp");
                    dependencies.iter().flatten().for_each(|dep| {
                        command.arg("--with").arg(dep);
                    });
                    command.arg("python").arg(file_path.to_str().unwrap());
                });

                let client = child_process_client(
                    command,
                    timeout,
                    self.provider.clone(),
                    &effective_working_dir,
                    container.map(|c| c.id().to_string()),
                    self.client_name.clone(),
                    self.mcp_client_capabilities(),
                )
                .await?;

                Box::new(client)
            }
            ExtensionConfig::Frontend { .. } => {
                return Err(ExtensionError::ConfigError(
                    "Invalid extension type: Frontend extensions cannot be added as server extensions".to_string()
                ));
            }
        };

        let server_info = client.get_info().cloned();

        let mut extensions = self.extensions.lock().await;
        extensions.insert(
            sanitized_name,
            Extension::new(
                config,
                resolved_config,
                Arc::from(client),
                server_info,
                temp_dir,
            ),
        );
        drop(extensions);
        self.invalidate_tools_cache_and_bump_version().await;

        Ok(())
    }

    pub async fn add_client(
        &self,
        name: String,
        config: ExtensionConfig,
        client: McpClientBox,
        info: Option<ServerInfo>,
        temp_dir: Option<TempDir>,
    ) {
        let normalized = name_to_key(&name);
        self.extensions.lock().await.insert(
            normalized,
            Extension::new(config.clone(), config.clone(), client, info, temp_dir),
        );
        self.invalidate_tools_cache_and_bump_version().await;
    }

    /// Get extensions info for building the system prompt
    pub async fn get_extensions_info(&self, working_dir: &std::path::Path) -> Vec<ExtensionInfo> {
        let working_dir_str = working_dir.to_string_lossy();
        self.extensions
            .lock()
            .await
            .iter()
            .map(|(name, ext)| {
                let instructions = ext.get_instructions().unwrap_or_default();
                let instructions = instructions.replace("{{WORKING_DIR}}", &working_dir_str);
                ExtensionInfo::new(name, &instructions, ext.supports_resources())
            })
            .collect()
    }

    /// Get aggregated usage statistics
    pub async fn remove_extension(&self, name: &str) -> ExtensionResult<()> {
        let sanitized_name = name_to_key(name);
        self.extensions.lock().await.remove(&sanitized_name);
        self.invalidate_tools_cache_and_bump_version().await;
        Ok(())
    }

    pub async fn update_working_dir(&self, new_dir: &std::path::Path) {
        let extensions = self.extensions.lock().await;
        for (name, ext) in extensions.iter() {
            if let Err(e) = ext.client.update_working_dir(new_dir.to_path_buf()).await {
                tracing::warn!(extension = %name, error = %e, "failed to update roots");
            }
        }
    }

    pub async fn get_extension_and_tool_counts(&self, session_id: &str) -> (usize, usize) {
        let enabled_extensions_count = self.extensions.lock().await.len();

        let total_tools = self
            .get_prefixed_tools(session_id, None)
            .await
            .map(|tools| tools.len())
            .unwrap_or(0);

        (enabled_extensions_count, total_tools)
    }

    pub async fn list_extensions(&self) -> ExtensionResult<Vec<String>> {
        Ok(self.extensions.lock().await.keys().cloned().collect())
    }

    pub async fn is_extension_enabled(&self, name: &str) -> bool {
        let normalized = name_to_key(name);
        self.extensions.lock().await.contains_key(&normalized)
    }

    pub async fn get_extension_configs(&self) -> Vec<ExtensionConfig> {
        self.extensions
            .lock()
            .await
            .values()
            .map(|ext| ext.config.clone())
            .collect()
    }

    /// Get the extension prompt including client instructions
    pub async fn get_planning_prompt(&self, tools_info: Vec<ToolInfo>) -> String {
        let mut context: HashMap<&str, Value> = HashMap::new();
        context.insert("tools", serde_json::to_value(tools_info).unwrap());

        prompt_template::render_template("plan.md", &context).expect("Prompt should render")
    }

    async fn get_server_client(&self, name: impl Into<String>) -> Option<McpClientBox> {
        let normalized = name_to_key(&name.into());
        self.extensions
            .lock()
            .await
            .get(&normalized)
            .map(|ext| ext.get_client())
    }

    pub async fn list_prompts_from_extension(
        &self,
        session_id: &str,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Prompt>, ErrorData> {
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
            .list_prompts(session_id, None, cancellation_token)
            .await
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Unable to list prompts for {}, {:?}", extension_name, e),
                    None,
                )
            })
            .map(|lp| lp.prompts)
    }

    pub async fn list_prompts(
        &self,
        session_id: &str,
        cancellation_token: CancellationToken,
    ) -> Result<HashMap<String, Vec<Prompt>>, ErrorData> {
        let mut futures = FuturesUnordered::new();

        let names: Vec<_> = self.extensions.lock().await.keys().cloned().collect();
        for extension_name in names {
            let token = cancellation_token.clone();
            futures.push(async move {
                (
                    extension_name.clone(),
                    self.list_prompts_from_extension(session_id, extension_name.as_str(), token)
                        .await,
                )
            });
        }

        let mut all_prompts = HashMap::new();
        let mut errors = Vec::new();

        // Process results as they complete
        while let Some(result) = futures.next().await {
            let (name, prompts) = result;
            match prompts {
                Ok(content) => {
                    all_prompts.insert(name.to_string(), content);
                }
                Err(tool_error) => {
                    errors.push(tool_error);
                }
            }
        }

        if !errors.is_empty() {
            tracing::debug!(
                errors = ?errors
                    .into_iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<_>>(),
                "errors from listing prompts"
            );
        }

        Ok(all_prompts)
    }

    pub async fn get_prompt(
        &self,
        session_id: &str,
        extension_name: &str,
        name: &str,
        arguments: Value,
        cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult> {
        let client = self
            .get_server_client(extension_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Extension {} not found", extension_name))?;

        client
            .get_prompt(session_id, name, arguments, cancellation_token)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get prompt: {}", e))
    }

    pub async fn search_available_extensions(&self) -> Result<Vec<Content>, ErrorData> {
        let mut output_parts = vec![];

        // First get disabled extensions from current config (skip hidden ones)
        let mut disabled_extensions: Vec<String> = vec![];
        for extension in get_all_extensions() {
            if !extension.enabled && !is_hidden_extension(&extension.config.name()) {
                let config = extension.config.clone();
                let description = match &config {
                    ExtensionConfig::Builtin {
                        description,
                        display_name,
                        ..
                    } => {
                        if description.is_empty() {
                            display_name.as_deref().unwrap_or("Built-in extension")
                        } else {
                            description
                        }
                    }
                    ExtensionConfig::Sse { .. } => "SSE extension (unsupported)",
                    ExtensionConfig::Platform { description, .. }
                    | ExtensionConfig::StreamableHttp { description, .. }
                    | ExtensionConfig::Stdio { description, .. }
                    | ExtensionConfig::Frontend { description, .. }
                    | ExtensionConfig::InlinePython { description, .. } => description,
                };
                disabled_extensions.push(format!("- {} - {}", config.name(), description));
            }
        }

        // Get currently enabled extensions that can be disabled (skip hidden ones)
        let enabled_extensions: Vec<String> = self
            .extensions
            .lock()
            .await
            .keys()
            .filter(|name| !is_hidden_extension(name))
            .cloned()
            .collect();

        // Build output string
        if !disabled_extensions.is_empty() {
            output_parts.push(format!(
                "Extensions available to enable:\n{}\n",
                disabled_extensions.join("\n")
            ));
        } else {
            output_parts.push("No extensions available to enable.\n".to_string());
        }

        if !enabled_extensions.is_empty() {
            output_parts.push(format!(
                "\n\nExtensions available to disable:\n{}\n",
                enabled_extensions
                    .iter()
                    .map(|name| format!("- {}", name))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else {
            output_parts.push("No extensions that can be disabled.\n".to_string());
        }

        Ok(vec![Content::text(output_parts.join("\n"))])
    }

    pub async fn collect_moim(
        &self,
        session_id: &str,
        working_dir: &std::path::Path,
    ) -> Option<String> {
        // Skip MOIM for models with small context windows to avoid consuming limited context
        const MIN_CONTEXT_FOR_MOIM: usize = 32_000;
        if let Ok(provider_guard) = self.provider.try_lock() {
            if let Some(provider) = provider_guard.as_ref() {
                if provider.get_model_config().context_limit() < MIN_CONTEXT_FOR_MOIM {
                    return None;
                }
            }
        }

        // Use minute-level granularity to prevent conversation changes every second
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:00").to_string();
        let mut content = format!(
            "<info-msg>\nIt is currently {}\nWorking directory: {}\n",
            timestamp,
            working_dir.display()
        );

        if let Ok(session) = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
        {
            if let (Some(total), Some(config)) =
                (session.total_tokens, session.model_config.as_ref())
            {
                let limit = config.context_limit();
                if total > 0 && limit > 0 {
                    let pct = (total as f64 / limit as f64 * 100.0).round() as u32;
                    content.push_str(&format!(
                        "Context: ~{}k/{}k tokens used ({}%)\n",
                        total / 1000,
                        limit / 1000,
                        pct
                    ));
                }
            }
        }

        let platform_clients: Vec<(String, McpClientBox)> = {
            let extensions = self.extensions.lock().await;
            extensions
                .iter()
                .filter_map(|(name, extension)| {
                    let is_platform = match &extension.config {
                        ExtensionConfig::Platform { .. } => true,
                        ExtensionConfig::Builtin { name: ext_name, .. } => {
                            PLATFORM_EXTENSIONS.contains_key(name_to_key(ext_name).as_str())
                        }
                        _ => false,
                    };
                    if is_platform {
                        Some((name.clone(), extension.get_client()))
                    } else {
                        None
                    }
                })
                .collect()
        };

        for (name, client) in platform_clients {
            if let Some(moim_content) = client.get_moim(session_id).await {
                tracing::debug!("MOIM content from {}: {} chars", name, moim_content.len());
                content.push('\n');
                content.push_str(&moim_content);
            }
        }

        content.push_str("\n</info-msg>");

        Some(content)
    }
}

#[cfg(test)]
mod tests {
    use super::connection::should_attempt_oauth_fallback;
    use super::helpers::{insert_trusted_tool_update_meta, remove_untrusted_mcp_app_meta};
    use super::*;
    use crate::agents::tool_execution::ToolCallContext;
    use rmcp::model::CallToolRequestParams;
    use rmcp::model::CallToolResult;
    use rmcp::model::Meta;
    use rmcp::model::{InitializeResult, JsonObject};
    use rmcp::service::ClientInitializeError;
    use rmcp::{object, ServiceError as Error};

    use rmcp::model::ListPromptsResult;
    use rmcp::model::ListResourcesResult;
    use rmcp::model::ListToolsResult;
    use rmcp::model::ReadResourceResult;
    use rmcp::model::ServerNotification;

    use tokio::sync::mpsc;

    impl ExtensionManager {
        async fn add_mock_extension(&self, name: String, client: McpClientBox) {
            self.add_mock_extension_with_tools(name, client, vec![])
                .await;
        }

        async fn add_mock_extension_with_tools(
            &self,
            name: String,
            client: McpClientBox,
            available_tools: Vec<String>,
        ) {
            let sanitized_name = name_to_key(&name);
            let config = ExtensionConfig::Builtin {
                name: name.clone(),
                display_name: Some(name.clone()),
                description: "built-in".to_string(),
                timeout: None,
                bundled: None,
                available_tools,
            };
            let extension = Extension::new(config.clone(), config.clone(), client, None, None);
            self.extensions
                .lock()
                .await
                .insert(sanitized_name, extension);
            self.invalidate_tools_cache_and_bump_version().await;
        }
    }

    struct MockClient {}

    #[async_trait::async_trait]
    impl McpClientTrait for MockClient {
        fn get_info(&self) -> Option<&InitializeResult> {
            None
        }

        async fn list_resources(
            &self,
            _session_id: &str,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListResourcesResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn read_resource(
            &self,
            _session_id: &str,
            _uri: &str,
            _cancellation_token: CancellationToken,
        ) -> Result<ReadResourceResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn list_tools(
            &self,
            _session_id: &str,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListToolsResult, Error> {
            use serde_json::json;
            use std::sync::Arc;
            Ok(ListToolsResult {
                tools: vec![
                    Tool::new(
                        "tool".to_string(),
                        "A basic tool".to_string(),
                        Arc::new(json!({}).as_object().unwrap().clone()),
                    ),
                    Tool::new(
                        "available_tool".to_string(),
                        "An available tool".to_string(),
                        Arc::new(json!({}).as_object().unwrap().clone()),
                    ),
                    Tool::new(
                        "hidden_tool".to_string(),
                        "hidden tool".to_string(),
                        Arc::new(json!({}).as_object().unwrap().clone()),
                    ),
                ],
                next_cursor: None,
                meta: None,
            })
        }

        async fn call_tool(
            &self,
            _ctx: &ToolCallContext,
            name: &str,
            _arguments: Option<JsonObject>,
            _cancellation_token: CancellationToken,
        ) -> Result<CallToolResult, Error> {
            match name {
                "tool" | "test__tool" | "available_tool" | "hidden_tool" => {
                    Ok(CallToolResult::success(vec![]))
                }
                _ => Err(Error::TransportClosed),
            }
        }

        async fn list_prompts(
            &self,
            _session_id: &str,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListPromptsResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn get_prompt(
            &self,
            _session_id: &str,
            _name: &str,
            _arguments: Value,
            _cancellation_token: CancellationToken,
        ) -> Result<GetPromptResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
            mpsc::channel(1).1
        }
    }

    #[tokio::test]
    async fn test_dispatch_tool_call() {
        use super::super::tool_execution::ToolCallContext;

        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        // Add some mock clients using the helper method
        extension_manager
            .add_mock_extension("test_client".to_string(), Arc::new(MockClient {}))
            .await;

        extension_manager
            .add_mock_extension("__cli__ent__".to_string(), Arc::new(MockClient {}))
            .await;

        extension_manager
            .add_mock_extension("client 🚀".to_string(), Arc::new(MockClient {}))
            .await;

        let ctx = ToolCallContext::new(
            "test-session-id".to_string(),
            None,
            Some("test-req-id".to_string()),
        );

        let tool_call =
            CallToolRequestParams::new("test_client__tool".to_string()).with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = CallToolRequestParams::new("test_client__available_tool".to_string())
            .with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = CallToolRequestParams::new("__cli__ent____tool".to_string())
            .with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call =
            CallToolRequestParams::new("client___tool".to_string()).with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let invalid_tool_call =
            CallToolRequestParams::new("client___tools".to_string()).with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, invalid_tool_call, CancellationToken::default())
            .await;
        if let Err(err) = result {
            let tool_err = err.downcast_ref::<ErrorData>().expect("Expected ErrorData");
            assert_eq!(tool_err.code, ErrorCode::RESOURCE_NOT_FOUND);
        } else {
            panic!("Expected ErrorData with ErrorCode::RESOURCE_NOT_FOUND");
        }

        let invalid_tool_call =
            CallToolRequestParams::new("_client__tools".to_string()).with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, invalid_tool_call, CancellationToken::default())
            .await;
        if let Err(err) = result {
            let tool_err = err.downcast_ref::<ErrorData>().expect("Expected ErrorData");
            assert_eq!(tool_err.code, ErrorCode::RESOURCE_NOT_FOUND);
        } else {
            panic!("Expected ErrorData with ErrorCode::RESOURCE_NOT_FOUND");
        }
    }

    #[tokio::test]
    async fn test_tool_availability_filtering() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        // Only "available_tool" should be available to the LLM
        let available_tools = vec!["available_tool".to_string()];

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(MockClient {}),
                available_tools,
            )
            .await;

        let tools = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();

        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(!tool_names.iter().any(|name| name == "test_extension__tool")); // Default unavailable
        assert!(tool_names
            .iter()
            .any(|name| name == "test_extension__available_tool"));
        assert!(!tool_names
            .iter()
            .any(|name| name == "test_extension__hidden_tool"));
        assert!(tool_names.len() == 1);
    }

    #[tokio::test]
    async fn test_tool_availability_defaults_to_available() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(MockClient {}),
                vec![], // Empty available_tools means all tools are available by default
            )
            .await;

        let tools = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();

        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(tool_names.iter().any(|name| name == "test_extension__tool"));
        assert!(tool_names
            .iter()
            .any(|name| name == "test_extension__available_tool"));
        assert!(tool_names
            .iter()
            .any(|name| name == "test_extension__hidden_tool"));
        assert!(tool_names.len() == 3);
    }

    #[tokio::test]
    async fn test_dispatch_unavailable_tool_returns_error() {
        use super::super::tool_execution::ToolCallContext;

        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        let available_tools = vec!["available_tool".to_string()];

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(MockClient {}),
                available_tools,
            )
            .await;

        let ctx = ToolCallContext::new(
            "test-session-id".to_string(),
            None,
            Some("test-req-id".to_string()),
        );

        let unavailable_tool_call = CallToolRequestParams::new("test_extension__tool".to_string())
            .with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, unavailable_tool_call, CancellationToken::default())
            .await;

        if let Err(err) = result {
            let tool_err = err.downcast_ref::<ErrorData>().expect("Expected ErrorData");
            assert_eq!(tool_err.code, ErrorCode::RESOURCE_NOT_FOUND);
        } else {
            panic!("Expected ErrorData with ErrorCode::RESOURCE_NOT_FOUND");
        }

        // Try to call an available tool - should succeed
        let available_tool_call =
            CallToolRequestParams::new("test_extension__available_tool".to_string())
                .with_arguments(object!({}));

        let result = extension_manager
            .dispatch_tool_call(&ctx, available_tool_call, CancellationToken::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streamable_http_header_env_substitution() {
        let mut env_map = HashMap::new();
        env_map.insert("AUTH_TOKEN".to_string(), "secret123".to_string());
        env_map.insert("API_KEY".to_string(), "key456".to_string());

        // Test ${VAR} syntax
        let result = substitute_env_vars("Bearer ${ AUTH_TOKEN }", &env_map);
        assert_eq!(result, "Bearer secret123");

        // Test ${VAR} syntax without spaces
        let result = substitute_env_vars("Bearer ${AUTH_TOKEN}", &env_map);
        assert_eq!(result, "Bearer secret123");

        // Test $VAR syntax
        let result = substitute_env_vars("Bearer $AUTH_TOKEN", &env_map);
        assert_eq!(result, "Bearer secret123");

        // Test multiple substitutions
        let result = substitute_env_vars("Key: $API_KEY, Token: ${AUTH_TOKEN}", &env_map);
        assert_eq!(result, "Key: key456, Token: secret123");

        // Test no substitution when variable doesn't exist
        let result = substitute_env_vars("Bearer ${UNKNOWN_VAR}", &env_map);
        assert_eq!(result, "Bearer ${UNKNOWN_VAR}");

        // Test mixed content
        let result = substitute_env_vars(
            "Authorization: Bearer ${AUTH_TOKEN} and API ${API_KEY}",
            &env_map,
        );
        assert_eq!(result, "Authorization: Bearer secret123 and API key456");
    }

    #[tokio::test]
    async fn test_substitute_env_vars_no_recursive_expansion() {
        let mut env_map = HashMap::new();
        env_map.insert("TOKEN".to_string(), "abc$KEY".to_string());
        env_map.insert("KEY".to_string(), "xyz".to_string());

        // A substituted value containing $KEY should NOT be re-expanded
        let result = substitute_env_vars("${TOKEN}", &env_map);
        assert_eq!(result, "abc$KEY");

        let result = substitute_env_vars("$TOKEN", &env_map);
        assert_eq!(result, "abc$KEY");
    }

    #[tokio::test]
    async fn test_collect_moim_uses_minute_granularity() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = std::path::Path::new("/tmp");

        if let Some(moim) = em.collect_moim("test-session-id", working_dir).await {
            // Timestamp should end with :00 (seconds fixed to 00)
            assert!(
                moim.contains(":00\n"),
                "Timestamp should use minute granularity"
            );
        }
    }

    #[tokio::test]
    async fn test_tools_cache_invalidated_on_add_extension() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension("ext_a".to_string(), Arc::new(MockClient {}))
            .await;

        let tools_after_first = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();
        let tool_names: Vec<String> = tools_after_first
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        assert!(tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(!tool_names.iter().any(|n| n.starts_with("ext_b__")));

        extension_manager
            .add_mock_extension("ext_b".to_string(), Arc::new(MockClient {}))
            .await;

        let tools_after_second = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();
        let tool_names: Vec<String> = tools_after_second
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        assert!(tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(tool_names.iter().any(|n| n.starts_with("ext_b__")));
    }

    #[tokio::test]
    async fn test_tools_cache_invalidated_on_remove_extension() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension("ext_a".to_string(), Arc::new(MockClient {}))
            .await;
        extension_manager
            .add_mock_extension("ext_b".to_string(), Arc::new(MockClient {}))
            .await;

        let tools_before = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();
        let tool_names: Vec<String> = tools_before.iter().map(|t| t.name.to_string()).collect();
        assert!(tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(tool_names.iter().any(|n| n.starts_with("ext_b__")));

        extension_manager.remove_extension("ext_b").await.unwrap();

        let tools_after = extension_manager
            .get_prefixed_tools("test-session-id", None)
            .await
            .unwrap();
        let tool_names: Vec<String> = tools_after.iter().map(|t| t.name.to_string()).collect();
        assert!(tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(!tool_names.iter().any(|n| n.starts_with("ext_b__")));
    }

    #[tokio::test]
    async fn test_get_prefixed_tools_excluding() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension("ext_a".to_string(), Arc::new(MockClient {}))
            .await;
        extension_manager
            .add_mock_extension("ext_b".to_string(), Arc::new(MockClient {}))
            .await;

        let tools = extension_manager
            .get_prefixed_tools_excluding("test-session-id", "ext_a")
            .await
            .unwrap();
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

        assert!(!tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(tool_names.iter().any(|n| n.starts_with("ext_b__")));
    }

    #[tokio::test]
    async fn test_get_prefixed_tools_by_extension_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension("ext_a".to_string(), Arc::new(MockClient {}))
            .await;
        extension_manager
            .add_mock_extension("ext_b".to_string(), Arc::new(MockClient {}))
            .await;

        let tools = extension_manager
            .get_prefixed_tools("test-session-id", Some("ext_a".to_string()))
            .await
            .unwrap();
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

        assert!(tool_names.iter().any(|n| n.starts_with("ext_a__")));
        assert!(!tool_names.iter().any(|n| n.starts_with("ext_b__")));
    }

    #[tokio::test]
    async fn test_resolve_tool_error_includes_available_tools() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extension_manager =
            ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());

        extension_manager
            .add_mock_extension("ext_a".to_string(), Arc::new(MockClient {}))
            .await;

        let result = extension_manager
            .resolve_tool("test-session-id", "definitely_not_a_real_tool")
            .await;

        let err = match result {
            Ok(_) => panic!("resolve_tool should fail for an unknown name"),
            Err(e) => e,
        };

        let msg = err.message.to_string();
        assert!(
            msg.contains("definitely_not_a_real_tool"),
            "error should echo the bad name; got: {msg}"
        );
        assert!(
            msg.contains("ext_a__"),
            "error should list at least one real tool name; got: {msg}"
        );
    }

    #[test]
    fn test_remove_untrusted_mcp_app_meta_strips_spoofed_payload() {
        let mut result = CallToolResult::success(vec![]);
        result.meta = Some(Meta(
            serde_json::from_value(serde_json::json!({
                "goose": {
                    "mcpApp": {
                        "resourceUri": "ui://spoofed/app",
                    },
                    "other": true,
                },
                TRUSTED_TOOL_UPDATE_META_KEY: {
                    "mcpApp": {
                        "resourceUri": "ui://spoofed/internal",
                    },
                },
            }))
            .unwrap(),
        ));

        remove_untrusted_mcp_app_meta(&mut result);

        let meta = result.meta.expect("expected remaining meta");
        assert_eq!(meta.0.get(TRUSTED_TOOL_UPDATE_META_KEY), None);
        assert_eq!(
            meta.0.get("goose"),
            Some(&serde_json::json!({ "other": true }))
        );
    }

    #[test]
    fn test_insert_trusted_tool_update_meta_stores_backend_payload() {
        let mut result = CallToolResult::success(vec![]);
        let attachment = GooseMcpAppToolAttachment {
            tool_name: "weather__render".to_string(),
            extension_name: "weather".to_string(),
            resource_uri: "ui://weather/app".to_string(),
            tool_meta: None,
            resource_result: Some(serde_json::json!({
                "contents": [
                    {
                        "uri": "ui://weather/app",
                        "mimeType": "text/html;profile=mcp-app",
                        "text": "<div>Hello</div>",
                    },
                ],
            })),
            read_error: None,
        };

        insert_trusted_tool_update_meta(&mut result, &attachment);

        let meta = result.meta.expect("expected trusted meta");
        assert_eq!(
            meta.0.get(TRUSTED_TOOL_UPDATE_META_KEY),
            Some(&serde_json::json!({
                "mcpApp": {
                    "toolName": "weather__render",
                    "extensionName": "weather",
                    "resourceUri": "ui://weather/app",
                    "resourceResult": {
                        "contents": [
                            {
                                "uri": "ui://weather/app",
                                "mimeType": "text/html;profile=mcp-app",
                                "text": "<div>Hello</div>",
                            },
                        ],
                    },
                },
            })),
        );
    }

    #[tokio::test]
    async fn test_add_extension_noop_on_identical_config() {
        // When add_extension is called with a config that is byte-for-byte identical to
        // the already-loaded one, it must return Ok(()) without removing the extension.
        let temp_dir = tempfile::tempdir().unwrap();
        let em = Arc::new(ExtensionManager::new_without_provider(
            temp_dir.path().to_path_buf(),
        ));

        let config = ExtensionConfig::Frontend {
            name: "test-ext".to_string(),
            description: "original".to_string(),
            tools: vec![],
            instructions: None,
            bundled: None,
            available_tools: vec![],
        };

        em.add_client(
            "test-ext".to_string(),
            config.clone(),
            Arc::new(MockClient {}),
            None,
            None,
        )
        .await;
        assert_eq!(em.extensions.lock().await.len(), 1);

        // Calling add_extension with the same config must be a no-op (Ok, count unchanged).
        let result = em.add_extension(config, None, None, None).await;
        assert!(result.is_ok(), "identical config should be a no-op");
        assert_eq!(
            em.extensions.lock().await.len(),
            1,
            "extension must not be removed on no-op"
        );
    }

    #[tokio::test]
    async fn test_add_extension_replaces_extension_on_config_change() {
        // When add_extension is called with an updated config (same name, different fields),
        // the existing extension must be removed so the caller can re-add with new config.
        let temp_dir = tempfile::tempdir().unwrap();
        let em = Arc::new(ExtensionManager::new_without_provider(
            temp_dir.path().to_path_buf(),
        ));

        let config_a = ExtensionConfig::Frontend {
            name: "test-ext".to_string(),
            description: "version-a".to_string(),
            tools: vec![],
            instructions: None,
            bundled: None,
            available_tools: vec![],
        };
        let config_b = ExtensionConfig::Frontend {
            name: "test-ext".to_string(),
            description: "version-b".to_string(), // changed
            tools: vec![],
            instructions: None,
            bundled: None,
            available_tools: vec![],
        };

        em.add_client(
            "test-ext".to_string(),
            config_a,
            Arc::new(MockClient {}),
            None,
            None,
        )
        .await;
        assert_eq!(em.extensions.lock().await.len(), 1);

        // add_extension with changed config attempts to create a new client (fails here
        // because Frontend configs cannot be added as server extensions), but must preserve
        // the old extension so the session isn't left without it.
        let result = em.add_extension(config_b, None, None, None).await;
        assert!(result.is_err(), "Frontend add_extension must return Err");
        assert_eq!(
            em.extensions.lock().await.len(),
            1,
            "old extension must be preserved when replacement client creation fails"
        );
    }

    fn transport_err(error: Box<dyn std::error::Error + Send + Sync>) -> ClientInitializeError {
        ClientInitializeError::TransportError {
            error: rmcp::transport::DynamicTransportError::from_parts(
                "test",
                std::any::TypeId::of::<()>(),
                error,
            ),
            context: "test context".into(),
        }
    }

    fn streamable_err(
        e: rmcp::transport::streamable_http_client::StreamableHttpError<reqwest::Error>,
    ) -> ClientInitializeError {
        transport_err(Box::new(e))
    }

    #[test]
    fn test_oauth_fallback_on_typed_auth_required() {
        let err = streamable_err(
            rmcp::transport::streamable_http_client::StreamableHttpError::AuthRequired(
                rmcp::transport::streamable_http_client::AuthRequiredError::new(
                    "Bearer realm=\"test\"".to_string(),
                ),
            ),
        );
        assert!(should_attempt_oauth_fallback(&Err(err)));
    }

    #[test]
    fn test_oauth_fallback_on_unexpected_response_http_401_prefix() {
        let err = streamable_err(
            rmcp::transport::streamable_http_client::StreamableHttpError::UnexpectedServerResponse(
                std::borrow::Cow::Borrowed("HTTP 401 Unauthorized"),
            ),
        );
        assert!(should_attempt_oauth_fallback(&Err(err)));
    }
}
