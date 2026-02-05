//! ACP Server - Protocol handler for client connections
//!
//! The Server handles ACP protocol communication and maintains
//! an in-memory cache of active sessions. It delegates actual
//! work to Session instances.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{info, instrument, warn};

use agent_client_protocol_schema::{
    AgentCapabilities, Implementation, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, SessionId,
};
use sacp::{Client, ConnectionTo, Responder};

use crate::db::Database;
use crate::extension::ExtensionCatalog;
use crate::notifier::AcpNotifier;
use crate::provider::ProviderConfig;
use crate::session::Session;
use crate::{Error, Result};

/// ACP protocol server
///
/// Maintains database connection, extension catalog, and in-memory session cache.
/// Sessions are kept in memory to preserve extension connections.
pub struct Server {
    /// Database for session persistence
    db: Arc<Database>,
    /// Global extension catalog (shared across all sessions)
    catalog: Arc<RwLock<ExtensionCatalog>>,
    /// In-memory session cache (sessions kept alive for extension connections)
    sessions: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<Session>>>>>,
}

impl Server {
    /// Create a new server with the default database path and no config file
    pub fn new() -> Result<Self> {
        Self::with_paths("goose2.db", None)
    }

    /// Create a new server with custom database and config paths
    pub fn with_paths(db_path: &str, config_path: Option<&Path>) -> Result<Self> {
        let db = Database::open(db_path)?;

        let catalog = match config_path {
            Some(path) => ExtensionCatalog::load(path)?,
            None => ExtensionCatalog::default(),
        };

        Ok(Self {
            db: Arc::new(db),
            catalog: Arc::new(RwLock::new(catalog)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Reload the extension catalog from a config file
    ///
    /// This can be called at runtime to pick up new extensions
    /// without restarting the server.
    pub fn reload_extensions(&self, config_path: &Path) -> Result<()> {
        let mut catalog = self.catalog.write();
        catalog.reload(config_path)?;
        info!("Extension catalog reloaded");
        Ok(())
    }

    /// Run the server, listening on stdin/stdout via ACP
    #[instrument(skip(self))]
    pub async fn run(self) -> Result<()> {
        info!("Starting ACP server");

        let db = self.db.clone();
        let catalog = self.catalog.clone();
        let sessions = self.sessions.clone();

        // Set up ACP transport
        let stdin = tokio::io::stdin().compat();
        let stdout = tokio::io::stdout().compat_write();
        let transport = sacp::ByteStreams::new(stdout, stdin);

        sacp::Agent
            .builder()
            .name("goose2")
            .on_receive_request(
                |req: InitializeRequest, responder: Responder<InitializeResponse>, _cx| async move {
                    info!(protocol_version = %req.protocol_version, "Initialize request");
                    let response = InitializeResponse::new(req.protocol_version)
                        .agent_info(Implementation::new("goose2", env!("CARGO_PKG_VERSION")))
                        .agent_capabilities(AgentCapabilities::default());
                    responder.respond(response)
                },
                sacp::on_receive_request!(),
            )
            .on_receive_request(
                {
                    let db = db.clone();
                    let catalog = catalog.clone();
                    let sessions = sessions.clone();
                    move |req: NewSessionRequest, responder: Responder<NewSessionResponse>, _cx| {
                        let db = db.clone();
                        let catalog = catalog.clone();
                        let sessions = sessions.clone();
                        async move {
                            let result = handle_new_session(&db, &catalog, &sessions, req).await;
                            match result {
                                Ok(response) => responder.respond(response),
                                Err(e) => Err(sacp::Error::internal_error().data(e.to_string())),
                            }
                        }
                    }
                },
                sacp::on_receive_request!(),
            )
            .on_receive_request(
                {
                    let db = db.clone();
                    let catalog = catalog.clone();
                    let sessions = sessions.clone();
                    move |req: PromptRequest,
                          responder: Responder<PromptResponse>,
                          cx: ConnectionTo<Client>| {
                        let db = db.clone();
                        let catalog = catalog.clone();
                        let sessions = sessions.clone();
                        async move {
                            let result = handle_prompt(&db, &catalog, &sessions, req, cx).await;
                            match result {
                                Ok(response) => responder.respond(response),
                                Err(e) => Err(sacp::Error::internal_error().data(e.to_string())),
                            }
                        }
                    }
                },
                sacp::on_receive_request!(),
            )
            .connect_to(transport)
            .await
            .map_err(|e| Error::Internal(format!("ACP connection error: {}", e)))?;

        Ok(())
    }
}

/// Handle session/new request
async fn handle_new_session(
    db: &Arc<Database>,
    catalog: &Arc<RwLock<ExtensionCatalog>>,
    sessions: &Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<Session>>>>>,
    req: NewSessionRequest,
) -> Result<NewSessionResponse> {
    let session_id = SessionId::new(uuid::Uuid::new_v4().to_string());
    let session_id_str = session_id.to_string();

    info!(session_id = %session_id, cwd = ?req.cwd, "Creating new session");

    // Create session in database
    db.create_session(&session_id_str)?;

    // Create in-memory session with provider config from environment
    let provider_config = ProviderConfig::from_env();
    let session = Session::new(
        session_id.clone(),
        db.clone(),
        catalog.clone(),
        provider_config,
    )?;

    // Cache the session
    {
        let mut cache = sessions.write();
        cache.insert(session_id_str, Arc::new(tokio::sync::Mutex::new(session)));
    }

    Ok(NewSessionResponse::new(session_id))
}

/// Handle session/prompt request
async fn handle_prompt(
    db: &Arc<Database>,
    catalog: &Arc<RwLock<ExtensionCatalog>>,
    sessions: &Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<Session>>>>>,
    req: PromptRequest,
    cx: ConnectionTo<Client>,
) -> Result<PromptResponse> {
    let session_id = req.session_id.clone();
    let session_id_str = session_id.to_string();

    info!(session_id = %session_id, "Handling prompt");

    // Get or load session
    let session = get_or_load_session(db, catalog, sessions, &session_id_str).await?;

    // Create notifier for this connection
    let notifier = AcpNotifier::new(cx);

    // Run the prompt
    let mut session_guard = session.lock().await;
    let stop_reason = session_guard.prompt(req, &notifier).await?;

    Ok(PromptResponse::new(stop_reason))
}

/// Get session from cache, or load from DB if not cached
///
/// Uses a check-then-load pattern with proper locking to prevent race conditions.
async fn get_or_load_session(
    db: &Arc<Database>,
    catalog: &Arc<RwLock<ExtensionCatalog>>,
    sessions: &Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<Session>>>>>,
    session_id: &str,
) -> Result<Arc<tokio::sync::Mutex<Session>>> {
    // First check cache with read lock
    {
        let cache = sessions.read();
        if let Some(session) = cache.get(session_id) {
            return Ok(session.clone());
        }
    }

    // Not in cache - load from DB (outside of any lock)
    warn!(session_id = %session_id, "Session not in cache, loading from DB");

    let session_data = db
        .load_session(session_id)?
        .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;

    // Reconstruct session (extensions will be re-enabled)
    let provider_config = ProviderConfig::from_env();
    let session_id_typed = SessionId::new(session_id.to_string());
    let session = Session::from_db(
        session_id_typed,
        db.clone(),
        catalog.clone(),
        provider_config,
        &session_data,
    )
    .await?;

    let session = Arc::new(tokio::sync::Mutex::new(session));

    // Now acquire write lock to insert (another task might have loaded it too, that's ok)
    {
        let mut cache = sessions.write();
        // Use entry API to avoid overwriting if another task beat us
        cache
            .entry(session_id.to_string())
            .or_insert_with(|| session.clone());
    }

    Ok(session)
}
