use anyhow::{anyhow, Result};
use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, Initialized, Notification},
    request::{GotoDefinition, HoverRequest, Initialize, References, Request, Shutdown},
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, InitializeParams, InitializeResult,
    InitializedParams, Location, Position, ReferenceParams, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
    VersionedTextDocumentIdentifier, WorkspaceFolder,
};
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use super::protocol::JsonRpcProtocol;
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
    process: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    protocol: JsonRpcProtocol,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    diagnostics: Arc<Mutex<HashMap<PathBuf, Vec<LspDiagnostic>>>>,
    root_uri: Url,
    open_files: Arc<Mutex<HashMap<PathBuf, u32>>>,
    initialized: Arc<Mutex<bool>>,
}

impl LspClient {
    pub async fn new(config: LspConfig) -> Result<Self> {
        info!("Starting LSP client: {}", config.name);

        let mut cmd = Command::new(&config.cmd);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.envs);

        let mut process = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn LSP process {}: {}", config.cmd, e))?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;

        let stdin = Arc::new(Mutex::new(stdin));
        let protocol = JsonRpcProtocol::new();
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(Mutex::new(HashMap::new()));
        let open_files = Arc::new(Mutex::new(HashMap::new()));
        let initialized = Arc::new(Mutex::new(false));

        let root_uri = Url::from_directory_path(&config.workspace_root)
            .map_err(|_| anyhow!("Invalid workspace root path"))?;

        let mut client = Self {
            config,
            process,
            stdin,
            protocol,
            pending_requests: pending_requests.clone(),
            diagnostics: diagnostics.clone(),
            root_uri: root_uri.clone(),
            open_files,
            initialized,
        };

        client.start_reader(stdout, pending_requests, diagnostics)?;
        client.initialize().await?;

        Ok(client)
    }

    fn start_reader(
        &self,
        stdout: ChildStdout,
        pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
        diagnostics: Arc<Mutex<HashMap<PathBuf, Vec<LspDiagnostic>>>>,
    ) -> Result<()> {
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);

            loop {
                match JsonRpcProtocol::read_message(&mut reader) {
                    Ok(message) => {
                        if message.get("id").is_some() {
                            if let Ok(response) = JsonRpcProtocol::parse_response(message) {
                                if let Some(sender) =
                                    pending_requests.lock().unwrap().remove(&response.id)
                                {
                                    if let Some(result) = response.result {
                                        let _ = sender.send(result);
                                    } else if let Some(error) = response.error {
                                        warn!("LSP error: {:?}", error);
                                    }
                                }
                            }
                        } else if message.get("method").is_some() {
                            if let Ok(notification) = JsonRpcProtocol::parse_notification(message) {
                                if notification.method == "textDocument/publishDiagnostics" {
                                    if let Some(params) = notification.params {
                                        if let Ok(uri) = serde_json::from_value::<String>(
                                            params.get("uri").cloned().unwrap_or_default(),
                                        ) {
                                            if let Ok(url) = Url::parse(&uri) {
                                                if let Ok(path) = url.to_file_path() {
                                                    if let Ok(diags) = serde_json::from_value::<
                                                        Vec<lsp_types::Diagnostic>,
                                                    >(
                                                        params
                                                            .get("diagnostics")
                                                            .cloned()
                                                            .unwrap_or_default(),
                                                    ) {
                                                        let lsp_diags: Vec<LspDiagnostic> = diags
                                                            .into_iter()
                                                            .map(Into::into)
                                                            .collect();
                                                        diagnostics
                                                            .lock()
                                                            .unwrap()
                                                            .insert(path, lsp_diags);
                                                        debug!("Updated diagnostics for {:?}", url);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading LSP message: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
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
            capabilities: lsp_types::ClientCapabilities {
                text_document: Some(lsp_types::TextDocumentClientCapabilities {
                    synchronization: Some(lsp_types::TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        did_save: Some(false),
                    }),
                    hover: Some(lsp_types::HoverClientCapabilities {
                        dynamic_registration: Some(false),
                        content_format: Some(vec![lsp_types::MarkupKind::PlainText]),
                    }),
                    definition: Some(lsp_types::GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(lsp_types::ReferenceClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let result: InitializeResult = self
            .send_request(Initialize::METHOD, Some(serde_json::to_value(params)?))
            .await?;

        let notification_params = InitializedParams {};
        self.send_notification(
            Initialized::METHOD,
            Some(serde_json::to_value(notification_params)?),
        )
        .await?;

        *self.initialized.lock().unwrap() = true;
        info!("LSP client initialized: {}", self.config.name);

        Ok(result)
    }

    async fn send_request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<T> {
        let (tx, rx) = oneshot::channel();

        let id = {
            let mut stdin = self.stdin.lock().unwrap();
            let id = self.protocol.send_request(&mut stdin, method, params)?;
            self.pending_requests.lock().unwrap().insert(id, tx);
            id
        };

        let result = rx.await.map_err(|_| anyhow!("Request {} cancelled", id))?;

        serde_json::from_value(result).map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<()> {
        let mut stdin = self.stdin.lock().unwrap();
        self.protocol.send_notification(&mut stdin, method, params)
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
            .unwrap()
            .insert(file_path.to_path_buf(), 0);
        self.send_notification(
            DidOpenTextDocument::METHOD,
            Some(serde_json::to_value(params)?),
        )
        .await
    }

    pub async fn text_document_did_change(&self, file_path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let uri = Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?;

        let version = {
            let mut open_files = self.open_files.lock().unwrap();
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

        self.send_notification(
            DidChangeTextDocument::METHOD,
            Some(serde_json::to_value(params)?),
        )
        .await
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

        self.send_request(HoverRequest::METHOD, Some(serde_json::to_value(params)?))
            .await
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

        self.send_request(GotoDefinition::METHOD, Some(serde_json::to_value(params)?))
            .await
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

        self.send_request(References::METHOD, Some(serde_json::to_value(params)?))
            .await
    }

    pub fn get_diagnostics(&self, file_path: &Path) -> Vec<LspDiagnostic> {
        self.diagnostics
            .lock()
            .unwrap()
            .get(file_path)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down LSP client: {}", self.config.name);

        self.send_request::<Value>(Shutdown::METHOD, None)
            .await
            .ok();

        self.send_notification("exit", None).await.ok();

        self.process.kill().ok();

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
