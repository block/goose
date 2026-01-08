use anyhow::Result;
use axum::http::{HeaderMap, HeaderName};
use chrono::{DateTime, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{future, FutureExt};
use rand::{distributions::Alphanumeric, Rng};
use rmcp::service::{ClientInitializeError, ServiceError};
use rmcp::transport::streamable_http_client::{
    AuthRequiredError, StreamableHttpClientTransportConfig, StreamableHttpError,
};
use rmcp::transport::{
    ConfigureCommandExt, DynamicTransportError, StreamableHttpClientTransport, TokioChildProcess,
};
use std::collections::HashMap;
use std::option::Option;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use super::extension::{
    ExtensionConfig, ExtensionError, ExtensionInfo, ExtensionResult, PlatformExtensionContext,
    ToolInfo, PLATFORM_EXTENSIONS,
};
use super::tool_execution::ToolCallResult;
use super::types::SharedProvider;
use crate::agents::extension::{Envs, ProcessExit};
use crate::agents::extension_malware_check;
use crate::agents::mcp_client::{McpClient, McpClientTrait};
use crate::config::search_path::SearchPaths;
use crate::config::{get_all_extensions, Config};
use crate::oauth::oauth_flow;
use crate::prompt_template;
use crate::subprocess::configure_command_no_window;
use rmcp::model::{
    CallToolRequestParam, Content, ErrorCode, ErrorData, GetPromptResult, Prompt, Resource,
    ResourceContents, ServerInfo, Tool,
};
use rmcp::transport::auth::AuthClient;
use schemars::_private::NoSerialize;
use serde_json::Value;

type McpClientBox = Arc<Mutex<Box<dyn McpClientTrait>>>;

struct Extension {
    pub config: ExtensionConfig,

    client: McpClientBox,
    server_info: Option<ServerInfo>,
    _temp_dir: Option<tempfile::TempDir>,
}

impl Extension {
    fn new(
        config: ExtensionConfig,
        client: McpClientBox,
        server_info: Option<ServerInfo>,
        temp_dir: Option<tempfile::TempDir>,
    ) -> Self {
        Self {
            client,
            config,
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
        self.server_info
            .as_ref()
            .and_then(|info| info.instructions.clone())
    }

    fn get_client(&self) -> McpClientBox {
        self.client.clone()
    }
}

/// Manages goose extensions / MCP clients and their interactions
pub struct ExtensionManager {
    extensions: Mutex<HashMap<String, Extension>>,
    context: Mutex<PlatformExtensionContext>,
    provider: SharedProvider,
}

/// A flattened representation of a resource used by the agent to prepare inference
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub client_name: String,      // The name of the client that owns the resource
    pub uri: String,              // The URI of the resource
    pub name: String,             // The name of the resource
    pub content: String,          // The content of the resource
    pub timestamp: DateTime<Utc>, // The timestamp of the resource
    pub priority: f32,            // The priority of the resource
    pub token_count: Option<u32>, // The token count of the resource (filled in by the agent)
}

impl ResourceItem {
    pub fn new(
        client_name: String,
        uri: String,
        name: String,
        content: String,
        timestamp: DateTime<Utc>,
        priority: f32,
    ) -> Self {
        Self {
            client_name,
            uri,
            name,
            content,
            timestamp,
            priority,
            token_count: None,
        }
    }
}

/// Sanitizes a string by replacing invalid characters with underscores.
/// Valid characters match [a-zA-Z0-9_-]
fn normalize(input: String) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        result.push(match c {
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => c,
            c if c.is_whitespace() => continue, // effectively "strip" whitespace
            _ => '_',                           // Replace any other non-ASCII character with '_'
        });
    }
    result.to_lowercase()
}

/// Generates extension name from server info; adds random suffix on collision.
fn generate_extension_name(
    server_info: Option<&ServerInfo>,
    name_exists: impl Fn(&str) -> bool,
) -> String {
    let base = server_info
        .and_then(|info| {
            let name = info.server_info.name.as_str();
            (!name.is_empty()).then(|| normalize(name.to_string()))
        })
        .unwrap_or_else(|| "unnamed".to_string());

    if !name_exists(&base) {
        return base;
    }

    let suffix: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    format!("{base}_{suffix}")
}

fn resolve_command(cmd: &str) -> PathBuf {
    SearchPaths::builder()
        .with_npm()
        .resolve(cmd)
        .unwrap_or_else(|_| {
            // let the OS raise the error
            PathBuf::from(cmd)
        })
}

fn require_str_parameter<'a>(v: &'a serde_json::Value, name: &str) -> Result<&'a str, ErrorData> {
    let v = v.get(name).ok_or_else(|| {
        ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} is required"),
            None,
        )
    })?;
    match v.as_str() {
        Some(r) => Ok(r),
        None => Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} must be a string"),
            None,
        )),
    }
}

pub fn get_parameter_names(tool: &Tool) -> Vec<String> {
    let mut names: Vec<String> = tool
        .input_schema
        .get("properties")
        .and_then(|props| props.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default();
    names.sort();
    names
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(None)))
    }
}

async fn child_process_client(
    mut command: Command,
    timeout: &Option<u64>,
    provider: SharedProvider,
) -> ExtensionResult<McpClient> {
    #[cfg(unix)]
    command.process_group(0);
    configure_command_no_window(&mut command);

    if let Ok(path) = SearchPaths::builder().path() {
        command.env("PATH", path);
    }

    let (transport, mut stderr) = TokioChildProcess::builder(command)
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stderr = stderr.take().ok_or_else(|| {
        ExtensionError::SetupError("failed to attach child process stderr".to_owned())
    })?;

    let stderr_task = tokio::spawn(async move {
        let mut all_stderr = Vec::new();
        stderr.read_to_end(&mut all_stderr).await?;
        Ok::<String, std::io::Error>(String::from_utf8_lossy(&all_stderr).into())
    });

    let client_result = McpClient::connect(
        transport,
        Duration::from_secs(timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT)),
        provider,
    )
    .await;

    match client_result {
        Ok(client) => Ok(client),
        Err(error) => {
            let error_task_out = stderr_task.await?;
            Err::<McpClient, ExtensionError>(match error_task_out {
                Ok(stderr_content) => ProcessExit::new(stderr_content, error).into(),
                Err(e) => e.into(),
            })
        }
    }
}

fn extract_auth_error(
    res: &Result<McpClient, ClientInitializeError>,
) -> Option<&AuthRequiredError> {
    match res {
        Ok(_) => None,
        Err(err) => match err {
            ClientInitializeError::TransportError {
                error: DynamicTransportError { error, .. },
                ..
            } => error
                .downcast_ref::<StreamableHttpError<reqwest::Error>>()
                .and_then(|auth_error| match auth_error {
                    StreamableHttpError::AuthRequired(auth_required_error) => {
                        Some(auth_required_error)
                    }
                    _ => None,
                }),
            _ => None,
        },
    }
}

/// Merge environment variables from direct envs and keychain-stored env_keys
async fn merge_environments(
    envs: &Envs,
    env_keys: &[String],
    ext_name: &str,
) -> Result<HashMap<String, String>, ExtensionError> {
    let mut all_envs = envs.get_env();
    let config_instance = Config::global();

    for key in env_keys {
        if all_envs.contains_key(key) {
            continue;
        }

        match config_instance.get(key, true) {
            Ok(value) => {
                if value.is_null() {
                    warn!(
                        key = %key,
                        ext_name = %ext_name,
                        "Secret key not found in config (returned null)."
                    );
                    continue;
                }

                if let Some(str_val) = value.as_str() {
                    all_envs.insert(key.clone(), str_val.to_string());
                } else {
                    warn!(
                        key = %key,
                        ext_name = %ext_name,
                        value_type = %value.get("type").and_then(|t| t.as_str()).unwrap_or("unknown"),
                        "Secret value is not a string; skipping."
                    );
                }
            }
            Err(e) => {
                error!(
                    key = %key,
                    ext_name = %ext_name,
                    error = %e,
                    "Failed to fetch secret from config."
                );
                return Err(ExtensionError::ConfigError(format!(
                    "Failed to fetch secret '{}' from config: {}",
                    key, e
                )));
            }
        }
    }

    Ok(all_envs)
}

/// Substitute environment variables in a string. Supports both ${VAR} and $VAR syntax.
fn substitute_env_vars(value: &str, env_map: &HashMap<String, String>) -> String {
    let mut result = value.to_string();

    let re_braces =
        regex::Regex::new(r"\$\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}").expect("valid regex");
    for cap in re_braces.captures_iter(value) {
        if let Some(var_name) = cap.get(1) {
            if let Some(env_value) = env_map.get(var_name.as_str()) {
                result = result.replace(&cap[0], env_value);
            }
        }
    }

    let re_simple = regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").expect("valid regex");
    for cap in re_simple.captures_iter(&result.clone()) {
        if let Some(var_name) = cap.get(1) {
            if !value.contains(&format!("${{{}}}", var_name.as_str())) {
                if let Some(env_value) = env_map.get(var_name.as_str()) {
                    result = result.replace(&cap[0], env_value);
                }
            }
        }
    }

    result
}

async fn create_streamable_http_client(
    uri: &str,
    timeout: Option<u64>,
    headers: &HashMap<String, String>,
    name: &str,
    all_envs: &HashMap<String, String>,
    provider: SharedProvider,
) -> ExtensionResult<Box<dyn McpClientTrait>> {
    let mut default_headers = HeaderMap::new();
    for (key, value) in headers {
        let substituted_value = substitute_env_vars(value, all_envs);
        default_headers.insert(
            HeaderName::try_from(key)
                .map_err(|_| ExtensionError::ConfigError(format!("invalid header: {}", key)))?,
            substituted_value.parse().map_err(|_| {
                ExtensionError::ConfigError(format!("invalid header value: {}", key))
            })?,
        );
    }

    let http_client = reqwest::Client::builder()
        .default_headers(default_headers)
        .build()
        .map_err(|_| ExtensionError::ConfigError("could not construct http client".to_string()))?;

    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig {
            uri: uri.into(),
            ..Default::default()
        },
    );

    let timeout_duration =
        Duration::from_secs(timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT));

    let client_res = McpClient::connect(transport, timeout_duration, provider.clone()).await;

    if extract_auth_error(&client_res).is_some() {
        let am = oauth_flow(&uri.to_string(), &name.to_string())
            .await
            .map_err(|_| ExtensionError::SetupError("auth error".to_string()))?;
        let auth_client = AuthClient::new(reqwest::Client::default(), am);
        let transport = StreamableHttpClientTransport::with_client(
            auth_client,
            StreamableHttpClientTransportConfig {
                uri: uri.into(),
                ..Default::default()
            },
        );
        Ok(Box::new(
            McpClient::connect(transport, timeout_duration, provider).await?,
        ))
    } else {
        Ok(Box::new(client_res?))
    }
}

async fn create_stdio_client(
    cmd: &str,
    args: &[String],
    all_envs: HashMap<String, String>,
    timeout: &Option<u64>,
    provider: SharedProvider,
) -> ExtensionResult<Box<dyn McpClientTrait>> {
    extension_malware_check::deny_if_malicious_cmd_args(cmd, args).await?;

    let resolved_cmd = resolve_command(cmd);
    let command = Command::new(resolved_cmd).configure(|command| {
        command.args(args).envs(all_envs);
    });

    Ok(Box::new(
        child_process_client(command, timeout, provider).await?,
    ))
}

impl ExtensionManager {
    pub fn new(provider: SharedProvider) -> Self {
        Self {
            extensions: Mutex::new(HashMap::new()),
            context: Mutex::new(PlatformExtensionContext {
                session_id: None,
                extension_manager: None,
            }),
            provider,
        }
    }

    /// Create a new ExtensionManager with no provider (useful for tests)
    pub fn new_without_provider() -> Self {
        Self::new(Arc::new(Mutex::new(None)))
    }

    pub async fn set_context(&self, context: PlatformExtensionContext) {
        *self.context.lock().await = context;
    }

    pub async fn get_context(&self) -> PlatformExtensionContext {
        self.context.lock().await.clone()
    }

    pub async fn supports_resources(&self) -> bool {
        self.extensions
            .lock()
            .await
            .values()
            .any(|ext| ext.supports_resources())
    }

    pub async fn add_extension(&self, config: ExtensionConfig) -> ExtensionResult<()> {
        let config_name = config.key().to_string();
        let sanitized_name = normalize(config_name.clone());

        if self.extensions.lock().await.contains_key(&sanitized_name) {
            return Ok(());
        }

        let mut temp_dir = None;

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
                ..
            } => {
                let all_envs = merge_environments(envs, env_keys, &sanitized_name).await?;
                create_streamable_http_client(
                    uri,
                    *timeout,
                    headers,
                    name,
                    &all_envs,
                    self.provider.clone(),
                )
                .await?
            }
            ExtensionConfig::Stdio {
                cmd,
                args,
                envs,
                env_keys,
                timeout,
                ..
            } => {
                let all_envs = merge_environments(envs, env_keys, &sanitized_name).await?;
                create_stdio_client(cmd, args, all_envs, timeout, self.provider.clone()).await?
            }
            ExtensionConfig::Builtin { name, timeout, .. } => {
                let cmd = std::env::current_exe()
                    .and_then(|path| {
                        path.to_str().map(|s| s.to_string()).ok_or_else(|| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UTF-8 in executable path",
                            )
                        })
                    })
                    .map_err(|e| {
                        ExtensionError::ConfigError(format!(
                            "Failed to resolve executable path: {}",
                            e
                        ))
                    })?;
                let command = Command::new(cmd).configure(|command| {
                    command.arg("mcp").arg(name);
                });
                Box::new(child_process_client(command, timeout, self.provider.clone()).await?)
            }
            ExtensionConfig::Platform { name, .. } => {
                let normalized_key = normalize(name.clone());
                let def = PLATFORM_EXTENSIONS
                    .get(normalized_key.as_str())
                    .ok_or_else(|| {
                        ExtensionError::ConfigError(format!("Unknown platform extension: {}", name))
                    })?;
                let context = self.get_context().await;
                (def.client_factory)(context)
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

                Box::new(child_process_client(command, timeout, self.provider.clone()).await?)
            }
            ExtensionConfig::Frontend { .. } => {
                return Err(ExtensionError::ConfigError(
                    "Invalid extension type: Frontend extensions cannot be added as server extensions".to_string()
                ));
            }
        };

        let server_info = client.get_info().cloned();

        // Only generate name from server info when config has no name (e.g., CLI --with-*-extension args)
        let mut extensions = self.extensions.lock().await;
        let final_name = if sanitized_name.is_empty() {
            generate_extension_name(server_info.as_ref(), |n| extensions.contains_key(n))
        } else {
            sanitized_name
        };
        extensions.insert(
            final_name,
            Extension::new(config, Arc::new(Mutex::new(client)), server_info, temp_dir),
        );

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
        self.extensions
            .lock()
            .await
            .insert(name, Extension::new(config, client, info, temp_dir));
    }

    /// Get extensions info for building the system prompt
    pub async fn get_extensions_info(&self) -> Vec<ExtensionInfo> {
        self.extensions
            .lock()
            .await
            .iter()
            .map(|(name, ext)| {
                ExtensionInfo::new(
                    name,
                    ext.get_instructions().unwrap_or_default().as_str(),
                    ext.supports_resources(),
                )
            })
            .collect()
    }

    /// Get aggregated usage statistics
    pub async fn remove_extension(&self, name: &str) -> ExtensionResult<()> {
        let sanitized_name = normalize(name.to_string());
        self.extensions.lock().await.remove(&sanitized_name);
        Ok(())
    }

    pub async fn get_extension_and_tool_counts(&self) -> (usize, usize) {
        let enabled_extensions_count = self.extensions.lock().await.len();

        let total_tools = self
            .get_prefixed_tools(None)
            .await
            .map(|tools| tools.len())
            .unwrap_or(0);

        (enabled_extensions_count, total_tools)
    }

    pub async fn list_extensions(&self) -> ExtensionResult<Vec<String>> {
        Ok(self.extensions.lock().await.keys().cloned().collect())
    }

    pub async fn is_extension_enabled(&self, name: &str) -> bool {
        self.extensions.lock().await.contains_key(name)
    }

    pub async fn get_extension_configs(&self) -> Vec<ExtensionConfig> {
        self.extensions
            .lock()
            .await
            .values()
            .map(|ext| ext.config.clone())
            .collect()
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(
        &self,
        extension_name: Option<String>,
    ) -> ExtensionResult<Vec<Tool>> {
        self.get_prefixed_tools_impl(extension_name, None).await
    }

    async fn get_prefixed_tools_impl(
        &self,
        extension_name: Option<String>,
        exclude: Option<&str>,
    ) -> ExtensionResult<Vec<Tool>> {
        // Filter clients based on the provided extension_name or include all if None
        let filtered_clients: Vec<_> = self
            .extensions
            .lock()
            .await
            .iter()
            .filter(|(name, _ext)| {
                if let Some(excluded) = exclude {
                    if name.as_str() == excluded {
                        return false;
                    }
                }

                if let Some(ref name_filter) = extension_name {
                    *name == name_filter
                } else {
                    true
                }
            })
            .map(|(name, ext)| (name.clone(), ext.config.clone(), ext.get_client()))
            .collect();

        let cancel_token = CancellationToken::default();
        let client_futures = filtered_clients.into_iter().map(|(name, config, client)| {
            let cancel_token = cancel_token.clone();
            task::spawn(async move {
                let mut tools = Vec::new();
                let client_guard = client.lock().await;
                let mut client_tools = client_guard.list_tools(None, cancel_token).await?;

                loop {
                    for tool in client_tools.tools {
                        let is_available = config.is_tool_available(&tool.name);

                        if is_available {
                            tools.push(Tool {
                                name: format!("{}__{}", name, tool.name).into(),
                                description: tool.description,
                                input_schema: tool.input_schema,
                                annotations: tool.annotations,
                                output_schema: tool.output_schema,
                                icons: tool.icons,
                                title: tool.title,
                                meta: tool.meta,
                            });
                        }
                    }

                    if client_tools.next_cursor.is_none() {
                        break;
                    }

                    client_tools = client_guard
                        .list_tools(client_tools.next_cursor, CancellationToken::default())
                        .await?;
                }

                Ok::<Vec<Tool>, ExtensionError>(tools)
            })
        });

        // Collect all results concurrently
        let results = future::join_all(client_futures).await;

        // Aggregate tools and handle errors
        let mut tools = Vec::new();
        for result in results {
            match result {
                Ok(Ok(client_tools)) => tools.extend(client_tools),
                Ok(Err(err)) => return Err(err),
                Err(join_err) => return Err(ExtensionError::from(join_err)),
            }
        }

        Ok(tools)
    }

    pub async fn get_prefixed_tools_excluding(&self, exclude: &str) -> ExtensionResult<Vec<Tool>> {
        self.get_prefixed_tools_impl(None, Some(exclude)).await
    }

    /// Get the extension prompt including client instructions
    pub async fn get_planning_prompt(&self, tools_info: Vec<ToolInfo>) -> String {
        let mut context: HashMap<&str, Value> = HashMap::new();
        context.insert("tools", serde_json::to_value(tools_info).unwrap());

        prompt_template::render_global_file("plan.md", &context).expect("Prompt should render")
    }

    /// Find and return a reference to the appropriate client for a tool call
    async fn get_client_for_tool(&self, prefixed_name: &str) -> Option<(String, McpClientBox)> {
        self.extensions
            .lock()
            .await
            .iter()
            .find(|(key, _)| prefixed_name.starts_with(*key))
            .map(|(name, extension)| (name.clone(), extension.get_client()))
    }

    // Function that gets executed for read_resource tool
    pub async fn read_resource_tool(
        &self,
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ErrorData> {
        let uri = require_str_parameter(&params, "uri")?;

        let extension_name = params.get("extension_name").and_then(|v| v.as_str());

        // If extension name is provided, we can just look it up
        if let Some(ext_name) = extension_name {
            let read_result = self
                .read_resource(uri, ext_name, cancellation_token.clone())
                .await?;

            let mut result = Vec::new();
            for content in read_result.contents {
                if let ResourceContents::TextResourceContents { text, .. } = content {
                    let content_str = format!("{}\n\n{}", uri, text);
                    result.push(Content::text(content_str));
                }
            }
            return Ok(result);
        }

        // If extension name is not provided, we need to search for the resource across all extensions
        // Loop through each extension and try to read the resource, don't raise an error if the resource is not found
        // TODO: do we want to find if a provided uri is in multiple extensions?
        // currently it will return the first match and skip any others

        // Collect extension names first to avoid holding the lock during iteration
        let extension_names: Vec<String> = self.extensions.lock().await.keys().cloned().collect();

        for extension_name in extension_names {
            let read_result = self
                .read_resource(uri, &extension_name, cancellation_token.clone())
                .await;
            match read_result {
                Ok(read_result) => {
                    let mut result = Vec::new();
                    for content in read_result.contents {
                        if let ResourceContents::TextResourceContents { text, .. } = content {
                            let content_str = format!("{}\n\n{}", uri, text);
                            result.push(Content::text(content_str));
                        }
                    }
                    return Ok(result);
                }
                Err(_) => continue,
            }
        }

        // None of the extensions had the resource so we raise an error
        let available_extensions = self
            .extensions
            .lock()
            .await
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "Resource with uri '{}' not found. Here are the available extensions: {}",
            uri, available_extensions
        );

        Err(ErrorData::new(
            ErrorCode::RESOURCE_NOT_FOUND,
            error_msg,
            None,
        ))
    }

    pub async fn read_resource(
        &self,
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

        let client_guard = client.lock().await;
        client_guard
            .read_resource(uri, cancellation_token)
            .await
            .map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Could not read resource with uri: {}", uri),
                    None,
                )
            })
    }

    pub async fn get_ui_resources(&self) -> Result<Vec<(String, Resource)>, ErrorData> {
        let mut ui_resources = Vec::new();

        let extensions_to_check: Vec<(String, McpClientBox)> = {
            let extensions = self.extensions.lock().await;
            extensions
                .iter()
                .map(|(name, ext)| (name.clone(), ext.get_client()))
                .collect()
        };

        for (extension_name, client) in extensions_to_check {
            let client_guard = client.lock().await;

            match client_guard
                .list_resources(None, CancellationToken::default())
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

        let client_guard = client.lock().await;
        client_guard
            .list_resources(None, cancellation_token)
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
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ErrorData> {
        let extension = params.get("extension").and_then(|v| v.as_str());

        match extension {
            Some(extension_name) => {
                // Handle single extension case
                self.list_resources_from_extension(extension_name, cancellation_token)
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
                            self.list_resources_from_extension(&name.clone(), token)
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

    pub async fn dispatch_tool_call(
        &self,
        tool_call: CallToolRequestParam,
        cancellation_token: CancellationToken,
    ) -> Result<ToolCallResult> {
        // Dispatch tool call based on the prefix naming convention
        let (client_name, client) =
            self.get_client_for_tool(&tool_call.name)
                .await
                .ok_or_else(|| {
                    ErrorData::new(ErrorCode::RESOURCE_NOT_FOUND, tool_call.name.clone(), None)
                })?;

        // rsplit returns the iterator in reverse, tool_name is then at 0
        let tool_name = tool_call
            .name
            .strip_prefix(client_name.as_str())
            .and_then(|s| s.strip_prefix("__"))
            .ok_or_else(|| {
                ErrorData::new(ErrorCode::RESOURCE_NOT_FOUND, tool_call.name.clone(), None)
            })?
            .to_string();

        if let Some(extension) = self.extensions.lock().await.get(&client_name) {
            if !extension.config.is_tool_available(&tool_name) {
                return Err(ErrorData::new(
                    ErrorCode::RESOURCE_NOT_FOUND,
                    format!(
                        "Tool '{}' is not available for extension '{}'",
                        tool_name, client_name
                    ),
                    None,
                )
                .into());
            }
        }

        let arguments = tool_call.arguments.clone();
        let client = client.clone();
        let notifications_receiver = client.lock().await.subscribe().await;

        let fut = async move {
            let client_guard = client.lock().await;
            client_guard
                .call_tool(&tool_name, arguments, cancellation_token)
                .await
                .map_err(|e| match e {
                    ServiceError::McpError(error_data) => error_data,
                    _ => {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), e.maybe_to_value())
                    }
                })
        };

        Ok(ToolCallResult {
            result: Box::new(fut.boxed()),
            notification_stream: Some(Box::new(ReceiverStream::new(notifications_receiver))),
        })
    }

    pub async fn list_prompts_from_extension(
        &self,
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

        let client_guard = client.lock().await;
        client_guard
            .list_prompts(None, cancellation_token)
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
        cancellation_token: CancellationToken,
    ) -> Result<HashMap<String, Vec<Prompt>>, ErrorData> {
        let mut futures = FuturesUnordered::new();

        let names: Vec<_> = self.extensions.lock().await.keys().cloned().collect();
        for extension_name in names {
            let token = cancellation_token.clone();
            futures.push(async move {
                (
                    extension_name.clone(),
                    self.list_prompts_from_extension(extension_name.as_str(), token)
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
        extension_name: &str,
        name: &str,
        arguments: Value,
        cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult> {
        let client = self
            .get_server_client(extension_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Extension {} not found", extension_name))?;

        let client_guard = client.lock().await;
        client_guard
            .get_prompt(name, arguments, cancellation_token)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get prompt: {}", e))
    }

    pub async fn search_available_extensions(&self) -> Result<Vec<Content>, ErrorData> {
        let mut output_parts = vec![];

        // First get disabled extensions from current config
        let mut disabled_extensions: Vec<String> = vec![];
        for extension in get_all_extensions() {
            if !extension.enabled {
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

        // Get currently enabled extensions that can be disabled
        let enabled_extensions: Vec<String> =
            self.extensions.lock().await.keys().cloned().collect();

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

    async fn get_server_client(&self, name: impl Into<String>) -> Option<McpClientBox> {
        self.extensions
            .lock()
            .await
            .get(&name.into())
            .map(|ext| ext.get_client())
    }

    pub async fn collect_moim(&self) -> Option<String> {
        // Use minute-level granularity to prevent conversation changes every second
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:00").to_string();
        let mut content = format!("<info-msg>\nIt is currently {}\n", timestamp);

        let platform_clients: Vec<(String, McpClientBox)> = {
            let extensions = self.extensions.lock().await;
            extensions
                .iter()
                .filter_map(|(name, extension)| {
                    if let ExtensionConfig::Platform { .. } = &extension.config {
                        Some((name.clone(), extension.get_client()))
                    } else {
                        None
                    }
                })
                .collect()
        };

        for (name, client) in platform_clients {
            let client_guard = client.lock().await;
            if let Some(moim_content) = client_guard.get_moim().await {
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
    use super::*;
    use rmcp::model::CallToolResult;
    use rmcp::model::{InitializeResult, JsonObject};
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
            let sanitized_name = normalize(name.clone());
            let config = ExtensionConfig::Builtin {
                name: name.clone(),
                display_name: Some(name.clone()),
                description: "built-in".to_string(),
                timeout: None,
                bundled: None,
                available_tools,
            };
            let extension = Extension::new(config, client, None, None);
            self.extensions
                .lock()
                .await
                .insert(sanitized_name, extension);
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
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListResourcesResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn read_resource(
            &self,
            _uri: &str,
            _cancellation_token: CancellationToken,
        ) -> Result<ReadResourceResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn list_tools(
            &self,
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
            name: &str,
            _arguments: Option<JsonObject>,
            _cancellation_token: CancellationToken,
        ) -> Result<CallToolResult, Error> {
            match name {
                "tool" | "test__tool" | "available_tool" | "hidden_tool" => Ok(CallToolResult {
                    content: vec![],
                    is_error: None,
                    structured_content: None,
                    meta: None,
                }),
                _ => Err(Error::TransportClosed),
            }
        }

        async fn list_prompts(
            &self,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListPromptsResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn get_prompt(
            &self,
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
    async fn test_get_client_for_tool() {
        let extension_manager = ExtensionManager::new_without_provider();

        // Add some mock clients using the helper method
        extension_manager
            .add_mock_extension(
                "test_client".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        extension_manager
            .add_mock_extension(
                "__client".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        extension_manager
            .add_mock_extension(
                "__cli__ent__".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        extension_manager
            .add_mock_extension(
                "client 🚀".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        // Test basic case
        assert!(extension_manager
            .get_client_for_tool("test_client__tool")
            .await
            .is_some());

        // Test leading underscores
        assert!(extension_manager
            .get_client_for_tool("__client__tool")
            .await
            .is_some());

        // Test multiple underscores in client name, and ending with __
        assert!(extension_manager
            .get_client_for_tool("__cli__ent____tool")
            .await
            .is_some());

        // Test unicode in tool name, "client 🚀" should become "client_"
        assert!(extension_manager
            .get_client_for_tool("client___tool")
            .await
            .is_some());
    }

    #[tokio::test]
    async fn test_dispatch_tool_call() {
        // test that dispatch_tool_call parses out the sanitized name correctly, and extracts
        // tool_names
        let extension_manager = ExtensionManager::new_without_provider();

        // Add some mock clients using the helper method
        extension_manager
            .add_mock_extension(
                "test_client".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        extension_manager
            .add_mock_extension(
                "__cli__ent__".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        extension_manager
            .add_mock_extension(
                "client 🚀".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
            )
            .await;

        // verify a normal tool call
        let tool_call = CallToolRequestParam {
            name: "test_client__tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = CallToolRequestParam {
            name: "test_client__test__tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // verify a multiple underscores dispatch
        let tool_call = CallToolRequestParam {
            name: "__cli__ent____tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // Test unicode in tool name, "client 🚀" should become "client_"
        let tool_call = CallToolRequestParam {
            name: "client___tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = CallToolRequestParam {
            name: "client___test__tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // this should error out, specifically for an ToolError::ExecutionError
        let invalid_tool_call = CallToolRequestParam {
            name: "client___tools".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(invalid_tool_call, CancellationToken::default())
            .await
            .unwrap()
            .result
            .await;
        assert!(matches!(
            result,
            Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                ..
            })
        ));

        // this should error out, specifically with an ToolError::NotFound
        // this client doesn't exist
        let invalid_tool_call = CallToolRequestParam {
            name: "_client__tools".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(invalid_tool_call, CancellationToken::default())
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
        let extension_manager = ExtensionManager::new_without_provider();

        // Only "available_tool" should be available to the LLM
        let available_tools = vec!["available_tool".to_string()];

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
                available_tools,
            )
            .await;

        let tools = extension_manager.get_prefixed_tools(None).await.unwrap();

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
        let extension_manager = ExtensionManager::new_without_provider();

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
                vec![], // Empty available_tools means all tools are available by default
            )
            .await;

        let tools = extension_manager.get_prefixed_tools(None).await.unwrap();

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
        let extension_manager = ExtensionManager::new_without_provider();

        let available_tools = vec!["available_tool".to_string()];

        extension_manager
            .add_mock_extension_with_tools(
                "test_extension".to_string(),
                Arc::new(Mutex::new(Box::new(MockClient {}))),
                available_tools,
            )
            .await;

        // Try to call an unavailable tool
        let unavailable_tool_call = CallToolRequestParam {
            name: "test_extension__tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(unavailable_tool_call, CancellationToken::default())
            .await;

        // Should return RESOURCE_NOT_FOUND error
        if let Err(err) = result {
            let tool_err = err.downcast_ref::<ErrorData>().expect("Expected ErrorData");
            assert_eq!(tool_err.code, ErrorCode::RESOURCE_NOT_FOUND);
            assert!(tool_err.message.contains("is not available"));
        } else {
            panic!("Expected ErrorData with ErrorCode::RESOURCE_NOT_FOUND");
        }

        // Try to call an available tool - should succeed
        let available_tool_call = CallToolRequestParam {
            name: "test_extension__available_tool".to_string().into(),
            arguments: Some(object!({})),
        };

        let result = extension_manager
            .dispatch_tool_call(available_tool_call, CancellationToken::default())
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

    mod generate_extension_name_tests {
        use super::*;
        use rmcp::model::Implementation;
        use test_case::test_case;

        fn make_info(name: &str) -> ServerInfo {
            ServerInfo {
                server_info: Implementation {
                    name: name.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        #[test_case(Some("kiwi-mcp-server"), None, "^kiwi-mcp-server$" ; "already normalized server name")]
        #[test_case(Some("Context7"), None, "^context7$" ; "mixed case normalized")]
        #[test_case(Some("@huggingface/mcp-services"), None, "^_huggingface_mcp-services$" ; "special chars normalized")]
        #[test_case(None, None, "^unnamed$" ; "no server info falls back")]
        #[test_case(Some(""), None, "^unnamed$" ; "empty server name falls back")]
        #[test_case(Some("github-mcp-server"), Some("github-mcp-server"), r"^github-mcp-server_[A-Za-z0-9]{6}$" ; "duplicate adds suffix")]
        fn test_generate_name(server_name: Option<&str>, collision: Option<&str>, expected: &str) {
            let info = server_name.map(make_info);
            let result = generate_extension_name(info.as_ref(), |n| collision == Some(n));
            let re = regex::Regex::new(expected).unwrap();
            assert!(re.is_match(&result));
        }
    }

    #[tokio::test]
    async fn test_collect_moim_uses_minute_granularity() {
        let em = ExtensionManager::new_without_provider();

        if let Some(moim) = em.collect_moim().await {
            // Timestamp should end with :00 (seconds fixed to 00)
            assert!(
                moim.contains(":00\n"),
                "Timestamp should use minute granularity"
            );
        }
    }
}
