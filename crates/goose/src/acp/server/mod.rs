use crate::acp::custom_requests::*;
use crate::acp::fs::AcpTools;
use crate::acp::tools::AcpAwareToolMeta;
use crate::acp::{PermissionDecision, ACP_CURRENT_MODEL};
use crate::agents::extension::{Envs, PLATFORM_EXTENSIONS};
use crate::agents::extension_manager::TRUSTED_TOOL_UPDATE_META_KEY;
use crate::agents::mcp_client::{GooseMcpHostInfo, McpClientTrait};
use crate::agents::platform_extensions::developer::DeveloperClient;
use crate::agents::{Agent, AgentConfig, ExtensionConfig, GoosePlatform, SessionConfig};
use crate::config::base::CONFIG_YAML_NAME;
use crate::config::extensions::get_enabled_extensions_with_config;
use crate::config::paths::Paths;
use crate::config::permission::PermissionManager;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{ActionRequiredData, Message, MessageContent, ToolRequest};
use crate::mcp_utils::ToolResult;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::Provider;
use crate::providers::inventory::{
    InventoryIdentity, ProviderInventoryEntry, ProviderInventoryService, RefreshJobPlan,
    RefreshPlan, RefreshSkipReason,
};
use crate::session::session_manager::{SessionListCursor, SessionType};
use crate::session::{EnabledExtensionsState, Session, SessionManager};
use crate::source_roots::SourceRoot;
use crate::utils::sanitize_unicode_tags;
use agent_client_protocol::schema::{
    AgentCapabilities, Annotations, AuthMethod, AuthMethodAgent, AuthenticateRequest,
    AuthenticateResponse, AvailableCommand, AvailableCommandInput, AvailableCommandsUpdate,
    BlobResourceContents, CancelNotification, CloseSessionRequest, CloseSessionResponse,
    ConfigOptionUpdate, Content, ContentBlock, ContentChunk, CurrentModeUpdate, EmbeddedResource,
    EmbeddedResourceResource, FileSystemCapabilities, ForkSessionRequest, ForkSessionResponse,
    ImageContent, InitializeRequest, InitializeResponse, ListSessionsRequest, ListSessionsResponse,
    LoadSessionRequest, LoadSessionResponse, McpCapabilities, McpServer, Meta, ModelId, ModelInfo,
    NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind,
    PromptCapabilities, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, ResourceLink, SessionCapabilities, SessionCloseCapabilities,
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption, SessionId,
    SessionInfo, SessionInfoUpdate, SessionListCapabilities, SessionMode, SessionModeId,
    SessionModeState, SessionModelState, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse, StopReason,
    TextContent, TextResourceContents, ToolCall, ToolCallContent, ToolCallId, ToolCallLocation,
    ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind, UnstructuredCommandInput,
    Usage, UsageUpdate,
};
use agent_client_protocol::util::MatchDispatchFrom;
use agent_client_protocol::{
    Agent as SacpAgent, ByteStreams, Client, ConnectionTo, Dispatch, HandleDispatchFrom, Handled,
    Responder,
};
use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fs_err as fs;
use futures::future::{BoxFuture, Either};
use futures::stream::{self, StreamExt};
use futures::FutureExt;
use rmcp::model::{
    AnnotateAble, CallToolResult, RawContent, RawTextContent, ResourceContents, Role,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strum::{EnumMessage, VariantNames};
use tokio::sync::{Mutex, OnceCell};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

mod config;
mod custom_dispatch;
mod dictation;
mod dispatch;
mod extensions;
mod onboarding;
mod providers;
mod resources;
mod sessions;
mod sources;
mod tools;

mod protocol;
mod provider_config;
mod session_types;
mod tool_calls;
pub use provider_config::*;
pub use session_types::*;
pub use tool_calls::*;

pub type AcpProviderFactory = Arc<
    dyn Fn(
            String,
            crate::model::ModelConfig,
            Vec<ExtensionConfig>,
            Option<PathBuf>,
        ) -> BoxFuture<'static, Result<Arc<dyn Provider>>>
        + Send
        + Sync,
>;

const SESSION_LIST_PAGE_SIZE: usize = 50;
const ACP_SESSION_LIST_TYPES: [SessionType; 3] =
    [SessionType::User, SessionType::Scheduled, SessionType::Acp];

/// Convenience conversions from any `Display` error into an `agent_client_protocol::Error`.
///
/// Replaces the repetitive `.internal_err()`
/// pattern. Use `.internal_err()?` for server-side failures and `.invalid_params_err()?`
/// for bad client input. For custom messages use `.internal_err_ctx("context")?`.
#[allow(dead_code)]
trait ResultExt<T> {
    fn internal_err(self) -> Result<T, agent_client_protocol::Error>;
    fn invalid_params_err(self) -> Result<T, agent_client_protocol::Error>;
    fn internal_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error>;
    fn invalid_params_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn internal_err(self) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| agent_client_protocol::Error::internal_error().data(e.to_string()))
    }
    fn invalid_params_err(self) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| agent_client_protocol::Error::invalid_params().data(e.to_string()))
    }
    fn internal_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| {
            agent_client_protocol::Error::internal_error().data(format!("{context}: {e}"))
        })
    }
    fn invalid_params_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("{context}: {e}"))
        })
    }
}

const DEFAULT_PROVIDER_ID: &str = "goose";
const DEFAULT_PROVIDER_LABEL: &str = "Goose (Default)";
const PROVIDER_CONFIG_STATUS_CHECK_CONCURRENCY: usize = 16;

async fn ensure_refresh_identity_current(
    provider_id: &str,
    planned_identity: &InventoryIdentity,
) -> Result<()> {
    let current_identity = crate::providers::inventory_identity(provider_id)
        .await?
        .into_identity()?;
    if current_identity != *planned_identity {
        anyhow::bail!("provider inventory identity changed before refresh completed");
    }

    Ok(())
}

impl GooseAcpAgent {
    fn available_commands_update(working_dir: &std::path::Path) -> AvailableCommandsUpdate {
        let commands = crate::slash_commands::slash_command::list_acp_commands(Some(working_dir))
            .into_iter()
            .map(|entry| {
                let mut command = AvailableCommand::new(entry.name, entry.description);
                if let Some(input_hint) = entry.input_hint {
                    command = command.input(AvailableCommandInput::Unstructured(
                        UnstructuredCommandInput::new(input_hint),
                    ));
                }
                command
            })
            .collect();

        AvailableCommandsUpdate::new(commands)
    }

    fn send_available_commands_update(
        cx: &ConnectionTo<Client>,
        session_id: &SessionId,
        working_dir: &std::path::Path,
    ) -> Result<(), agent_client_protocol::Error> {
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AvailableCommandsUpdate(Self::available_commands_update(working_dir)),
        ))
    }

    pub fn permission_manager(&self) -> Arc<PermissionManager> {
        Arc::clone(&self.permission_manager)
    }

    // TODO: goose reads Paths::in_state_dir globally (e.g. RequestLog), ignoring this data_dir.
    pub async fn new(options: GooseAcpAgentOptions) -> Result<Self> {
        let session_manager = Arc::new(SessionManager::new(options.data_dir));

        // Eagerly initialize the SQLite pool so it's ready when providers/sessions need it.
        let storage_clone = session_manager.storage().clone();
        tokio::spawn(async move {
            let _ = storage_clone.pool().await;
        });

        let permission_manager = Arc::new(PermissionManager::new(options.config_dir.clone()));
        let provider_inventory = ProviderInventoryService::new(session_manager.storage().clone());

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            provider_factory: options.provider_factory,
            builtins: options.builtins,
            client_fs_capabilities: OnceCell::new(),
            client_terminal: OnceCell::new(),
            client_mcp_host_info: OnceCell::new(),
            use_login_shell_path: OnceCell::new(),
            config_dir: options.config_dir,
            session_manager,
            permission_manager,
            goose_mode: options.goose_mode,
            disable_session_naming: options.disable_session_naming,
            provider_inventory,
            goose_platform: options.goose_platform,
            additional_source_roots: options.additional_source_roots,
        })
    }

    fn load_config(&self) -> Result<Config> {
        Config::new(self.config_dir.join(CONFIG_YAML_NAME), "goose").map_err(Into::into)
    }

    fn config(&self) -> Result<Config, agent_client_protocol::Error> {
        self.load_config().internal_err_ctx("Failed to read config")
    }

    async fn create_provider(
        &self,
        provider_name: &str,
        model_config: crate::model::ModelConfig,
        extensions: Vec<ExtensionConfig>,
        working_dir: Option<PathBuf>,
    ) -> Result<Arc<dyn Provider>> {
        (self.provider_factory)(
            provider_name.to_string(),
            model_config,
            extensions,
            working_dir,
        )
        .await
    }

    async fn prepare_session_init_config(
        &self,
        resolved: &Result<(String, crate::model::ModelConfig), String>,
        mode_state: &SessionModeState,
        goose_session: &Session,
    ) -> (
        Option<SessionModelState>,
        Option<Vec<SessionConfigOption>>,
        Option<Arc<dyn Provider>>,
    ) {
        let Ok((provider_name, model_config)) = resolved else {
            return (None, None, None);
        };

        let Some(mut inventory) = self
            .provider_inventory
            .entry_for_provider(provider_name)
            .await
            .ok()
            .flatten()
        else {
            return (None, None, None);
        };

        let mut prebuilt_provider = None;
        if should_refresh_inventory_for_session_init(&inventory) {
            match self.load_config() {
                Ok(config) => {
                    let ext_state = EnabledExtensionsState::extensions_or_default(
                        Some(&goose_session.extension_data),
                        &config,
                    );
                    Config::global().invalidate_secrets_cache();
                    match self
                        .create_provider(
                            provider_name,
                            model_config.clone(),
                            ext_state,
                            Some(goose_session.working_dir.clone()),
                        )
                        .await
                    {
                        Ok(provider) => {
                            let provider_id = provider_name.clone();
                            prebuilt_provider = Some(provider.clone());
                            match self
                                .provider_inventory
                                .plan_refresh_jobs(std::slice::from_ref(&provider_id))
                                .await
                            {
                                Ok(plan)
                                    if plan
                                        .started
                                        .iter()
                                        .any(|job| job.provider_id == provider_id) =>
                                {
                                    let refresh_job = plan
                                        .started
                                        .into_iter()
                                        .find(|job| job.provider_id == provider_id);
                                    if let Some(refresh_job) = refresh_job {
                                        let mut refresh_guard = self
                                            .provider_inventory
                                            .refresh_guard(&refresh_job.identity);
                                        let fetch_result: Result<Vec<String>> =
                                            match ensure_refresh_identity_current(
                                                &provider_id,
                                                &refresh_job.identity,
                                            )
                                            .await
                                            {
                                                Ok(()) => match AssertUnwindSafe(
                                                    provider.fetch_recommended_models(),
                                                )
                                                .catch_unwind()
                                                .await
                                                {
                                                    Ok(Ok(models)) => Ok(models),
                                                    Ok(Err(error)) => {
                                                        Err(anyhow::anyhow!(error.to_string()))
                                                    }
                                                    Err(_) => Err(anyhow::anyhow!(
                                                        "provider inventory refresh task panicked"
                                                    )),
                                                },
                                                Err(error) => Err(error),
                                            };
                                        match fetch_result {
                                            Ok(models) => {
                                                if let Err(error) = self
                                                    .provider_inventory
                                                    .store_refreshed_models_for_identity(
                                                        &refresh_job.identity,
                                                        &models,
                                                    )
                                                    .await
                                                {
                                                    warn!(
                                                        provider = %provider_id,
                                                        error = %error,
                                                        "failed to store refreshed provider inventory during session init"
                                                    );
                                                } else {
                                                    refresh_guard.complete();
                                                }
                                            }
                                            Err(error) => {
                                                let error_message = error.to_string();
                                                if let Err(store_error) = self
                                                    .provider_inventory
                                                    .store_refresh_error_for_identity(
                                                        &refresh_job.identity,
                                                        error_message.clone(),
                                                    )
                                                    .await
                                                {
                                                    warn!(
                                                        provider = %provider_id,
                                                        error = %store_error,
                                                        "failed to store provider inventory refresh error during session init"
                                                    );
                                                } else {
                                                    refresh_guard.complete();
                                                }
                                                warn!(
                                                    provider = %provider_id,
                                                    error = %error_message,
                                                    "provider inventory refresh failed during session init"
                                                );
                                            }
                                        }
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => warn!(
                                    provider = %provider_id,
                                    error = %error,
                                    "failed to plan provider inventory refresh during session init"
                                ),
                            }

                            if let Ok(Some(refreshed_inventory)) = self
                                .provider_inventory
                                .entry_for_provider(provider_name)
                                .await
                            {
                                inventory = refreshed_inventory;
                            }
                        }
                        Err(error) => warn!(
                            provider = %provider_name,
                            error = %error,
                            "failed to initialize provider during synchronous inventory refresh"
                        ),
                    }
                }
                Err(error) => warn!(
                    provider = %provider_name,
                    error = %error,
                    "failed to load config during synchronous inventory refresh"
                ),
            }
        }

        let (model_state, config_options) = build_eager_config_from_inventory(
            provider_name,
            model_config.model_name.as_str(),
            &inventory,
            mode_state,
            goose_session,
        )
        .await;
        (Some(model_state), Some(config_options), prebuilt_provider)
    }

    fn spawn_agent_setup(
        &self,
        cx: &ConnectionTo<Client>,
        agent_tx: tokio::sync::watch::Sender<AgentSetupSignal>,
        req: AgentSetupRequest,
    ) {
        let AgentSetupRequest {
            session_id,
            goose_session,
            mcp_servers,
            resolved_provider,
            prebuilt_provider,
        } = req;

        let goose_mode = goose_session.goose_mode;
        let setup_session_id = goose_session.id.clone();
        let agent_session_id = SessionId::new(setup_session_id.clone());
        let sid = sid_short(session_id.0.as_ref());

        let cx = cx.clone();
        let sessions = Arc::clone(&self.sessions);
        let session_manager = Arc::clone(&self.session_manager);
        let permission_manager = Arc::clone(&self.permission_manager);
        let config_dir = self.config_dir.clone();
        let builtins = self.builtins.clone();
        let client_fs_capabilities = self
            .client_fs_capabilities
            .get()
            .cloned()
            .unwrap_or_default();
        let client_terminal = self.client_terminal.get().copied().unwrap_or(false);
        let client_mcp_host_info = self.client_mcp_host_info.get().cloned();
        let use_login_shell_path = self.use_login_shell_path.get().copied().unwrap_or(false);
        let provider_factory = Arc::clone(&self.provider_factory);
        let disable_session_naming = self.disable_session_naming;
        let goose_platform = self.goose_platform.clone();

        tokio::spawn(async move {
            let t_setup = std::time::Instant::now();
            debug!(target: "perf", sid = %sid, "perf: agent_setup start (background)");
            // Shared config — read once, used by both phases.
            let config = match Config::new(config_dir.join(CONFIG_YAML_NAME), "goose") {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    error!(error = %msg, "Background agent setup failed (config)");
                    let _ = agent_tx.send(Some(Err(msg)));
                    return;
                }
            };

            let session_name_update_tx =
                (!disable_session_naming).then(|| spawn_session_name_update_notifier(cx.clone()));

            // ── Phase 1: create agent + init provider (fast, ~55ms) ──────
            let phase1: Result<Arc<Agent>, String> = async {
                let agent = Arc::new(Agent::with_config(
                    AgentConfig::new(
                        session_manager,
                        permission_manager,
                        None,
                        goose_mode,
                        disable_session_naming,
                        goose_platform,
                    )
                    .with_mcp_host_info(client_mcp_host_info)
                    .with_session_name_update_tx(session_name_update_tx)
                    .with_use_login_shell_path(use_login_shell_path),
                ));

                // Init provider — reuse the pre-resolved name + model when
                // available (already computed in on_new_session), otherwise
                // fall back to reading config (e.g. load_session path).
                let (provider_name, model_config) = match resolved_provider {
                    Some(resolved) => resolved,
                    None => resolve_provider_and_model_from_config(&config, &goose_session).await?,
                };
                let ext_state = EnabledExtensionsState::extensions_or_default(
                    Some(&goose_session.extension_data),
                    &config,
                );
                let provider = match prebuilt_provider {
                    Some(provider) => provider,
                    None => provider_factory(
                        provider_name.to_string(),
                        model_config,
                        ext_state,
                        Some(goose_session.working_dir.clone()),
                    )
                    .await
                    .map_err(|e| e.to_string())?,
                };
                agent
                    .update_provider(provider.clone(), &goose_session.id)
                    .await
                    .map_err(|e| e.to_string())?;

                agent
                    .update_goose_mode(goose_mode, &setup_session_id)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(agent)
            }
            .await;

            let agent = match phase1 {
                Ok(agent) => {
                    // Signal ProviderReady — unblocks setProvider / update_provider
                    // while extensions continue loading below.
                    let _ =
                        agent_tx.send(Some(Ok(AgentSetupProgress::ProviderReady(agent.clone()))));
                    debug!(target: "perf", sid = %sid, ms = t_setup.elapsed().as_millis() as u64, "perf: agent_setup provider_ready (signalled)");
                    agent
                }
                Err(e) => {
                    error!(error = %e, "Background agent setup failed (provider init)");
                    debug!(target: "perf", sid = %sid, ms = t_setup.elapsed().as_millis() as u64, "perf: agent_setup failed (provider)");
                    let _ = agent_tx.send(Some(Err(e)));
                    return;
                }
            };

            // ── Phase 2: load extensions (slow, may take seconds) ────────
            let phase2: Result<(), String> = async {
                let mut extensions = get_enabled_extensions_with_config(&config);
                extensions.extend(builtins.iter().map(|b| builtin_to_extension_config(b)));

                let acp_developer = if (client_fs_capabilities.read_text_file
                    || client_fs_capabilities.write_text_file
                    || client_terminal)
                    && extensions.iter().any(|e| e.name() == "developer")
                {
                    let context = agent.extension_manager.get_context().clone();
                    match DeveloperClient::new(context) {
                        Ok(dev_client) => {
                            let client: Arc<dyn McpClientTrait> = Arc::new(AcpTools {
                                inner: Arc::new(dev_client),
                                cx: cx.clone(),
                                session_id: session_id.clone(),
                                fs_read: client_fs_capabilities.read_text_file,
                                fs_write: client_fs_capabilities.write_text_file,
                                terminal: client_terminal,
                            });
                            let dev_ext = extensions.iter().find(|e| e.name() == "developer");
                            let available_tools = dev_ext
                                .and_then(|e| match e {
                                    ExtensionConfig::Platform {
                                        available_tools, ..
                                    } => Some(available_tools.clone()),
                                    _ => None,
                                })
                                .unwrap_or_default();
                            let def = &PLATFORM_EXTENSIONS["developer"];
                            let config = ExtensionConfig::Platform {
                                name: def.name.into(),
                                description: def.description.into(),
                                display_name: Some(def.display_name.into()),
                                bundled: Some(true),
                                available_tools,
                            };
                            Some((client, config))
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to create developer client");
                            None
                        }
                    }
                } else {
                    None
                };

                let skip_developer = acp_developer.is_some();
                let sid_str = Some(agent_session_id.0.to_string());

                if skip_developer {
                    extensions.retain(|ext| ext.name() != "developer");
                }

                let ext_manager = &agent.extension_manager;
                let working_dir = goose_session.working_dir.clone();
                let extension_futures = extensions
                    .into_iter()
                    .map(|ext| {
                        let ext_manager = Arc::clone(ext_manager);
                        let sid_inner = sid_str.clone();
                        let working_dir = working_dir.clone();
                        async move {
                            let name = ext.name().to_string();
                            if let Err(e) = ext_manager
                                .add_extension(ext, Some(working_dir), None, sid_inner.as_deref())
                                .await
                            {
                                warn!(extension = %name, error = %e, "extension load failed");
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                futures::future::join_all(extension_futures).await;

                if let Some((client, config)) = acp_developer {
                    let info = client.get_info().cloned();
                    agent
                        .extension_manager
                        .add_client("developer".into(), config, client, info, None)
                        .await;
                }

                GooseAcpAgent::add_mcp_extensions(&agent, mcp_servers, &setup_session_id)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(())
            }
            .await;

            if let Err(e) = &phase2 {
                // Extension failures are non-fatal — individual failures are
                // already logged as warnings.  Log the top-level error but
                // don't block the session: the provider is ready and the agent
                // is usable.
                error!(error = %e, "Background agent setup: extension phase had errors");
            }

            // Promote the handle to Ready and apply any working directory that
            // was set while we were loading — regardless of phase-2 outcome,
            // since the agent (with its provider) is fully usable.
            {
                let mut locked = sessions.lock().await;
                if let Some(session) = locked.get_mut(session_id.0.as_ref()) {
                    if let Some(dir) = session.pending_working_dir.take() {
                        agent.extension_manager.update_working_dir(&dir).await;
                    }
                    session.agent = AgentHandle::Ready(agent.clone());
                }
            }

            let _ = agent_tx.send(Some(Ok(AgentSetupProgress::FullyReady(agent))));
            debug!(
                target: "perf",
                sid = %sid,
                ms = t_setup.elapsed().as_millis() as u64,
                "perf: agent_setup done{}",
                if phase2.is_err() { " (with extension errors)" } else { "" }
            );
        });
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    /// Convert ACP prompt content blocks into a user message.
    fn convert_acp_prompt_to_message(prompt: &[ContentBlock]) -> Message {
        let mut message = Message::user();
        for block in prompt {
            match block {
                ContentBlock::Text(text) => {
                    let annotated = if let Some(ref ann) = text.annotations {
                        let audience: Vec<Role> = ann
                            .audience
                            .as_ref()
                            .map(|roles| {
                                roles
                                    .iter()
                                    .filter_map(|r| match r {
                                        agent_client_protocol::schema::Role::Assistant => {
                                            Some(Role::Assistant)
                                        }
                                        agent_client_protocol::schema::Role::User => {
                                            Some(Role::User)
                                        }
                                        _ => None,
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let raw = RawTextContent {
                            text: sanitize_unicode_tags(&text.text),
                            meta: None,
                        };
                        if audience.is_empty() {
                            raw.no_annotation()
                        } else {
                            raw.no_annotation().with_audience(audience)
                        }
                    } else {
                        // No annotations — regular user text.
                        let sanitized = sanitize_unicode_tags(&text.text);
                        RawTextContent {
                            text: sanitized,
                            meta: None,
                        }
                        .no_annotation()
                    };
                    message = message.with_content(MessageContent::Text(annotated));
                }
                ContentBlock::Image(image) => {
                    message = message.with_image(&image.data, &image.mime_type);
                }
                ContentBlock::Resource(resource) => {
                    if let EmbeddedResourceResource::TextResourceContents(text_resource) =
                        &resource.resource
                    {
                        let header = format!("--- Resource: {} ---\n", text_resource.uri);
                        let content = format!("{}{}\n---\n", header, text_resource.text);
                        message = message.with_text(&content);
                    }
                }
                ContentBlock::ResourceLink(link) => {
                    if let Some(text) = read_resource_link(link.clone()) {
                        message = message.with_text(text);
                    }
                }
                ContentBlock::Audio(..) | _ => (),
            }
        }
        message
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_message_content(
        &self,
        content_item: &MessageContent,
        session_id: &SessionId,
        session_id_str: &str,
        message_id: Option<&str>,
        agent: &Arc<Agent>,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        match content_item {
            MessageContent::Text(text) => {
                cx.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                        TextContent::new(text.text.clone()),
                    ))),
                ))?;
            }
            MessageContent::ToolRequest(tool_request) => {
                self.handle_tool_request(
                    tool_request,
                    session_id,
                    session_id_str,
                    message_id,
                    session,
                    cx,
                )
                .await?;
            }
            MessageContent::ToolResponse(tool_response) => {
                self.handle_tool_response(
                    tool_response,
                    session_id,
                    session_id_str,
                    message_id,
                    session,
                    cx,
                )
                .await?;
            }
            MessageContent::Thinking(thinking) => {
                cx.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                        TextContent::new(thinking.thinking.clone()),
                    ))),
                ))?;
            }
            MessageContent::ActionRequired(action_required) => {
                if let ActionRequiredData::ToolConfirmation {
                    id,
                    tool_name,
                    arguments,
                    prompt,
                } = &action_required.data
                {
                    self.handle_tool_permission_request(
                        cx,
                        agent,
                        session_id,
                        id.clone(),
                        tool_name.clone(),
                        arguments.clone(),
                        prompt.clone(),
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_tool_request(
        &self,
        tool_request: &crate::conversation::message::ToolRequest,
        session_id: &SessionId,
        session_id_for_persist: &str,
        message_id: Option<&str>,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        session
            .tool_requests
            .insert(tool_request.id.clone(), tool_request.clone());

        let pending_tool_call = pending_tool_call_from_request(tool_request);
        let initial_tool_call = pending_tool_call
            .tool_call
            .meta(pending_tool_call.identity_meta.clone());
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(initial_tool_call),
        ))?;

        if Config::global()
            .get_goose_disable_tool_call_summary()
            .unwrap_or(false)
        {
            return Ok(());
        }

        if let Ok(tool_call) = &tool_request.tool_call {
            let agent = match &session.agent {
                AgentHandle::Ready(a) => a.clone(),
                AgentHandle::Loading(_) => return Ok(()),
            };
            let sid = session_id.clone();
            let request_id = tool_request.id.clone();
            let cx = cx.clone();
            let name = tool_call.name.to_string();
            let identity_meta = pending_tool_call.identity_meta.clone();
            let fallback_title = pending_tool_call.fallback_title.clone();
            let session_id_for_persist = session_id_for_persist.to_string();
            let message_id_for_persist = message_id.map(|s| s.to_string());
            let session_manager = self.session_manager.clone();
            let args_json = tool_call
                .arguments
                .as_ref()
                .map(|a| {
                    let s = serde_json::to_string(a).unwrap_or_default();
                    if s.len() > 300 {
                        format!("{}…", crate::utils::safe_truncate(&s, 300))
                    } else {
                        s
                    }
                })
                .unwrap_or_default();

            tokio::spawn(async move {
                let (title, from_llm) = match agent.provider().await {
                    Ok(provider) => {
                        if provider.manages_own_context() {
                            return;
                        }

                        let system =
                            "Summarize this tool call in a short lowercase phrase (3-8 words). \
                             No punctuation. No quotes. Examples: reading project configuration, \
                             checking network connectivity, listing files in src directory";
                        let user_text = format!("Tool: {name}\nArguments: {args_json}");
                        let message = Message::user().with_text(&user_text);
                        // The fast model occasionally returns an empty response
                        // under load (rate limiting, transient network). One
                        // retry with a short backoff is enough to recover the
                        // common cases without paying for the regular model.
                        let mut llm_outcome: Option<String> = None;
                        for attempt in 0..2 {
                            match provider
                                .complete_fast(&sid.0, system, std::slice::from_ref(&message), &[])
                                .await
                            {
                                Ok((response, _)) => {
                                    let summary: String = response
                                        .content
                                        .iter()
                                        .filter_map(|c: &MessageContent| c.as_text())
                                        .collect::<String>()
                                        .trim()
                                        .to_string();
                                    if !summary.is_empty() {
                                        llm_outcome = Some(summary);
                                        break;
                                    }
                                    if attempt == 0 {
                                        warn!(
                                            "tool call summary: fast_complete returned empty for {request_id} ({name}), retrying once",
                                        );
                                        tokio::time::sleep(std::time::Duration::from_millis(150))
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    if attempt == 0 {
                                        warn!(
                                            "tool call summary: fast_complete errored for {request_id} ({name}): {e}, retrying once",
                                        );
                                        tokio::time::sleep(std::time::Duration::from_millis(150))
                                            .await;
                                    } else {
                                        warn!(
                                            "tool call summary: fast_complete errored for {request_id} ({name}) after retry: {e}",
                                        );
                                    }
                                }
                            }
                        }
                        match llm_outcome {
                            Some(summary) => (summary, true),
                            None => {
                                warn!(
                                    "tool call summary: falling back to deterministic title for {request_id} ({name}) — replay will not show an LLM summary for this call",
                                );
                                (fallback_title.clone(), false)
                            }
                        }
                    }
                    Err(e) => {
                        warn!("tool call summary: failed to get provider: {e}");
                        (fallback_title.clone(), false)
                    }
                };

                let fields = ToolCallUpdateFields::new().title(title.clone());
                let _ = cx.send_notification(SessionNotification::new(
                    sid,
                    SessionUpdate::ToolCallUpdate(
                        ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields)
                            .meta(identity_meta),
                    ),
                ));

                // Best-effort persistence: only persist the LLM-generated title
                // (not the deterministic fallback) so reload uses fallback_title
                // for older or failed cases just like today.
                if from_llm {
                    if let Some(msg_id) = message_id_for_persist {
                        let patch = serde_json::json!({
                            crate::conversation::message::TOOL_META_TITLE_KEY: title,
                        });
                        if let Err(e) = session_manager
                            .update_tool_request_meta(
                                &session_id_for_persist,
                                &msg_id,
                                &request_id,
                                patch,
                            )
                            .await
                        {
                            warn!(
                                "tool call summary: persist failed for {request_id} in {msg_id}: {e}",
                            );
                        }
                    } else {
                        warn!(
                            "tool call summary: missing message_id for {request_id} — title will not survive reload",
                        );
                    }
                }
            });
        }

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &crate::conversation::message::ToolResponse,
        session_id: &SessionId,
        session_id_str: &str,
        message_id: Option<&str>,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        let status = match &tool_response.tool_result {
            Ok(result) if result.is_error == Some(true) => ToolCallStatus::Failed,
            Ok(_) => ToolCallStatus::Completed,
            Err(_) => ToolCallStatus::Failed,
        };

        let mut fields = ToolCallUpdateFields::new().status(status);
        if let Some(raw_output) = extract_tool_raw_output(&tool_response.tool_result) {
            fields = fields.raw_output(raw_output);
        }
        if !tool_response
            .tool_result
            .as_ref()
            .is_ok_and(|r| r.is_acp_aware())
        {
            let content = build_tool_call_content(&tool_response.tool_result);
            fields = fields.content(content);

            let locations = extract_locations_from_meta(tool_response).unwrap_or_else(|| {
                if let Some(tool_request) = session.tool_requests.get(&tool_response.id) {
                    extract_tool_locations(tool_request, tool_response)
                } else {
                    Vec::new()
                }
            });
            if !locations.is_empty() {
                fields = fields.locations(locations);
            }
        }

        let update = ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
            .meta(extract_tool_call_update_meta(tool_response));
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCallUpdate(update),
        ))?;

        // Chain summarization: when this response completes a multi-tool
        // chain, fire one LLM summary covering the run.
        session.responded_tool_ids.insert(tool_response.id.clone());
        self.maybe_summarize_chain(&tool_response.id, session_id, session_id_str, session, cx);
        let _ = message_id;

        Ok(())
    }

    /// If `tool_call_id` belongs to a multi-tool chain and every step in that
    /// chain has now had its response processed, spawn a single LLM
    /// summarization task that persists the chain summary on the first tool
    /// request and notifies the client. Idempotent — fires at most once per
    /// chain.
    fn maybe_summarize_chain(
        &self,
        tool_call_id: &str,
        session_id: &SessionId,
        _session_id_str: &str,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) {
        let Some(chain) = session.chain_membership.get(tool_call_id).cloned() else {
            warn!(
                "tool chain summary: skipped — no chain registered for tool_call_id {tool_call_id}",
            );
            return;
        };
        if !chain
            .ids
            .iter()
            .all(|id| session.responded_tool_ids.contains(id))
        {
            let total = chain.ids.len();
            let responded = chain
                .ids
                .iter()
                .filter(|id| session.responded_tool_ids.contains(*id))
                .count();
            let missing: Vec<&String> = chain
                .ids
                .iter()
                .filter(|id| !session.responded_tool_ids.contains(*id))
                .collect();
            warn!(
                "tool chain summary: waiting on {pending}/{total} responses for chain anchored at {anchor:?} (missing: {missing:?})",
                pending = total - responded,
                anchor = chain.ids.first(),
            );
            return;
        }
        let Some(first_id) = chain.ids.first() else {
            warn!("tool chain summary: skipped — empty chain.ids for tool_call_id {tool_call_id}");
            return;
        };
        if !session.summarized_chains.insert(first_id.clone()) {
            debug!("tool chain summary: chain anchored at {first_id} already summarized; skipping");
            return;
        }

        let agent = match &session.agent {
            AgentHandle::Ready(a) => a.clone(),
            AgentHandle::Loading(_) => {
                warn!(
                    "tool chain summary: agent still loading; skipping chain anchored at {first_id}",
                );
                return;
            }
        };

        // Snapshot (name, args_json) for each step in document order.
        let steps: Vec<(String, String)> = chain
            .ids
            .iter()
            .filter_map(|id| {
                let req = session.tool_requests.get(id)?;
                let tool_call = req.tool_call.as_ref().ok()?;
                let name = tool_call.name.to_string();
                let args = tool_call
                    .arguments
                    .as_ref()
                    .map(|a| serde_json::to_string(a).unwrap_or_default())
                    .unwrap_or_default();
                let args = if args.len() > 200 {
                    format!("{}…", crate::utils::safe_truncate(&args, 200))
                } else {
                    args
                };
                Some((name, args))
            })
            .collect();
        if steps.len() < 2 {
            return;
        }

        let identity_meta = session
            .tool_requests
            .get(first_id)
            .and_then(tool_call_identity_meta);

        let sid = session_id.clone();
        let chain_for_task = chain.clone();
        let cx = cx.clone();
        let session_manager = self.session_manager.clone();

        let first_id = first_id.clone();
        tokio::spawn(async move {
            let provider = match agent.provider().await {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        "tool chain summary: failed to get provider for chain anchored at {first_id}: {e}",
                    );
                    return;
                }
            };
            if provider.manages_own_context() {
                warn!(
                    "tool chain summary: provider manages own context; skipping chain anchored at {first_id}",
                );
                return;
            }

            let system = "Summarize this sequence of tool calls in a short lowercase phrase \
                 (3-8 words). No punctuation. No quotes. \
                 Examples: applied dark mode polish, scanned for security issues, \
                 refactored config loading";

            let mut user_text = String::from("Tool call sequence:\n");
            for (i, (name, args)) in steps.iter().enumerate() {
                user_text.push_str(&format!("Step {}: {} {}\n", i + 1, name, args));
            }
            let message = Message::user().with_text(&user_text);

            // Match the per-tool retry policy: one retry on empty/error keeps
            // the chain header reliable when the fast model is rate-limited or
            // momentarily flaky, without escalating to the regular model.
            let mut summary: Option<String> = None;
            for attempt in 0..2 {
                match provider
                    .complete_fast(&sid.0, system, std::slice::from_ref(&message), &[])
                    .await
                {
                    Ok((response, _)) => {
                        let s = response
                            .content
                            .iter()
                            .filter_map(|c: &MessageContent| c.as_text())
                            .collect::<String>()
                            .trim()
                            .to_string();
                        if !s.is_empty() {
                            summary = Some(s);
                            break;
                        }
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete returned empty for chain anchored at {first_id} ({} steps), retrying once",
                                steps.len(),
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        }
                    }
                    Err(e) => {
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_id}: {e}, retrying once",
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        } else {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_id} after retry: {e}",
                            );
                        }
                    }
                }
            }
            let Some(summary) = summary else {
                warn!(
                    "tool chain summary: no LLM summary produced for chain anchored at {first_id} — replay will fall back to the deterministic phrase",
                );
                return;
            };

            let count = chain_for_task.ids.len();
            let patch = serde_json::json!({
                crate::conversation::message::TOOL_META_CHAIN_SUMMARY_KEY: {
                    "summary": &summary,
                    "count": count,
                },
            });
            if let Err(e) = session_manager
                .update_tool_request_meta(&sid.0, &chain_for_task.message_id, &first_id, patch)
                .await
            {
                warn!(
                    "tool chain summary: persist failed for chain anchored at {first_id} in {}: {e}",
                    chain_for_task.message_id,
                );
            }

            let meta = with_tool_chain_summary_meta(identity_meta, &summary, count);
            let fields = ToolCallUpdateFields::new();
            let _ = cx.send_notification(SessionNotification::new(
                sid,
                SessionUpdate::ToolCallUpdate(
                    ToolCallUpdate::new(ToolCallId::new(first_id), fields).meta(meta),
                ),
            ));
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_tool_permission_request(
        &self,
        cx: &ConnectionTo<Client>,
        agent: &Arc<Agent>,
        session_id: &SessionId,
        request_id: String,
        tool_name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
        prompt: Option<String>,
    ) -> Result<(), agent_client_protocol::Error> {
        let cx = cx.clone();
        let agent = agent.clone();
        let session_id = session_id.clone();

        let formatted_name = format_tool_name(&tool_name);

        let mut fields = ToolCallUpdateFields::new()
            .title(formatted_name)
            .kind(ToolKind::default())
            .status(ToolCallStatus::Pending)
            .raw_input(serde_json::Value::Object(arguments));
        if let Some(p) = prompt {
            fields = fields.content(vec![ToolCallContent::Content(Content::new(
                ContentBlock::Text(TextContent::new(p)),
            ))]);
        }
        let tool_call_update = ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields);

        fn option(kind: PermissionOptionKind) -> PermissionOption {
            let id = serde_json::to_value(kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            PermissionOption::new(id.clone(), id, kind)
        }
        let options = vec![
            option(PermissionOptionKind::AllowAlways),
            option(PermissionOptionKind::AllowOnce),
            option(PermissionOptionKind::RejectOnce),
            option(PermissionOptionKind::RejectAlways),
        ];

        let permission_request =
            RequestPermissionRequest::new(session_id, tool_call_update, options);

        cx.send_request(permission_request)
            .on_receiving_result(move |result| async move {
                match result {
                    Ok(response) => {
                        agent
                            .handle_confirmation(
                                request_id,
                                outcome_to_confirmation(&response.outcome),
                            )
                            .await;
                        Ok(())
                    }
                    Err(e) => {
                        error!(error = ?e, "permission request failed");
                        agent
                            .handle_confirmation(
                                request_id,
                                PermissionConfirmation {
                                    principal_type: PrincipalType::Tool,
                                    permission: Permission::Cancel,
                                },
                            )
                            .await;
                        Ok(())
                    }
                }
            })?;

        Ok(())
    }

    fn is_builtin_agent_command(command: &str) -> bool {
        let normalized = command.trim_start_matches('/');

        crate::agents::execute_commands::list_commands()
            .iter()
            .any(|cmd| cmd.name == normalized)
            || crate::agents::execute_commands::COMPACT_TRIGGERS
                .iter()
                .filter_map(|trigger| trigger.strip_prefix('/'))
                .any(|trigger| trigger == normalized)
    }
}

fn outcome_to_confirmation(outcome: &RequestPermissionOutcome) -> PermissionConfirmation {
    PermissionConfirmation {
        principal_type: PrincipalType::Tool,
        permission: Permission::from(PermissionDecision::from(outcome)),
    }
}

fn extract_tool_call_update_meta(
    tool_response: &crate::conversation::message::ToolResponse,
) -> Option<Meta> {
    let tool_result = tool_response.tool_result.as_ref().ok()?;
    let goose_meta = tool_result
        .meta
        .as_ref()?
        .0
        .get(TRUSTED_TOOL_UPDATE_META_KEY)?
        .clone();
    let mut meta_map = serde_json::Map::new();
    meta_map.insert("goose".to_string(), goose_meta);
    Some(meta_map)
}

fn replay_message_meta(message: &Message) -> Meta {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "goose".to_string(),
        serde_json::Value::Object(replay_message_goose_meta(message)),
    );
    meta
}

fn replay_message_goose_meta(message: &Message) -> serde_json::Map<String, serde_json::Value> {
    let mut goose = serde_json::Map::new();
    goose.insert("created".to_string(), serde_json::json!(message.created));
    if let Some(id) = &message.id {
        goose.insert("messageId".to_string(), serde_json::json!(id));
    }
    goose
}

fn merge_replay_message_meta(meta: Option<Meta>, message: &Message) -> Meta {
    let replay_goose = replay_message_goose_meta(message);
    let mut meta = meta.unwrap_or_default();
    let goose_value = meta
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(goose) = goose_value {
        for (key, value) in replay_goose {
            goose.insert(key, value);
        }
    } else {
        *goose_value = serde_json::Value::Object(replay_goose);
    }

    meta
}

fn build_tool_call_content(tool_result: &ToolResult<CallToolResult>) -> Vec<ToolCallContent> {
    match tool_result {
        Ok(result) => result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Text(TextContent::new(val.text.clone())),
                ))),
                RawContent::Image(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Image(ImageContent::new(val.data.clone(), val.mime_type.clone())),
                ))),
                RawContent::Resource(val) => {
                    let resource = match &val.resource {
                        ResourceContents::TextResourceContents {
                            mime_type,
                            text,
                            uri,
                            ..
                        } => EmbeddedResourceResource::TextResourceContents(
                            TextResourceContents::new(text.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                        ResourceContents::BlobResourceContents {
                            mime_type,
                            blob,
                            uri,
                            ..
                        } => EmbeddedResourceResource::BlobResourceContents(
                            BlobResourceContents::new(blob.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                    };
                    Some(ToolCallContent::Content(Content::new(
                        ContentBlock::Resource(EmbeddedResource::new(resource)),
                    )))
                }
                RawContent::Audio(_) | RawContent::ResourceLink(_) => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn extract_tool_raw_output(tool_result: &ToolResult<CallToolResult>) -> Option<serde_json::Value> {
    tool_result
        .as_ref()
        .ok()
        .and_then(|result| result.structured_content.clone())
}

pub struct GooseAcpHandler {
    pub agent: Arc<GooseAcpAgent>,
}

pub fn serve<R, W>(
    agent: Arc<GooseAcpAgent>,
    read: R,
    write: W,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{
    Box::pin(async move {
        let handler = GooseAcpHandler { agent };

        SacpAgent
            .builder()
            .name("goose-acp")
            .with_handler(handler)
            .connect_to(ByteStreams::new(write, read))
            .await?;

        Ok(())
    })
}

pub async fn run(builtins: Vec<String>) -> Result<()> {
    info!("listening on stdio");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let server = crate::acp::server_factory::AcpServer::new(
        crate::acp::server_factory::AcpServerFactoryConfig {
            builtins,
            data_dir: Paths::data_dir(),
            config_dir: Paths::config_dir(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        },
    );
    let agent = server.create_agent().await?;
    serve(agent, incoming, outgoing).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{ToolRequest, ToolResponse};
    use agent_client_protocol::schema::{
        EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
        PermissionOptionId, ResourceLink, SelectedPermissionOutcome, SessionConfigSelectOption,
        SessionMode, SessionModeId, SessionModeState,
    };
    use rmcp::model::{CallToolRequestParams, Content as RmcpContent};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    #[test_case(
        McpServer::Stdio(
            McpServerStdio::new("github", "/path/to/github-mcp-server")
                .args(vec!["stdio".into()])
                .env(vec![EnvVariable::new("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::Stdio {
            name: "github".into(),
            description: String::new(),
            cmd: "/path/to/github-mcp-server".into(),
            args: vec!["stdio".into()],
            envs: Envs::new(
                [(
                    "GITHUB_PERSONAL_ACCESS_TOKEN".into(),
                    "ghp_xxxxxxxxxxxx".into()
                )]
                .into()
            ),
            env_keys: vec![],
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Http(
            McpServerHttp::new("github", "https://api.githubcopilot.com/mcp/")
                .headers(vec![HttpHeader::new("Authorization", "Bearer ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::StreamableHttp {
            name: "github".into(),
            description: String::new(),
            uri: "https://api.githubcopilot.com/mcp/".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([(
                "Authorization".into(),
                "Bearer ghp_xxxxxxxxxxxx".into()
            )]),
            timeout: None,
            socket: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Sse(McpServerSse::new("test-sse", "https://agent-fin.biodnd.com/sse")),
        Err("SSE is unsupported, migrate to streamable_http".to_string())
    )]
    fn test_mcp_server_to_extension_config(
        input: McpServer,
        expected: Result<ExtensionConfig, String>,
    ) {
        assert_eq!(mcp_server_to_extension_config(input), expected);
    }

    fn new_resource_link(content: &str) -> anyhow::Result<(ResourceLink, NamedTempFile)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let name = file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let uri = format!("file://{}", file.path().to_str().unwrap());
        let link = ResourceLink::new(name, uri);
        Ok((link, file))
    }

    #[test]
    fn test_read_resource_link_non_file_scheme() {
        let (link, file) = new_resource_link("print(\"hello, world\")").unwrap();

        let result = read_resource_link(link).unwrap();
        let expected = format!(
            "

# {}
```
print(\"hello, world\")
```",
            file.path().to_str().unwrap(),
        );

        assert_eq!(result, expected,)
    }

    #[test]
    fn test_format_tool_name_with_extension() {
        assert_eq!(format_tool_name("developer__edit"), "developer: edit");
        assert_eq!(
            format_tool_name("platform__manage_extensions"),
            "platform: manage extensions"
        );
        assert_eq!(format_tool_name("todo__write"), "todo: write");
    }

    #[test]
    fn test_format_tool_name_without_extension() {
        assert_eq!(format_tool_name("simple_tool"), "simple tool");
        assert_eq!(format_tool_name("another_name"), "another name");
        assert_eq!(format_tool_name("single"), "single");
    }

    #[test]
    fn test_summarize_tool_call_no_args() {
        assert_eq!(
            summarize_tool_call("developer__shell", None),
            "developer: shell"
        );
    }

    #[test]
    fn test_summarize_tool_call_with_path() {
        let args = serde_json::json!({"path": "/src/main.rs", "content": "fn main() {}"});
        assert_eq!(
            summarize_tool_call("developer__edit", Some(&args)),
            "developer: edit · /src/main.rs"
        );
    }

    #[test]
    fn test_summarize_tool_call_with_command() {
        let args = serde_json::json!({"command": "cargo build"});
        assert_eq!(
            summarize_tool_call("developer__shell", Some(&args)),
            "developer: shell · cargo build"
        );
    }

    #[test]
    fn test_tool_call_identity_meta_uses_goose_extension_metadata() {
        let request = ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("context7__query-docs")),
            metadata: None,
            tool_meta: Some(serde_json::json!({"goose_extension": "context7"})),
        };

        let meta = tool_call_identity_meta(&request).expect("expected metadata");

        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "toolCall": {
                    "toolName": "context7__query-docs",
                    "extensionName": "context7",
                },
            })),
        );
    }

    fn tool_request_block(id: &str) -> crate::conversation::message::MessageContent {
        crate::conversation::message::MessageContent::ToolRequest(ToolRequest {
            id: id.to_string(),
            tool_call: Ok(CallToolRequestParams::new("dummy")),
            metadata: None,
            tool_meta: None,
        })
    }

    fn text_block(text: &str) -> crate::conversation::message::MessageContent {
        crate::conversation::message::MessageContent::text(text)
    }

    #[test]
    fn extract_tool_chains_returns_empty_for_no_tool_blocks() {
        let content = vec![text_block("hello"), text_block("world")];
        assert!(extract_tool_chains(&content).is_empty());
    }

    #[test]
    fn extract_tool_chains_returns_single_chain_when_only_tools() {
        let content = vec![
            tool_request_block("a"),
            tool_request_block("b"),
            tool_request_block("c"),
        ];
        let chains = extract_tool_chains(&content);
        assert_eq!(
            chains,
            vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]]
        );
    }

    #[test]
    fn extract_tool_chains_breaks_on_text_block() {
        let content = vec![
            tool_request_block("a"),
            tool_request_block("b"),
            text_block("interlude"),
            tool_request_block("c"),
            tool_request_block("d"),
        ];
        let chains = extract_tool_chains(&content);
        assert_eq!(
            chains,
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string(), "d".to_string()],
            ]
        );
    }

    #[test]
    fn extract_tool_chains_includes_singletons() {
        let content = vec![
            tool_request_block("a"),
            text_block("split"),
            tool_request_block("b"),
            text_block("split"),
            tool_request_block("c"),
        ];
        let chains = extract_tool_chains(&content);
        assert_eq!(
            chains,
            vec![
                vec!["a".to_string()],
                vec!["b".to_string()],
                vec!["c".to_string()],
            ]
        );
    }

    #[test]
    fn extract_tool_chains_keeps_run_when_text_leads_or_trails() {
        let content = vec![
            text_block("intro"),
            tool_request_block("a"),
            tool_request_block("b"),
            text_block("outro"),
        ];
        let chains = extract_tool_chains(&content);
        assert_eq!(chains, vec![vec!["a".to_string(), "b".to_string()]]);
    }

    fn buf_entry(tool_id: &str, msg_id: &str) -> (String, String) {
        (tool_id.to_string(), msg_id.to_string())
    }

    #[test]
    fn extend_chain_membership_skips_singleton_and_leaves_buffer() {
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![buf_entry("a", "row_1")];

        extend_chain_membership(&buffer, &mut membership);

        assert_eq!(buffer.len(), 1, "buffer is left intact for caller");
        assert!(
            membership.is_empty(),
            "single-tool runs should not register a chain",
        );
    }

    #[test]
    fn extend_chain_membership_registers_each_id_against_shared_chain() {
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![
            buf_entry("a", "row_first"),
            buf_entry("b", "row_second"),
            buf_entry("c", "row_third"),
        ];

        extend_chain_membership(&buffer, &mut membership);

        assert_eq!(membership.len(), 3);
        let chain_a = membership.get("a").expect("a registered");
        let chain_b = membership.get("b").expect("b registered");
        let chain_c = membership.get("c").expect("c registered");
        assert!(
            Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c),
            "every id in the run must point at the same ToolChain Arc",
        );
        assert_eq!(
            chain_a.ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }

    #[test]
    fn extend_chain_membership_anchors_on_first_row_for_split_messages() {
        // Sequential tool use (Bedrock/Anthropic) emits each tool request as
        // its own assistant message, with the tool response interleaved in
        // between. The chain should still form, anchored on the *first*
        // tool's row id so `update_tool_request_meta` can find that
        // ToolRequest when persisting the summary.
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![
            buf_entry("toolu_bdrk_1", "row_for_tool_1"),
            buf_entry("toolu_bdrk_2", "row_for_tool_2"),
        ];

        extend_chain_membership(&buffer, &mut membership);

        let chain = membership
            .get("toolu_bdrk_1")
            .expect("first tool registered");
        assert_eq!(
            chain.ids,
            vec!["toolu_bdrk_1".to_string(), "toolu_bdrk_2".to_string()],
        );
        let chain_via_second = membership
            .get("toolu_bdrk_2")
            .expect("second tool registered");
        assert!(Arc::ptr_eq(chain, chain_via_second));
    }

    #[test]
    fn extend_chain_membership_grows_chain_as_more_requests_arrive() {
        // The streaming loop re-registers eagerly each time a new request
        // arrives, so a chain that started at length 2 must grow to include
        // a third tool whose response is yet to come. Both the original
        // members and the new member must point at the new (extended) chain.
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let mut buffer = vec![buf_entry("a", "row_1"), buf_entry("b", "row_2")];
        extend_chain_membership(&buffer, &mut membership);

        buffer.push(buf_entry("c", "row_3"));
        extend_chain_membership(&buffer, &mut membership);

        let chain_a = membership.get("a").expect("a present");
        let chain_b = membership.get("b").expect("b present");
        let chain_c = membership.get("c").expect("c present");
        assert!(Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c));
        assert_eq!(
            chain_a.ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }

    #[test]
    fn with_tool_chain_summary_meta_creates_fresh_when_none() {
        let meta = with_tool_chain_summary_meta(None, "applied dark mode", 4)
            .expect("meta should be created");
        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "toolChainSummary": { "summary": "applied dark mode", "count": 4 },
            })),
        );
    }

    #[test]
    fn with_tool_chain_summary_meta_preserves_existing_tool_call_identity() {
        let existing = tool_call_identity_meta(&ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: None,
        });
        let meta = with_tool_chain_summary_meta(existing, "ran two commands", 2)
            .expect("meta should be created");
        let goose = meta.get("goose").expect("goose key");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            )
        );
        assert_eq!(
            goose.get("toolChainSummary"),
            Some(&serde_json::json!({ "summary": "ran two commands", "count": 2 }))
        );
    }

    #[test]
    fn replay_attaches_chain_summary_meta_for_first_tool_request_with_persisted_summary() {
        let tool_request = ToolRequest {
            id: "req_first".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: Some(serde_json::json!({
                crate::conversation::message::TOOL_META_CHAIN_SUMMARY_KEY: {
                    "summary": "applied dark mode polish",
                    "count": 3,
                },
            })),
        };

        let pending_tool_call = pending_tool_call_from_request(&tool_request);
        let mut meta = pending_tool_call.identity_meta;
        let chain_summary = tool_request
            .persisted_chain_summary()
            .expect("chain summary should be present");
        meta = with_tool_chain_summary_meta(meta, &chain_summary.summary, chain_summary.count);

        let goose = meta
            .as_ref()
            .and_then(|m| m.get("goose"))
            .expect("replay meta must include a goose namespace");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            ),
            "replay must preserve identity meta alongside the chain summary",
        );
        assert_eq!(
            goose.get("toolChainSummary"),
            Some(&serde_json::json!({ "summary": "applied dark mode polish", "count": 3 })),
            "replay must attach toolChainSummary so the chain header renders on first paint",
        );
    }

    #[test]
    fn replay_does_not_attach_chain_summary_for_tool_requests_without_persisted_summary() {
        let tool_request = ToolRequest {
            id: "req_second".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: None,
        };

        let chain_summary = tool_request.persisted_chain_summary();
        assert!(
            chain_summary.is_none(),
            "non-first tool requests must not carry chain summaries",
        );
    }

    #[test]
    fn test_summarize_tool_call_long_value_truncated() {
        let long_path = "a".repeat(80);
        let args = serde_json::json!({"path": long_path});
        let result = summarize_tool_call("developer__read_file", Some(&args));
        assert!(result.ends_with('…'));
        assert!(result.len() < 90);
    }

    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AllowOnce };
        "allow_once_maps_to_allow_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysAllow };
        "allow_always_maps_to_always_allow"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::DenyOnce };
        "reject_once_maps_to_deny_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysDeny };
        "reject_always_maps_to_always_deny"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("unknown".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "unknown_option_maps_to_cancel"
    )]
    #[test_case(
        RequestPermissionOutcome::Cancelled,
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "cancelled_maps_to_cancel"
    )]
    fn test_outcome_to_confirmation(
        input: RequestPermissionOutcome,
        expected: PermissionConfirmation,
    ) {
        assert_eq!(outcome_to_confirmation(&input), expected);
    }

    #[test_case(
        vec!["model-a".into(), "model-b".into()]
        => SessionModelState::new(
            ModelId::new("unused"),
            vec![ModelInfo::new(ModelId::new("unused"), "unused"),
                 ModelInfo::new(ModelId::new("model-a"), "model-a"),
                 ModelInfo::new(ModelId::new("model-b"), "model-b")],
        )
        ; "returns current and available models"
    )]
    #[test_case(
        vec![]
        => SessionModelState::new(
            ModelId::new("unused"),
            vec![ModelInfo::new(ModelId::new("unused"), "unused")],
        )
        ; "empty model list"
    )]
    fn test_build_model_state(models: Vec<String>) -> SessionModelState {
        let inventory = ProviderInventoryEntry {
            provider_id: "mock".to_string(),
            provider_name: "Mock".to_string(),
            description: "Mock".to_string(),
            default_model: "unused".to_string(),
            configured: true,
            provider_type: crate::providers::base::ProviderType::Builtin,
            category: crate::providers::catalog::ProviderSetupCategory::Model,
            config_keys: vec![],
            setup_steps: vec![],
            supports_refresh: true,
            refreshing: false,
            models: models
                .into_iter()
                .map(|id| crate::providers::inventory::InventoryModel {
                    name: id.clone(),
                    id,
                    family: None,
                    context_limit: None,
                    reasoning: None,
                    recommended: false,
                })
                .collect(),
            last_updated_at: None,
            last_refresh_attempt_at: None,
            last_refresh_error: None,
            model_selection_hint: None,
        };
        build_model_state("unused", &inventory)
    }

    fn json_object(pairs: Vec<(&str, serde_json::Value)>) -> rmcp::model::JsonObject {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test_case(None => None ; "none arguments")]
    #[test_case(Some(json_object(vec![])) => None ; "missing line key")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!(5))])) => Some(5) ; "line present")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!("not_a_number"))])) => None ; "line not a number")]
    fn test_get_requested_line(arguments: Option<rmcp::model::JsonObject>) -> Option<u32> {
        get_requested_line(arguments.as_ref())
    }

    #[test_case("read", true ; "read is developer file tool")]
    #[test_case("write", true ; "write is developer file tool")]
    #[test_case("edit", true ; "edit is developer file tool")]
    #[test_case("shell", false ; "shell is not developer file tool")]
    #[test_case("analyze", false ; "analyze is not developer file tool")]
    fn test_is_developer_file_tool(tool_name: &str, expected: bool) {
        assert_eq!(is_developer_file_tool(tool_name), expected);
    }

    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "line": 5}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(5))]
        ; "read returns requested line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), None)]
        ; "read without line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("write").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "content": "hi"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "write returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("edit").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "before": "a", "after": "b"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "edit returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("shell").with_arguments(serde_json::json!({"command": "ls"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => Vec::<(PathBuf, Option<u32>)>::new()
        ; "non file tool returns empty"
    )]
    fn test_extract_tool_locations(
        request: ToolRequest,
        response: ToolResponse,
    ) -> Vec<(PathBuf, Option<u32>)> {
        extract_tool_locations(&request, &response)
            .into_iter()
            .map(|loc| (loc.path, loc.line))
            .collect()
    }

    fn response_with_meta(meta: Option<serde_json::Value>) -> ToolResponse {
        let mut result = CallToolResult::success(vec![RmcpContent::text("")]);
        result.meta = meta.map(|v| serde_json::from_value(v).unwrap());
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(result),
            metadata: None,
        }
    }

    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt", "line": 5}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), Some(5))])
        ; "meta with path and line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt"}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), None)])
        ; "meta with path no line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({})))
        => None
        ; "meta without tool_locations key"
    )]
    #[test_case(
        response_with_meta(None)
        => None
        ; "no meta"
    )]
    fn test_extract_locations_from_meta(
        response: ToolResponse,
    ) -> Option<Vec<(PathBuf, Option<u32>)>> {
        extract_locations_from_meta(&response)
            .map(|locs| locs.into_iter().map(|loc| (loc.path, loc.line)).collect())
    }

    #[test]
    fn test_extract_tool_call_update_meta_ignores_untrusted_goose_meta() {
        let response = response_with_meta(Some(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://spoofed/app",
                },
            },
        })));

        assert_eq!(extract_tool_call_update_meta(&response), None);
    }

    #[test]
    fn test_extract_tool_call_update_meta_uses_trusted_meta_only() {
        let response = response_with_meta(Some(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://spoofed/app",
                },
            },
            TRUSTED_TOOL_UPDATE_META_KEY: {
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            },
        })));

        let extracted = extract_tool_call_update_meta(&response).expect("expected trusted meta");
        assert_eq!(
            extracted.get("goose"),
            Some(&serde_json::json!({
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_preserves_existing_goose_meta() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_1");
        let existing = serde_json::from_value(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            },
        }))
        .unwrap();

        let merged = merge_replay_message_meta(Some(existing), &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_1",
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_creates_fresh_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_2");

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_2",
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_omits_message_id_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]);

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
            })),
        );
    }

    #[test]
    fn test_extract_tool_raw_output_preserves_structured_content() {
        let mut result = CallToolResult::success(vec![RmcpContent::text("fallback")]);
        result.structured_content = Some(serde_json::json!({
            "restaurants": [
                {
                    "name": "Coffee Shop",
                    "unitToken": "unit-1",
                },
            ],
        }));

        assert_eq!(
            extract_tool_raw_output(&Ok(result)),
            Some(serde_json::json!({
                "restaurants": [
                    {
                        "name": "Coffee Shop",
                        "unitToken": "unit-1",
                    },
                ],
            })),
        );
    }

    fn make_session_with_usage(
        total_tokens: Option<i32>,
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        accumulated_total_tokens: Option<i32>,
        accumulated_input_tokens: Option<i32>,
        accumulated_output_tokens: Option<i32>,
    ) -> Session {
        Session {
            id: "session-1".to_string(),
            working_dir: PathBuf::from("/tmp"),
            name: "ACP Session".to_string(),
            user_set_name: false,
            session_type: SessionType::Acp,
            created_at: Default::default(),
            updated_at: Default::default(),
            extension_data: crate::session::ExtensionData::default(),
            total_tokens,
            input_tokens,
            output_tokens,
            accumulated_total_tokens,
            accumulated_input_tokens,
            accumulated_output_tokens,
            accumulated_cost: None,
            schedule_id: None,
            recipe: None,
            user_recipe_values: None,
            conversation: None,
            message_count: 0,
            provider_name: None,
            model_config: None,
            goose_mode: GooseMode::default(),
            archived_at: None,
            project_id: None,
        }
    }

    #[test]
    fn test_build_prompt_usage_uses_current_turn_tokens() {
        let session = make_session_with_usage(
            Some(120),
            Some(80),
            Some(40),
            Some(360),
            Some(210),
            Some(150),
        );
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_falls_back_to_current_tokens() {
        let session = make_session_with_usage(Some(120), Some(80), Some(40), None, None, None);
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_requires_total_tokens() {
        let session = make_session_with_usage(None, Some(80), Some(40), None, None, None);
        assert!(build_prompt_usage(&session).is_none());
    }

    #[test]
    fn test_build_usage_update_clamps_negative_used_to_zero() {
        let session = make_session_with_usage(Some(-7), Some(0), Some(0), None, None, None);
        let usage = build_usage_update(&session, 258_000);
        assert_eq!(usage.used, 0);
        assert_eq!(usage.size, 258_000);
    }

    #[test_case(
        GooseMode::Auto
        => Ok(SessionModeState::new(
            SessionModeId::new("auto"),
            vec![
                SessionMode::new(SessionModeId::new("auto"), "auto")
                    .description("Automatically approve tool calls"),
                SessionMode::new(SessionModeId::new("approve"), "approve")
                    .description("Ask before every tool call"),
                SessionMode::new(SessionModeId::new("smart_approve"), "smart_approve")
                    .description("Ask only for sensitive tool calls"),
                SessionMode::new(SessionModeId::new("chat"), "chat")
                    .description("Chat only, no tool calls"),
            ],
        ))
        ; "auto mode"
    )]
    #[test_case(
        GooseMode::Approve
        => Ok(SessionModeState::new(
            SessionModeId::new("approve"),
            vec![
                SessionMode::new(SessionModeId::new("auto"), "auto")
                    .description("Automatically approve tool calls"),
                SessionMode::new(SessionModeId::new("approve"), "approve")
                    .description("Ask before every tool call"),
                SessionMode::new(SessionModeId::new("smart_approve"), "smart_approve")
                    .description("Ask only for sensitive tool calls"),
                SessionMode::new(SessionModeId::new("chat"), "chat")
                    .description("Chat only, no tool calls"),
            ],
        ))
        ; "approve mode"
    )]
    fn test_build_mode_state(
        current_mode: GooseMode,
    ) -> Result<SessionModeState, agent_client_protocol::Error> {
        build_mode_state(current_mode)
    }

    #[test_case(
        build_mode_state(GooseMode::Auto).unwrap(),
        "openai",
        vec![
            SessionConfigSelectOption::new("anthropic", "anthropic"),
            SessionConfigSelectOption::new("openai", "openai"),
        ],
        SessionModelState::new(
            ModelId::new("gpt-4"),
            vec![ModelInfo::new(ModelId::new("gpt-4"), "gpt-4"), ModelInfo::new(ModelId::new("gpt-3.5"), "gpt-3.5")],
        )
        => vec![
            SessionConfigOption::select(
                "provider", "Provider", "openai",
                vec![
                    SessionConfigSelectOption::new("anthropic", "anthropic"),
                    SessionConfigSelectOption::new("openai", "openai"),
                ],
            ),
            SessionConfigOption::select(
                "mode", "Mode", "auto",
                vec![
                    SessionConfigSelectOption::new("auto", "auto").description("Automatically approve tool calls"),
                    SessionConfigSelectOption::new("approve", "approve").description("Ask before every tool call"),
                    SessionConfigSelectOption::new("smart_approve", "smart_approve").description("Ask only for sensitive tool calls"),
                    SessionConfigSelectOption::new("chat", "chat").description("Chat only, no tool calls"),
                ],
            ).category(SessionConfigOptionCategory::Mode),
            SessionConfigOption::select(
                "model", "Model", "gpt-4",
                vec![
                    SessionConfigSelectOption::new("gpt-4", "gpt-4"),
                    SessionConfigSelectOption::new("gpt-3.5", "gpt-3.5"),
                ],
            ).category(SessionConfigOptionCategory::Model),
        ]
        ; "auto mode with multiple models"
    )]
    #[test_case(
        build_mode_state(GooseMode::Approve).unwrap(),
        "openai",
        vec![SessionConfigSelectOption::new("openai", "openai")],
        SessionModelState::new(ModelId::new("only-model"), vec![ModelInfo::new(ModelId::new("only-model"), "only-model")])
        => vec![
            SessionConfigOption::select(
                "provider", "Provider", "openai",
                vec![SessionConfigSelectOption::new("openai", "openai")],
            ),
            SessionConfigOption::select(
                "mode", "Mode", "approve",
                vec![
                    SessionConfigSelectOption::new("auto", "auto").description("Automatically approve tool calls"),
                    SessionConfigSelectOption::new("approve", "approve").description("Ask before every tool call"),
                    SessionConfigSelectOption::new("smart_approve", "smart_approve").description("Ask only for sensitive tool calls"),
                    SessionConfigSelectOption::new("chat", "chat").description("Chat only, no tool calls"),
                ],
            ).category(SessionConfigOptionCategory::Mode),
            SessionConfigOption::select(
                "model", "Model", "only-model",
                vec![SessionConfigSelectOption::new("only-model", "only-model")],
            ).category(SessionConfigOptionCategory::Model),
        ]
        ; "approve mode with single model"
    )]
    fn test_build_config_options(
        mode_state: SessionModeState,
        provider_name: &'static str,
        provider_options: Vec<SessionConfigSelectOption>,
        model_state: SessionModelState,
    ) -> Vec<SessionConfigOption> {
        build_config_options(&mode_state, &model_state, provider_name, provider_options)
    }
}
