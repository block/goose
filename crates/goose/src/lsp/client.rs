use anyhow::{anyhow, Result};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidOpenTextDocument, Initialized, LogMessage, Notification,
        PublishDiagnostics,
    },
    request::{GotoDefinition, HoverRequest, Initialize, References, Request, Shutdown},
    ClientCapabilities, ClientInfo, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    GotoCapability, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverClientCapabilities,
    HoverParams, InitializeParams, InitializeResult, InitializedParams, Location, MarkupKind,
    MessageType, Position, PublishDiagnosticsParams, ReferenceClientCapabilities, ReferenceParams,
    ServerCapabilities, TextDocumentClientCapabilities, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
    TextDocumentSyncClientCapabilities, Url, VersionedTextDocumentIdentifier, WorkspaceFolder,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, error, info, warn};

use super::types::LspDiagnostic;

#[derive(Debug, Clone)]
pub struct LspConfig {
    pub name: String,
    pub language_id: String,
    pub cmd: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub root_patterns: Vec<String>,
    pub workspace_root: PathBuf,
    pub instructions: Option<String>,
}

pub struct LspClient {
    config: LspConfig,
    _child: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    diagnostics: Arc<Mutex<HashMap<PathBuf, Vec<LspDiagnostic>>>>,
    root_uri: Url,
    open_files: Arc<Mutex<HashMap<PathBuf, u32>>>,
    capabilities: Arc<Mutex<ServerCapabilities>>,
}

impl LspClient {
    pub async fn new(config: LspConfig) -> Result<Self> {
        info!("Starting LSP client: {}", config.name);

        let mut cmd = Command::new(&config.cmd);
        cmd.args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .envs(&config.envs)
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn LSP process {}: {}", config.cmd, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;

        let stdin = Arc::new(Mutex::new(stdin));
        let next_id = AtomicU64::new(1);
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(Mutex::new(HashMap::new()));
        let open_files = Arc::new(Mutex::new(HashMap::new()));
        let capabilities = Arc::new(Mutex::new(ServerCapabilities::default()));

        let root_uri = Url::from_directory_path(&config.workspace_root)
            .map_err(|_| anyhow!("Invalid workspace root path"))?;

        let mut client = Self {
            config,
            _child: Arc::new(Mutex::new(Some(child))),
            stdin,
            next_id,
            pending_requests: pending_requests.clone(),
            diagnostics: diagnostics.clone(),
            root_uri: root_uri.clone(),
            open_files,
            capabilities: capabilities.clone(),
        };

        client.start_message_router(stdout, pending_requests, diagnostics);
        let init_result = client.initialize().await?;
        *client.capabilities.lock().await = init_result.capabilities;

        Ok(client)
    }

    fn start_message_router(
        &self,
        stdout: ChildStdout,
        pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
        diagnostics: Arc<Mutex<HashMap<PathBuf, Vec<LspDiagnostic>>>>,
    ) {
        tokio::spawn(async move {
            let reader = Arc::new(Mutex::new(BufReader::new(stdout)));

            loop {
                match Self::receive_message(&reader).await {
                    Ok(msg) => {
                        if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                            if let Some(tx) = pending_requests.lock().await.remove(&id) {
                                let _ = tx.send(msg);
                            }
                        } else if let Some(method) = msg.get("method").and_then(|v| v.as_str()) {
                            match method {
                                PublishDiagnostics::METHOD => {
                                    if let Ok(params) =
                                        serde_json::from_value::<PublishDiagnosticsParams>(
                                            msg.get("params").cloned().unwrap_or_default(),
                                        )
                                    {
                                        if let Ok(path) = params.uri.to_file_path() {
                                            let lsp_diags: Vec<LspDiagnostic> = params
                                                .diagnostics
                                                .into_iter()
                                                .map(Into::into)
                                                .collect();
                                            diagnostics.lock().await.insert(path, lsp_diags);
                                            debug!("Updated diagnostics for {}", params.uri);
                                        }
                                    }
                                }
                                LogMessage::METHOD => {
                                    if let Ok(params) =
                                        serde_json::from_value::<lsp_types::LogMessageParams>(
                                            msg.get("params").cloned().unwrap_or_default(),
                                        )
                                    {
                                        match params.typ {
                                            MessageType::ERROR => error!("LSP: {}", params.message),
                                            MessageType::WARNING => {
                                                warn!("LSP: {}", params.message)
                                            }
                                            MessageType::INFO => info!("LSP: {}", params.message),
                                            MessageType::LOG => debug!("LSP: {}", params.message),
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {
                                    debug!("Unhandled LSP notification: {}", method);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Message router error (continuing): {}", e);
                    }
                }
            }
        });
    }

    async fn receive_message(reader: &Arc<Mutex<BufReader<ChildStdout>>>) -> Result<Value> {
        let mut reader = reader.lock().await;
        let mut headers = HashMap::new();

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            if line == "\r\n" || line == "\n" {
                break;
            }

            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        let content_length = headers
            .get("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or_else(|| anyhow!("Missing Content-Length"))?;

        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).await?;

        serde_json::from_slice(&body).map_err(|e| anyhow!("Failed to parse JSON: {}", e))
    }

    async fn initialize(&mut self) -> Result<InitializeResult> {
        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(self.root_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: self.root_uri.clone(),
                name: self.config.name.clone(),
            }]),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        did_save: Some(false),
                    }),
                    hover: Some(HoverClientCapabilities {
                        dynamic_registration: Some(false),
                        content_format: Some(vec![MarkupKind::PlainText]),
                    }),
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(ReferenceClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            client_info: Some(ClientInfo {
                name: "goose".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            locale: None,
            initialization_options: None,
            trace: None,
            ..Default::default()
        };

        let result: InitializeResult = self.request::<Initialize>(params).await?;

        self.notify::<Initialized>(InitializedParams {}).await?;

        info!("LSP client initialized: {}", self.config.name);

        Ok(result)
    }

    async fn request<R: Request>(&self, params: R::Params) -> Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();
        self.pending_requests.lock().await.insert(id, tx);

        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": R::METHOD,
            "params": serde_json::to_value(params)?
        });

        self.send_message(message).await?;

        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| anyhow!("Request timeout"))?
            .map_err(|_| anyhow!("Request cancelled"))?;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP error: {}", error));
        }

        serde_json::from_value(response["result"].clone())
            .map_err(|e| anyhow!("Failed to parse response: {}", e))
    }

    async fn notify<N: Notification>(&self, params: N::Params) -> Result<()>
    where
        N::Params: serde::Serialize,
    {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": N::METHOD,
            "params": serde_json::to_value(params)?
        });

        self.send_message(message).await
    }

    async fn send_message(&self, msg: Value) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", text.len());

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(text.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    pub async fn text_document_did_open(&self, file_path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: self.config.language_id.clone(),
                version: 0,
                text: content,
            },
        };

        self.open_files
            .lock()
            .await
            .insert(file_path.to_path_buf(), 0);
        self.notify::<DidOpenTextDocument>(params).await
    }

    pub async fn text_document_did_change(&self, file_path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let version = {
            let mut open_files = self.open_files.lock().await;
            let version = open_files.entry(file_path.to_path_buf()).or_insert(0);
            *version += 1;
            *version
        };

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri,
                version: version as i32,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: content,
            }],
        };

        self.notify::<DidChangeTextDocument>(params).await
    }

    pub async fn hover(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<Hover>> {
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
        };

        self.request::<HoverRequest>(params).await
    }

    pub async fn goto_definition(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        self.request::<GotoDefinition>(params).await
    }

    pub async fn find_references(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<Vec<Location>>> {
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        self.request::<References>(params).await
    }

    pub async fn get_diagnostics(&self, file_path: &Path) -> Vec<LspDiagnostic> {
        self.diagnostics
            .lock()
            .await
            .get(file_path)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_capabilities(&self) -> ServerCapabilities {
        self.capabilities.lock().await.clone()
    }

    pub async fn is_file_open(&self, file_path: &Path) -> bool {
        self.open_files.lock().await.contains_key(file_path)
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down LSP client: {}", self.config.name);

        self.request::<Shutdown>(()).await.ok();

        self.notify::<lsp_types::notification::Exit>(()).await.ok();

        if let Some(mut child) = self._child.lock().await.take() {
            child.kill().await.ok();
        }

        Ok(())
    }
}
