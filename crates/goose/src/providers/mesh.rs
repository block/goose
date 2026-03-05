/// Mesh LLM provider — manages a local mesh-llm process and delegates inference to it.
///
/// On first use, if mesh-llm isn't already running on the configured port, this provider:
/// 1. Downloads the mesh-llm release bundle (macOS aarch64) if not found on PATH or in ~/.mesh-llm/
/// 2. Starts `mesh-llm --auto` (or with a custom model/invite) as a detached background process
/// 3. Waits for the API to become ready
/// 4. Delegates all inference to it via the OpenAI-compatible API
use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ModelInfo, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::formats::openai::create_request;
use super::openai_compatible::{
    handle_response_openai_compat, handle_status_openai_compat, stream_openai_compat,
};
use super::retry::ProviderRetry;
use super::utils::ImageFormat;
use crate::config::ExtensionConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::utils::RequestLog;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use rmcp::model::Tool;
use std::time::Duration;
use tokio::sync::OnceCell;

const MESH_PROVIDER_NAME: &str = "mesh";
const MESH_DEFAULT_MODEL: &str = "GLM-4.7-Flash-Q4_K_M";
const MESH_DEFAULT_PORT: u16 = 9337;
const MESH_DOWNLOAD_URL: &str = "https://github.com/michaelneale/decentralized-inference/releases/latest/download/mesh-llm-aarch64-apple-darwin.tar.gz";
const MESH_INSTALL_DIR: &str = ".mesh-llm";

pub const MESH_KNOWN_MODELS: &[(&str, usize)] = &[
    ("GLM-4.7-Flash-Q4_K_M", 32_768),
    ("GLM-4-32B-0414-Q4_K_M", 32_768),
    ("Qwen2.5-32B-Instruct-Q4_K_M", 32_768),
    ("Qwen2.5-14B-Instruct-Q4_K_M", 32_768),
    ("Qwen2.5-Coder-14B-Instruct-Q4_K_M", 32_768),
    ("Qwen2.5-Coder-7B-Instruct-Q4_K_M", 32_768),
    ("Qwen2.5-3B-Instruct-Q4_K_M", 32_768),
    ("Qwen3-32B-Q4_K_M", 32_768),
    ("Qwen3-8B-Q4_K_M", 32_768),
    ("Mistral-Small-3.1-24B-Instruct-2503-Q4_K_M", 32_768),
    ("DeepSeek-R1-Distill-Qwen-32B-Q4_K_M", 32_768),
    ("Hermes-2-Pro-Mistral-7B-Q4_K_M", 32_768),
];

static MESH_ENSURED: OnceCell<()> = OnceCell::const_new();

#[derive(Debug, serde::Serialize)]
pub struct MeshProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    port: u16,
}

/// Find the mesh-llm binary: check PATH first, then ~/.mesh-llm/
fn find_mesh_binary() -> Option<std::path::PathBuf> {
    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg("mesh-llm")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }

    // Check install dir
    let install_dir = dirs::home_dir()?.join(MESH_INSTALL_DIR);
    let binary = install_dir.join("mesh-llm");
    if binary.exists() {
        return Some(binary);
    }

    None
}

/// Download and extract the mesh-llm bundle to ~/.mesh-llm/
async fn download_mesh_binary() -> Result<std::path::PathBuf> {
    let install_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(MESH_INSTALL_DIR);
    std::fs::create_dir_all(&install_dir)?;

    tracing::info!("Downloading mesh-llm from {}", MESH_DOWNLOAD_URL);

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "curl -fsSL '{}' | tar xz --strip-components=1 -C '{}'",
            MESH_DOWNLOAD_URL,
            install_dir.display()
        ))
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to download mesh-llm: {}", stderr);
    }

    let binary = install_dir.join("mesh-llm");
    if !binary.exists() {
        anyhow::bail!(
            "mesh-llm binary not found after download at {}",
            binary.display()
        );
    }

    // Sign the binaries on macOS to avoid Gatekeeper issues
    for name in &["mesh-llm", "rpc-server", "llama-server"] {
        let bin = install_dir.join(name);
        if bin.exists() {
            let _ = tokio::process::Command::new("codesign")
                .args(["-s", "-", &bin.to_string_lossy()])
                .output()
                .await;
            let _ = tokio::process::Command::new("xattr")
                .args(["-cr", &bin.to_string_lossy()])
                .output()
                .await;
        }
    }

    tracing::info!("mesh-llm installed to {}", install_dir.display());
    Ok(binary)
}

/// Check if mesh-llm is already listening on the given port
async fn is_mesh_running(port: u16) -> bool {
    let url = format!("http://localhost:{}/v1/models", port);
    match reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Start mesh-llm as a fully detached daemon (survives parent exit)
async fn start_mesh(binary: &std::path::Path, port: u16) -> Result<()> {
    let config = crate::config::Config::global();

    let model: String = config
        .get_param("MESH_MODEL")
        .unwrap_or_else(|_| MESH_DEFAULT_MODEL.to_string());

    let invite: Option<String> = config.get_param("MESH_INVITE").ok();

    let mut mesh_args = vec![
        binary.to_string_lossy().to_string(),
        "--auto".to_string(),
        "--model".to_string(),
        model.clone(),
        "--port".to_string(),
        port.to_string(),
    ];

    if let Some(ref token) = invite {
        mesh_args.push("--join".to_string());
        mesh_args.push(token.clone());
    }

    let log_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(MESH_INSTALL_DIR)
        .join("mesh-llm.log");

    // Ensure log dir exists
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    tracing::info!(
        "Starting mesh-llm: {}",
        mesh_args.join(" ")
    );

    // Write a launcher script and execute it. The script backgrounds mesh-llm
    // and exits, so the process is fully orphaned (reparented to launchd/PID 1).
    let launcher_path = log_path.with_file_name("mesh-launcher.sh");
    let quoted_args: Vec<String> = mesh_args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
        .collect();
    let script = format!(
        "#!/bin/sh\n{} >> '{}' 2>&1 &\n",
        quoted_args.join(" "),
        log_path.display()
    );
    std::fs::write(&launcher_path, &script)?;

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&launcher_path, std::fs::Permissions::from_mode(0o755))?;

    let status = std::process::Command::new(&launcher_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to start mesh-llm via launcher script");
    }

    tracing::info!("mesh-llm started, log: {}", log_path.display());
    Ok(())
}

/// Ensure mesh-llm is running: download if needed, start if not running, wait for ready
async fn ensure_mesh_running(port: u16) -> Result<()> {
    // Already running?
    if is_mesh_running(port).await {
        tracing::info!("mesh-llm already running on port {}", port);
        return Ok(());
    }

    // Find or download binary
    let binary = match find_mesh_binary() {
        Some(bin) => {
            tracing::info!("Found mesh-llm at {}", bin.display());
            bin
        }
        None => {
            tracing::info!("mesh-llm not found, downloading...");
            download_mesh_binary().await?
        }
    };

    // Start it
    start_mesh(&binary, port).await?;

    // Wait for API to become ready (mesh needs to discover, download model, start llama-server)
    // This can take a while on first run
    let timeout = Duration::from_secs(300); // 5 minutes for model download
    let start = std::time::Instant::now();
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        if is_mesh_running(port).await {
            tracing::info!("mesh-llm API ready on port {}", port);
            return Ok(());
        }

        if start.elapsed() > timeout {
            anyhow::bail!(
                "mesh-llm failed to become ready on port {} within {}s. Check log at ~/.mesh-llm/mesh-llm.log",
                port,
                timeout.as_secs()
            );
        }

        if start.elapsed().as_secs() % 30 == 0 {
            tracing::info!(
                "Waiting for mesh-llm to become ready... ({}s elapsed)",
                start.elapsed().as_secs()
            );
        }
    }
}

impl MeshProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let port: u16 = config
            .get_param("MESH_PORT")
            .unwrap_or(MESH_DEFAULT_PORT);

        // Ensure mesh-llm is running (idempotent, only runs once per process)
        MESH_ENSURED
            .get_or_try_init(|| ensure_mesh_running(port))
            .await?;

        let host = format!("http://localhost:{}", port);
        let api_client = ApiClient::new(host, AuthMethod::NoAuth)?;

        Ok(Self {
            api_client,
            model,
            port,
        })
    }
}

impl ProviderDef for MeshProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        let models = MESH_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();
        ProviderMetadata::with_models(
            MESH_PROVIDER_NAME,
            "Mesh",
            "Decentralized local LLM inference via mesh-llm. Automatically downloads and runs models on your GPU.",
            MESH_DEFAULT_MODEL,
            models,
            "https://github.com/michaelneale/decentralized-inference",
            vec![
                ConfigKey::new("MESH_MODEL", false, false, Some(MESH_DEFAULT_MODEL), true),
                ConfigKey::new("MESH_PORT", false, false, Some("9337"), false),
                ConfigKey::new("MESH_INVITE", false, false, None, false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for MeshProvider {
    fn get_name(&self) -> &str {
        MESH_PROVIDER_NAME
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "v1/models")
            .response_get()
            .await?;
        let json = handle_response_openai_compat(response).await?;

        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in models response".into())
        })?;

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Ensure mesh is still running (fast check, cached after first success)
        if !is_mesh_running(self.port).await {
            return Err(ProviderError::ExecutionError(
                "mesh-llm is not running. Restart Goose to retry.".to_string(),
            ));
        }

        let payload = create_request(model_config, system, messages, tools, &ImageFormat::OpenAi, true)?;

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "v1/chat/completions", &payload_clone)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }
}
