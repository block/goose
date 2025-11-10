use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, handle_status_openai_compat};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use rmcp::model::{Role, Tool};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::Duration;
use sysinfo::System;

static LLAMA_SERVER_PIDS: LazyLock<StdMutex<HashSet<u32>>> = LazyLock::new(|| {
    let registry = StdMutex::new(HashSet::new());

    let _ = std::panic::catch_unwind(|| {
        extern "C" fn cleanup_llama_servers() {
            if let Ok(pids) = LLAMA_SERVER_PIDS.lock() {
                for pid in pids.iter() {
                    eprintln!("CLEANUP: Killing llama-server PID {}", pid);
                    #[cfg(unix)]
                    unsafe {
                        let _ = libc::kill(-(*pid as i32), libc::SIGKILL);
                    }
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &pid.to_string()])
                            .output();
                    }
                }
            }
        }
        unsafe {
            libc::atexit(cleanup_llama_servers);
        }
    });

    registry
});
use tokio::pin;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

pub const EMBEDDED_HOST: &str = "127.0.0.1";
pub const EMBEDDED_DEFAULT_PORT: u16 = 8080;
pub const EMBEDDED_DEFAULT_CTX_SIZE: u32 = 8192;
pub const EMBEDDED_TIMEOUT: u64 = 600; // seconds
pub const EMBEDDED_STARTUP_TIMEOUT: u64 = 30; // seconds to wait for server to start
pub const EMBEDDED_DOC_URL: &str =
    "https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md";

/// Maximum size for tool output (4KB) to prevent context overflow with small models
const MAX_TOOL_OUTPUT_SIZE: usize = 4096;

/// Model file size threshold for automatic tool emulation (10GB)
/// Models smaller than this will automatically use emulation mode
const MODEL_SIZE_EMULATION_THRESHOLD: u64 = 10 * 1024 * 1024 * 1024; // 10GB in bytes

/// System prompt for tool emulation mode (when model doesn't support native tool calling)
const EMULATION_SYSTEM_PROMPT: &str = r#"You are Goose, a general-purpose AI agent. Your goal is to analyze and solve problems by writing code.

# Tool Call Format

When you need to execute a tool, write ONLY the JSON tool call on a new line:

{"tool": "tool_name", "args": {"param": "value"}}

The tool will execute immediately and you'll receive the result (success or error) to continue with.

# Available Tools

- **shell**: Execute shell commands
  - Format: {"tool": "shell", "args": {"command": "your_command_here"}}
  - Example: {"tool": "shell", "args": {"command": "ls ~/Downloads"}}

- **final_output**: Signal task completion with a detailed summary of work done
  - Format: {"tool": "final_output", "args": {"summary": "what_was_accomplished"}}

# Instructions

1. Analyze the request and break down into smaller tasks if appropriate
2. Execute ONE tool at a time
3. STOP when the original request was satisfied
4. Call the final_output tool when done

# Response Guidelines

- Use Markdown formatting for all responses except tool calls.
- Whenever taking actions, use the pronoun 'I'
"#;

/// Platform-specific defaults
#[derive(Debug, Clone)]
struct PlatformDefaults {
    gpu_layers: u32,
    batch_size: u32,
    threads: u32,
}

/// Detect platform and return intelligent defaults
fn detect_platform_defaults() -> PlatformDefaults {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_count = sys.cpus().len() as u32;
    let total_memory_gb = sys.total_memory() / (1024 * 1024 * 1024);
    let os = std::env::consts::OS;

    tracing::debug!(
        "Platform detection: OS={}, CPUs={}, Memory={}GB",
        os,
        cpu_count,
        total_memory_gb
    );

    match os {
        "macos" => {
            // Check if Apple Silicon (M1/M2/M3) by checking for arm64
            let is_apple_silicon = std::env::consts::ARCH == "aarch64";

            if is_apple_silicon {
                // Apple Silicon with unified memory - can use more GPU layers
                // Estimate based on available memory
                let gpu_layers = if total_memory_gb >= 64 {
                    80 // High memory system
                } else if total_memory_gb >= 32 {
                    60 // Medium memory system
                } else {
                    40 // Lower memory system
                };

                tracing::info!(
                    "Detected Apple Silicon with {}GB RAM, using {} GPU layers",
                    total_memory_gb,
                    gpu_layers
                );

                PlatformDefaults {
                    gpu_layers,
                    batch_size: 512,
                    threads: cpu_count,
                }
            } else {
                // Intel Mac - likely CPU only
                tracing::info!("Detected Intel Mac, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        "linux" => {
            // Try to detect NVIDIA GPU by checking if nvidia-smi exists
            let has_nvidia = std::process::Command::new("nvidia-smi").output().is_ok();

            if has_nvidia {
                // Assume modern NVIDIA GPU with 8-24GB VRAM
                // Conservative estimate: 1 layer ≈ 200-300MB for a 7B model
                let gpu_layers = 60; // Good for 12-16GB VRAM

                tracing::info!(
                    "Detected Linux with NVIDIA GPU, using {} GPU layers",
                    gpu_layers
                );

                PlatformDefaults {
                    gpu_layers,
                    batch_size: 512,
                    threads: cpu_count,
                }
            } else {
                // CPU-only Linux
                tracing::info!("Detected Linux without NVIDIA GPU, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        "windows" => {
            // Windows - check for NVIDIA
            let has_nvidia = std::process::Command::new("nvidia-smi.exe")
                .output()
                .is_ok();

            if has_nvidia {
                tracing::info!("Detected Windows with NVIDIA GPU, using 60 GPU layers");
                PlatformDefaults {
                    gpu_layers: 60,
                    batch_size: 512, // Slightly more conservative on Windows
                    threads: cpu_count,
                }
            } else {
                tracing::info!("Detected Windows without NVIDIA GPU, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        _ => {
            // Unknown platform - conservative defaults
            tracing::warn!("Unknown platform, using conservative CPU-only defaults");
            PlatformDefaults {
                gpu_layers: 0,
                batch_size: 256,
                threads: cpu_count.min(8),
            }
        }
    }
}

const LLAMA_CPP_VERSION: &str = "b6765";

/// Downloadable model definition
#[derive(Debug, Clone)]
struct DownloadableModel {
    /// Model name (without .gguf extension)
    name: &'static str,
    /// HuggingFace repository (e.g., "mradermacher/gpt-oss-20b-GGUF")
    repo: &'static str,
    /// File name in the repository
    filename: &'static str,
}

/// List of built-in downloadable models that are always available
const DOWNLOADABLE_MODELS: &[DownloadableModel] = &[
    DownloadableModel {
        name: "gpt-oss-20b-Q3_K_M",
        repo: "mradermacher/gpt-oss-20b-GGUF",
        filename: "gpt-oss-20b.Q3_K_M.gguf",
    },
    DownloadableModel {
        name: "qwen2.5-7b-instruct-q3_k_m",
        repo: "bartowski/Qwen2.5-7B-Instruct-GGUF",
        filename: "Qwen2.5-7B-Instruct-Q3_K_M.gguf",
    },
];

/// Check if a model name matches a downloadable model
fn find_downloadable_model(model_name: &str) -> Option<&'static DownloadableModel> {
    DOWNLOADABLE_MODELS
        .iter()
        .find(|m| m.name == model_name || m.name.eq_ignore_ascii_case(model_name))
}

/// Check if a model file exists in ~/.models
fn model_file_exists(model_name: &str) -> bool {
    if let Some(home_dir) = dirs::home_dir() {
        let models_dir = home_dir.join(".models");
        let model_path = models_dir.join(format!("{}.gguf", model_name));
        model_path.exists()
    } else {
        false
    }
}

/// Download a model from HuggingFace to ~/.models
async fn download_model(model: &DownloadableModel) -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let models_dir = home_dir.join(".models");
    std::fs::create_dir_all(&models_dir)?;

    let target_path = models_dir.join(format!("{}.gguf", model.name));

    if target_path.exists() {
        tracing::info!("Model {} already downloaded", model.name);
        return Ok(target_path);
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model.repo, model.filename
    );

    tracing::info!(
        "Downloading model {} from HuggingFace (this may take a while, model size ~3-12GB)...",
        model.name
    );
    tracing::info!("URL: {}", url);

    // Download with progress tracking
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download model: HTTP {}. URL: {}",
            response.status(),
            url
        ));
    }

    let total_size = response.content_length();
    if let Some(size) = total_size {
        tracing::info!(
            "Download size: {:.2} GB",
            size as f64 / (1024.0 * 1024.0 * 1024.0)
        );
    }

    // Download to temporary file first
    let temp_path = models_dir.join(format!(".{}.gguf.tmp", model.name));
    let mut file = tokio::fs::File::create(&temp_path).await?;
    let mut stream = response.bytes_stream();

    let mut downloaded: u64 = 0;
    let mut last_log_percent = 0;

    while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
        let chunk = chunk_result?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        downloaded += chunk.len() as u64;

        // Log progress every 10%
        if let Some(total) = total_size {
            let percent = (downloaded * 100 / total) as u32;
            if percent >= last_log_percent + 10 {
                tracing::info!("Download progress: {}%", percent);
                last_log_percent = percent;
            }
        }
    }

    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);

    // Rename to final location
    tokio::fs::rename(&temp_path, &target_path).await?;

    tracing::info!(
        "Model {} downloaded successfully to {:?}",
        model.name,
        target_path
    );
    Ok(target_path)
}

/// Downloads and extracts llama-server to cache directory, returns path to binary
async fn ensure_llama_server_binary() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("goose")
        .join("llama-server");

    let platform = detect_llama_platform()?;
    let install_dir = cache_dir.join(LLAMA_CPP_VERSION).join(&platform);
    let binary_name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let binary_path = if cfg!(windows) {
        install_dir.join(binary_name)
    } else {
        install_dir.join("build").join("bin").join(binary_name)
    };

    // If cached binary exists, use it
    if binary_path.exists() {
        tracing::debug!("Using cached llama-server: {:?}", binary_path);
        return Ok(binary_path);
    }

    // Download and extract
    tracing::info!(
        "Downloading llama-server {} for {} (one-time setup, ~50MB)...",
        LLAMA_CPP_VERSION,
        platform
    );
    download_and_extract_llama_server(&cache_dir, LLAMA_CPP_VERSION, &platform).await?;

    if !binary_path.exists() {
        return Err(anyhow::anyhow!(
            "Failed to extract llama-server binary after download"
        ));
    }

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))?;

        // Also make the dylibs/so files in the same directory executable if needed
        let bin_dir = binary_path.parent().unwrap();
        for entry in std::fs::read_dir(bin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "dylib" || ext == "so" {
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
                    }
                }
            }
        }
    }

    tracing::info!("llama-server ready at: {:?}", binary_path);
    Ok(binary_path)
}

fn detect_llama_platform() -> Result<String> {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x64",
        ("linux", "x86_64") => "ubuntu-x64",
        ("windows", "x86_64") => "win-cpu-x64",
        (os, arch) => {
            return Err(anyhow::anyhow!(
                "Unsupported platform: {}-{}. llama-server binaries are not available for this platform.",
                os, arch
            ))
        }
    };
    Ok(platform.to_string())
}

async fn download_and_extract_llama_server(
    cache_dir: &Path,
    version: &str,
    platform: &str,
) -> Result<()> {
    let url = format!(
        "https://github.com/ggerganov/llama.cpp/releases/download/{}/llama-{}-bin-{}.zip",
        version, version, platform
    );

    let install_dir = cache_dir.join(version).join(platform);
    std::fs::create_dir_all(&install_dir)?;

    tracing::debug!("Downloading from: {}", url);
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download llama-server: HTTP {}. URL: {}",
            response.status(),
            url
        ));
    }

    let bytes = response.bytes().await?;
    tracing::debug!("Downloaded {} bytes", bytes.len());

    // Extract ZIP
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    tracing::debug!("Extracting {} files...", archive.len());
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = install_dir.join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    tracing::debug!("Extraction complete");
    Ok(())
}

/// Manages a local llama-server process
///
/// This struct ensures single-instance management:
/// - Only one llama-server process runs at a time per ServerProcess instance
/// - Automatically restarts if model changes
/// - Properly shuts down on drop via tokio::spawn pattern (rmcp style)
struct ServerProcess {
    child: Option<tokio::process::Child>,
    port: u16,
    binary_path: Option<PathBuf>,
    /// Track which model is currently loaded to detect model changes
    loaded_model_path: Option<String>,
}

impl ServerProcess {
    fn new(port: u16) -> Self {
        Self {
            child: None,
            port,
            binary_path: None,
            loaded_model_path: None,
        }
    }

    fn find_available_port() -> Result<u16> {
        use std::net::{SocketAddr, TcpListener};

        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr)
            .map_err(|e| anyhow::anyhow!("Failed to bind to find available port: {}", e))?;

        let port = listener
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))?
            .port();

        drop(listener);

        Ok(port)
    }

    fn kill_process_on_port(port: u16) {
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("lsof")
                .args(["-ti", &format!(":{}", port)])
                .output()
            {
                if output.status.success() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    for pid_str in pids.lines() {
                        if let Ok(pid) = pid_str.trim().parse::<i32>() {
                            tracing::info!("Killing process {} on port {}", pid, port);
                            let _ = std::process::Command::new("kill")
                                .arg(pid.to_string())
                                .output();
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            let _ = std::process::Command::new("kill")
                                .args(["-9", &pid.to_string()])
                                .output();
                        }
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            if let Ok(output) = std::process::Command::new("netstat")
                .args(["-ano"])
                .output()
            {
                if output.status.success() {
                    let netstat_output = String::from_utf8_lossy(&output.stdout);
                    for line in netstat_output.lines() {
                        if line.contains(&format!(":{}", port)) {
                            if let Some(pid_str) = line.split_whitespace().last() {
                                if let Ok(pid) = pid_str.parse::<u32>() {
                                    tracing::info!("Killing process {} on port {}", pid, port);
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/PID", &pid.to_string()])
                                        .output();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn start(
        &mut self,
        model_path: &str,
        host: &str,
        ctx_size: u32,
        gpu_layers: u32,
        batch_size: u32,
        threads: u32,
    ) -> Result<()> {
        // Check if model has changed - if so, stop existing server
        if let Some(ref loaded_model) = self.loaded_model_path {
            if loaded_model != model_path && self.child.is_some() {
                tracing::info!(
                    "Model changed from {} to {}, restarting server",
                    loaded_model,
                    model_path
                );
                self.stop().await;
            }
        }

        if self.child.is_some() {
            return Ok(()); // Already running with correct model
        }

        // Kill any existing process on this port to ensure only one embedded server runs
        Self::kill_process_on_port(self.port);

        // Ensure we have the binary (download if needed)
        if self.binary_path.is_none() {
            self.binary_path = Some(ensure_llama_server_binary().await?);
        }

        let binary_path = self.binary_path.as_ref().unwrap();

        tracing::info!(
            "Starting llama-server with model: {} on {}:{}",
            model_path,
            host,
            self.port
        );

        // Create std::process::Command first so we can use pre_exec
        let mut std_command = std::process::Command::new(binary_path);
        std_command
            .arg("--model")
            .arg(model_path)
            .arg("--host")
            .arg(host)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--ctx-size")
            .arg(ctx_size.to_string())
            .arg("--n-gpu-layers")
            .arg(gpu_layers.to_string())
            .arg("--batch-size")
            .arg(batch_size.to_string())
            .arg("--threads")
            .arg(threads.to_string())
            .arg("--jinja")
            .arg("--json-schema")
            .arg("{}")
            .arg("--verbose")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        // On Unix, use pre_exec to set up process group for proper cleanup
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                std_command.pre_exec(|| {
                    // Create a new process group with this process as leader
                    // This allows us to kill the entire process tree with kill(-pid)
                    if libc::setpgid(0, 0) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }

                    #[cfg(target_os = "linux")]
                    {
                        // On Linux, ensure child dies when parent dies
                        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
        }

        // Convert to tokio Command and spawn with kill_on_drop
        let mut command = tokio::process::Command::from(std_command);
        command.kill_on_drop(true);

        let child = command
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start llama-server: {}", e))?;

        // Register PID in global registry for cleanup on exit
        if let Some(pid) = child.id() {
            if let Ok(mut pids) = LLAMA_SERVER_PIDS.lock() {
                pids.insert(pid);
                tracing::info!("Registered llama-server PID {} for cleanup on exit", pid);
            }
        }

        self.child = Some(child);
        self.loaded_model_path = Some(model_path.to_string());
        Ok(())
    }

    async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            tracing::info!("Stopping llama-server on port {}", self.port);

            let pid = child.id();

            // Use the same kill_process_group pattern as MCP
            #[cfg(unix)]
            {
                if let Some(pid) = pid {
                    // Try SIGTERM first
                    unsafe {
                        let _ = libc::kill(-(pid as i32), libc::SIGTERM);
                    }

                    // Wait a brief moment for graceful shutdown
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                    // Force kill with SIGKILL
                    unsafe {
                        let _ = libc::kill(-(pid as i32), libc::SIGKILL);
                    }
                }

                // Final fallback
                let _ = child.kill().await;
            }

            #[cfg(windows)]
            {
                if let Some(pid) = pid {
                    // Use taskkill to kill the process tree on Windows
                    let _ = tokio::process::Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &pid.to_string()])
                        .output()
                        .await;
                }

                // Final fallback
                let _ = child.kill().await;
            }

            // Clear the loaded model path since server is stopped
            self.loaded_model_path = None;
        }
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let pid = child.id();

            tracing::warn!(
                "Dropping ServerProcess - MUST kill llama-server PID {:?} on port {}",
                pid,
                self.port
            );

            // Use the sync kill syscalls directly in Drop (can't await)
            #[cfg(unix)]
            {
                if let Some(pid) = pid {
                    // Kill the entire process group - SIGTERM first
                    let result = unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
                    if result != 0 {
                        tracing::error!(
                            "Failed to send SIGTERM to process group {}: errno {}",
                            pid,
                            result
                        );
                    } else {
                        tracing::info!("Sent SIGTERM to process group {}", pid);
                    }

                    // Give it time to shut down gracefully
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    // Force kill with SIGKILL to ensure it's dead
                    let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
                    if result != 0 {
                        tracing::error!(
                            "Failed to send SIGKILL to process group {}: errno {}",
                            pid,
                            result
                        );
                    } else {
                        tracing::info!("Sent SIGKILL to process group {}", pid);
                    }

                    // Wait a moment for the kill to take effect
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }

            #[cfg(windows)]
            {
                if let Some(pid) = pid {
                    // Use taskkill to kill the process tree on Windows (synchronous)
                    match std::process::Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &pid.to_string()])
                        .output()
                    {
                        Ok(output) => {
                            if !output.status.success() {
                                tracing::error!(
                                    "taskkill failed: {}",
                                    String::from_utf8_lossy(&output.stderr)
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to execute taskkill: {}", e);
                        }
                    }
                }
            }

            // Also try the standard kill as fallback
            if let Err(e) = child.start_kill() {
                tracing::error!("Failed to kill child process: {}", e);
            }

            tracing::info!("ServerProcess Drop completed for port {}", self.port);
        }
    }
}

#[derive(serde::Serialize)]
pub struct EmbeddedProvider {
    name: String,
    #[serde(skip)]
    api_client: ApiClient,
    #[serde(skip)]
    server_process: Arc<Mutex<ServerProcess>>,
    model: ModelConfig,
    model_path: String,
    host: String,
    port: u16,
    ctx_size: u32,
    gpu_layers: u32,
    batch_size: u32,
    threads: u32,
    supports_streaming: bool,
    /// Whether the model supports native tool calling (None = not yet detected)
    #[serde(skip)]
    tool_calling_support: Arc<Mutex<Option<bool>>>,
}

impl EmbeddedProvider {
    /// Enumerate available GGUF models in ~/.models directory
    /// This is a static method that doesn't require a provider instance
    /// Includes both downloaded models and downloadable models (marked with suffix)
    pub fn enumerate_models() -> Result<Vec<String>> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        let models_dir = home_dir.join(".models");

        let mut models = Vec::new();

        // Scan for existing .gguf files in ~/.models
        if models_dir.exists() {
            let entries = std::fs::read_dir(&models_dir)
                .map_err(|e| anyhow::anyhow!("Failed to read models directory: {}", e))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "gguf" {
                            if let Some(file_stem) = path.file_stem() {
                                if let Some(model_name) = file_stem.to_str() {
                                    models.push(model_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add downloadable models to the list
        for downloadable in DOWNLOADABLE_MODELS {
            // Check if model is already downloaded
            if !model_file_exists(downloadable.name) {
                // Mark as downloadable
                models.push(format!("{} (to-download)", downloadable.name));
            } else if !models.contains(&downloadable.name.to_string()) {
                // Already downloaded but not in list
                models.push(downloadable.name.to_string());
            }
        }

        models.sort();
        Ok(models)
    }

    #[allow(clippy::too_many_lines)]
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        let models_dir = home_dir.join(".models");

        let model_name = &model.model_name;
        let clean_model_name = model_name.trim_end_matches(" (to-download)");

        // Find the actual model file
        let model_path: String = if clean_model_name == "embedded" {
            // Pick first available .gguf file
            if let Ok(entries) = std::fs::read_dir(&models_dir) {
                let first_gguf = entries
                    .flatten()
                    .map(|e| e.path())
                    .find(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "gguf"));

                if let Some(gguf_path) = first_gguf {
                    tracing::info!("Using first available GGUF model: {:?}", gguf_path);
                    gguf_path.to_string_lossy().to_string()
                } else {
                    return Err(anyhow::anyhow!(
                        "No GGUF models found in ~/.models/\nPlease add GGUF files to ~/.models/"
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Could not read models directory: {:?}",
                    models_dir
                ));
            }
        } else {
            // Try to find the specific model
            let model_file = if clean_model_name.ends_with(".gguf") {
                models_dir.join(clean_model_name)
            } else {
                models_dir.join(format!("{}.gguf", clean_model_name))
            };

            if !model_file.exists() {
                // we have ability to download some models
                if find_downloadable_model(clean_model_name).is_some() {
                    tracing::info!(
                        "Model '{}' will be downloaded on first use",
                        clean_model_name
                    );
                    model_file.to_string_lossy().to_string()
                } else {
                    // Not a downloadable model - show error
                    let available_models: Vec<String> = std::fs::read_dir(&models_dir)
                        .ok()
                        .map(|entries| {
                            entries
                                .flatten()
                                .filter_map(|e| {
                                    let path = e.path();
                                    if path.is_file()
                                        && path.extension().is_some_and(|ext| ext == "gguf")
                                    {
                                        path.file_stem()
                                            .and_then(|s| s.to_str())
                                            .map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    if available_models.is_empty() {
                        return Err(anyhow::anyhow!(
                            "Model '{}' not found in ~/.models/\nNo GGUF models found. Please add GGUF files to ~/.models/",
                            clean_model_name
                        ));
                    } else {
                        return Err(anyhow::anyhow!(
                            "Model '{}' not found in ~/.models/\n\nAvailable models:\n  {}\n\nPlease use one of these or add your model to ~/.models/",
                            clean_model_name,
                            available_models.join("\n  ")
                        ));
                    }
                }
            } else {
                model_file.to_string_lossy().to_string()
            }
        };

        // Detect platform-specific defaults
        let platform_defaults = detect_platform_defaults();

        let host: String = config
            .get_param("EMBEDDED_HOST")
            .unwrap_or_else(|_| EMBEDDED_HOST.to_string());

        // Find an available port by letting the OS choose (port 0)
        let port = ServerProcess::find_available_port()?;
        tracing::debug!("Allocated port {} for llama-server", port);

        let ctx_size: u32 = config
            .get_param("EMBEDDED_CTX_SIZE")
            .unwrap_or(EMBEDDED_DEFAULT_CTX_SIZE);
        let gpu_layers: u32 = config
            .get_param("EMBEDDED_GPU_LAYERS")
            .unwrap_or(platform_defaults.gpu_layers);
        let batch_size: u32 = config
            .get_param("EMBEDDED_BATCH_SIZE")
            .unwrap_or(platform_defaults.batch_size);
        let threads: u32 = config
            .get_param("EMBEDDED_THREADS")
            .unwrap_or(platform_defaults.threads);
        let timeout: Duration = Duration::from_secs(
            config
                .get_param("EMBEDDED_TIMEOUT")
                .unwrap_or(EMBEDDED_TIMEOUT),
        );

        tracing::info!(
            "Local provider configuration: port={}, gpu_layers={}, batch_size={}, threads={}, ctx_size={}",
            port,
            gpu_layers,
            batch_size,
            threads,
            ctx_size
        );

        let base_url = format!("http://{}:{}", host, port);
        let url = Url::parse(&base_url).map_err(|e| anyhow::anyhow!("Invalid base URL: {}", e))?;

        // No authentication needed for local server
        let auth = AuthMethod::Custom(Box::new(NoAuth));
        let api_client = ApiClient::with_timeout(url.to_string(), auth, timeout)?;

        let server_process = Arc::new(Mutex::new(ServerProcess::new(port)));

        let provider = Self {
            name: "embedded".to_string(),
            api_client,
            server_process,
            model,
            model_path,
            host,
            port,
            ctx_size,
            gpu_layers,
            batch_size,
            threads,
            supports_streaming: true,
            tool_calling_support: Arc::new(Mutex::new(None)),
        };

        // Don't start the server here - it will be started lazily on first use
        // This makes provider creation fast for operations like fetching model lists

        Ok(provider)
    }

    async fn ensure_server_running(&self) -> Result<()> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        let models_dir = home_dir.join(".models");

        let model_name = &self.model.model_name;
        let clean_model_name = model_name.trim_end_matches(" (to-download)");

        if let Some(downloadable) = find_downloadable_model(clean_model_name) {
            let model_path_candidate = models_dir.join(format!("{}.gguf", downloadable.name));

            if !model_path_candidate.exists() {
                tracing::info!(
                    "Model {} is not downloaded, starting download...",
                    downloadable.name
                );
                download_model(downloadable).await?;
            }
        }

        let actual_model_path = if clean_model_name.ends_with(".gguf") {
            models_dir.join(clean_model_name)
        } else {
            models_dir.join(format!("{}.gguf", clean_model_name))
        };
        let actual_model_path_str = actual_model_path.to_string_lossy().to_string();

        let mut process = self.server_process.lock().await;

        let was_running_before = process.child.is_some();
        let model_before = process.loaded_model_path.clone();

        process
            .start(
                &actual_model_path_str,
                &self.host,
                self.ctx_size,
                self.gpu_layers,
                self.batch_size,
                self.threads,
            )
            .await?;

        // Determine if we need to wait: either wasn't running before, or model changed
        let needs_wait =
            !was_running_before || model_before.as_deref() != Some(&actual_model_path_str);
        drop(process);

        if needs_wait {
            self.wait_for_server_ready().await?;
        }

        Ok(())
    }

    async fn wait_for_server_ready(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(EMBEDDED_STARTUP_TIMEOUT);

        tracing::info!(
            "Waiting for llama-server to load model on port {}...",
            self.port
        );

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for llama-server to start and load model"
                ));
            }

            // Try to make a small test request to verify the model is loaded
            // The health endpoint might return OK before the model is ready
            let test_payload = json!({
                "model": "embedded",
                "messages": [{"role": "user", "content": "test"}],
                "max_tokens": 1,
                "temperature": 0.0
            });

            match self
                .api_client
                .response_post("v1/chat/completions", &test_payload)
                .await
            {
                Ok(response) => {
                    // Check if we got a valid response (not a 503 loading error)
                    match handle_response_openai_compat(response).await {
                        Ok(_) => {
                            tracing::info!(
                                "llama-server is ready and model loaded on port {}",
                                self.port
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("{:?}", e);
                            if error_msg.contains("503") || error_msg.contains("Loading model") {
                                tracing::debug!("Model still loading, waiting...");
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            } else {
                                // Some other error - might indicate server issues
                                tracing::debug!(
                                    "Server error during readiness check: {}, retrying...",
                                    e
                                );
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                        }
                    }
                }
                Err(_) => {
                    // Server not responding yet
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        self.ensure_server_running()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to start server: {}", e)))?;

        let response = self
            .api_client
            .response_post("v1/chat/completions", payload)
            .await?;
        handle_response_openai_compat(response).await
    }

    /// Detect if the model supports native tool calling
    async fn detect_tool_support(&self) -> bool {
        // Create a simple test tool
        let test_tool = Tool::new(
            "test".to_string(),
            "test tool".to_string(),
            serde_json::Map::new(),
        );

        let test_message = Message::user().with_text("test");

        let test_payload = match create_request(
            &self.model,
            "test",
            &[test_message],
            &[test_tool],
            &super::utils::ImageFormat::OpenAi,
        ) {
            Ok(payload) => payload,
            Err(_) => return false,
        };

        match self.post(&test_payload).await {
            Ok(response) => {
                // Check if response has tool_calls in the expected OpenAI format
                response
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("tool_calls"))
                    .is_some()
            }
            Err(_) => false,
        }
    }

    /// Get or detect tool calling support
    async fn get_tool_support(&self) -> bool {
        // Check if forced emulation is enabled FIRST (before cache)
        let config = crate::config::Config::global();
        let force_emulation = config
            .get_param::<bool>("EMBEDDED_FORCE_TOOL_EMULATION")
            .unwrap_or(false);

        if force_emulation {
            tracing::info!("Tool emulation forced via EMBEDDED_FORCE_TOOL_EMULATION");
            return false;
        }

        // Check model file size - models < 10GB automatically use emulation
        match std::fs::metadata(&self.model_path) {
            Ok(metadata) => {
                let size_gb = metadata.len() as f64 / (1024.0 * 1024.0 * 1024.0);
                if metadata.len() < MODEL_SIZE_EMULATION_THRESHOLD {
                    tracing::info!(
                        "Model size {:.2}GB is below 10GB threshold, automatically using tool emulation mode",
                        size_gb
                    );
                    return false;
                }
                tracing::debug!(
                    "Model size: {:.2}GB, will test for native tool calling support",
                    size_gb
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Could not determine model file size: {}, will test for tool support",
                    e
                );
            }
        }

        let mut support = self.tool_calling_support.lock().await;

        if let Some(cached) = *support {
            return cached;
        }

        // Detect support for larger models
        tracing::debug!("Detecting tool calling support...");
        let detected = self.detect_tool_support().await;

        if detected {
            tracing::info!("Model supports native tool calling");
        } else {
            tracing::info!("Model does not support native tool calling, will use emulation mode");
        }

        *support = Some(detected);
        detected
    }
}

/// Tool executor for emulation mode (when model doesn't support native tool calling)
struct ToolExecutor;

impl ToolExecutor {
    async fn execute_tool_calls(text: &str) -> String {
        let mut result = String::new();
        let mut remaining = text;

        while let Some(start_idx) = remaining.find('{') {
            result.push_str(remaining.get(..start_idx).unwrap_or(""));

            let json_start = remaining.get(start_idx..).unwrap_or("");
            if let Some(end_idx) = Self::find_json_end(json_start) {
                let json_str = json_start.get(..=end_idx).unwrap_or("");

                match serde_json::from_str::<Value>(json_str) {
                    Ok(json) if Self::is_valid_tool_call(&json) => {
                        let tool_name = json.get("tool").and_then(|t| t.as_str());

                        match tool_name {
                            Some("shell") => {
                                if let Some(tool_result) = Self::execute_tool_call(&json).await {
                                    if let Some(args) = json.get("args") {
                                        if let Some(command) =
                                            args.get("command").and_then(|v| v.as_str())
                                        {
                                            result.push_str(&format!(
                                                "**Running command:** `{}`\n\n",
                                                command
                                            ));
                                        }
                                    }
                                    result.push_str(&tool_result);
                                }
                            }
                            _ => {
                                let formatted = Self::format_json_as_text(&json);
                                if !formatted.is_empty() {
                                    result.push_str(&formatted);
                                } else {
                                    result.push_str(json_str);
                                }
                            }
                        }
                        remaining = json_start.get(end_idx + 1..).unwrap_or("");
                    }
                    _ => {
                        result.push_str(json_str);
                        remaining = json_start.get(end_idx + 1..).unwrap_or("");
                    }
                }
            } else {
                result.push_str(remaining);
                break;
            }
        }

        result.push_str(remaining);

        if result.is_empty() {
            text.to_string()
        } else {
            result
        }
    }

    /// this is to help identify if we should show it as plain text
    fn is_valid_tool_call(json: &Value) -> bool {
        let has_valid_tool = json
            .get("tool")
            .and_then(|t| t.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        let has_args = json.get("args").is_some();

        has_valid_tool && has_args
    }

    /// Format JSON tool call as human-readable text
    /// Extracts content from args, formats as clean text without JSON syntax
    fn format_json_as_text(json: &Value) -> String {
        if let Some(args) = json.get("args") {
            if let Some(content) = args
                .get("summary")
                .or_else(|| args.get("message"))
                .or_else(|| args.get("content"))
                .or_else(|| args.get("result"))
                .or_else(|| args.get("text"))
                .and_then(|v| v.as_str())
            {
                return content.to_string();
            }

            if let Some(obj) = args.as_object() {
                if !obj.is_empty() {
                    let mut lines = Vec::new();
                    for (key, value) in obj {
                        let value_str = match value {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            Value::Null => "null".to_string(),
                            Value::Array(arr) => arr
                                .iter()
                                .map(|v| match v {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                            Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
                        };
                        lines.push(format!("{}: {}", key, value_str));
                    }
                    return lines.join("\n");
                }
            }
        }

        String::new()
    }

    /// Find the end of a JSON object in text
    fn find_json_end(text: &str) -> Option<usize> {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in text.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Execute a single tool call
    async fn execute_tool_call(json: &Value) -> Option<String> {
        let tool_name = json.get("tool")?.as_str()?;
        let args = json.get("args")?;

        match tool_name {
            "shell" => {
                if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                    tracing::info!("Executing shell command: {}", command);

                    #[cfg(target_os = "windows")]
                    let output_result = std::process::Command::new("cmd")
                        .args(["/C", command])
                        .output();

                    #[cfg(not(target_os = "windows"))]
                    let output_result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .output();

                    match output_result {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);

                            // Truncate if output is too large
                            let truncate_if_large = |s: &str| -> String {
                                if s.len() > MAX_TOOL_OUTPUT_SIZE {
                                    // Use char_indices to find a safe truncation point
                                    let truncate_at = s
                                        .char_indices()
                                        .take_while(|(idx, _)| *idx < MAX_TOOL_OUTPUT_SIZE)
                                        .last()
                                        .map(|(idx, ch)| idx + ch.len_utf8())
                                        .unwrap_or(0);
                                    format!(
                                        "{}... [truncated, {} bytes total]",
                                        s.get(..truncate_at).unwrap_or(""),
                                        s.len()
                                    )
                                } else {
                                    s.to_string()
                                }
                            };

                            let stdout_display = truncate_if_large(&stdout);
                            let stderr_display = truncate_if_large(&stderr);

                            if output.status.success() {
                                Some(format!(
                                    "Command executed successfully:\n```\n{}\n```",
                                    stdout_display
                                ))
                            } else {
                                Some(format!(
                                    "Command failed with exit code {}:\nstdout:\n```\n{}\n```\nstderr:\n```\n{}\n```",
                                    output.status.code().unwrap_or(-1),
                                    stdout_display,
                                    stderr_display
                                ))
                            }
                        }
                        Err(e) => Some(format!("Failed to execute command: {}", e)),
                    }
                } else {
                    Some("Error: shell tool requires 'command' argument".to_string())
                }
            }
            "final_output" => {
                // final_output is a no-op, just signals completion
                tracing::info!("Task completed");
                None
            }
            _ => Some(format!("Error: Unknown tool '{}'", tool_name)),
        }
    }
}

// No authentication provider
struct NoAuth;

#[async_trait]
impl super::api_client::AuthProvider for NoAuth {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        Ok(("X-No-Auth".to_string(), "true".to_string()))
    }
}

#[async_trait]
impl Provider for EmbeddedProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "embedded",
            "Local",
            "Local GGUF models via llama-server (auto-detects platform, looks in ~/.models)",
            "embedded",
            vec!["embedded"],
            EMBEDDED_DOC_URL,
            vec![
                ConfigKey::new("EMBEDDED_MODEL_PATH", false, false, None),
                ConfigKey::new("EMBEDDED_HOST", false, false, Some(EMBEDDED_HOST)),
                ConfigKey::new(
                    "EMBEDDED_PORT",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_PORT.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_CTX_SIZE",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_CTX_SIZE.to_string()),
                ),
                ConfigKey::new("EMBEDDED_GPU_LAYERS", false, false, Some("auto-detected")),
                ConfigKey::new("EMBEDDED_BATCH_SIZE", false, false, Some("auto-detected")),
                ConfigKey::new("EMBEDDED_THREADS", false, false, Some("auto-detected")),
                ConfigKey::new(
                    "EMBEDDED_TIMEOUT",
                    false,
                    false,
                    Some(&EMBEDDED_TIMEOUT.to_string()),
                ),
                ConfigKey::new("EMBEDDED_FORCE_TOOL_EMULATION", false, false, Some("false")),
            ],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Use the static enumerate_models method which includes downloadable models
        // This avoids needing to start the server just to list models
        match Self::enumerate_models() {
            Ok(models) => {
                if models.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(models))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to enumerate models: {}", e);
                Ok(None)
            }
        }
    }

    #[tracing::instrument(
        skip(self, _model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = crate::config::Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
        let filtered_tools = if goose_mode == "chat" { &[] } else { tools };

        // Check if we should use emulation mode
        let use_emulation = !filtered_tools.is_empty() && !self.get_tool_support().await;

        if use_emulation {
            tracing::info!("Using tool emulation mode");

            // Use emulation prompt as system parameter (not user message) to avoid Jinja issues
            let emulation_payload = create_request(
                &self.model,
                EMULATION_SYSTEM_PROMPT, // Emulation prompt as system
                messages,
                &[], // No tools parameter
                &super::utils::ImageFormat::OpenAi,
            )?;

            let response = self
                .with_retry(|| async {
                    let payload_clone = emulation_payload.clone();
                    self.post(&payload_clone).await
                })
                .await?;

            let message = response_to_message(&response)?;

            // Extract text from message and execute tool calls
            let text = message.as_concat_text();
            let augmented_text = ToolExecutor::execute_tool_calls(&text).await;

            // Create new message with augmented text
            let augmented_message = Message::new(
                Role::Assistant,
                chrono::Utc::now().timestamp(),
                vec![MessageContent::text(augmented_text)],
            );

            let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });

            let response_model = get_model(&response);
            Ok((augmented_message, ProviderUsage::new(response_model, usage)))
        } else {
            // Use native tool calling (current path)
            let payload = create_request(
                &self.model,
                system,
                messages,
                filtered_tools,
                &super::utils::ImageFormat::OpenAi,
            )?;

            let response = self
                .with_retry(|| async {
                    let payload_clone = payload.clone();
                    self.post(&payload_clone).await
                })
                .await?;

            let message = response_to_message(&response)?;

            let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });

            let response_model = get_model(&response);
            Ok((message, ProviderUsage::new(response_model, usage)))
        }
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.ensure_server_running()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to start server: {}", e)))?;

        let config = crate::config::Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
        let filtered_tools = if goose_mode == "chat" { &[] } else { tools };

        // Check if we should use emulation mode
        let use_emulation = !filtered_tools.is_empty() && !self.get_tool_support().await;

        // Determine what to send based on emulation mode
        let (final_system, final_messages, final_tools) = if use_emulation {
            tracing::info!("Using tool emulation mode in streaming");
            // Use emulation prompt as system parameter to avoid Jinja template issues
            (EMULATION_SYSTEM_PROMPT, messages.to_vec(), vec![])
        } else {
            (system, messages.to_vec(), filtered_tools.to_vec())
        };

        let mut payload = create_request(
            &self.model,
            final_system,
            &final_messages,
            &final_tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        let response = self
            .api_client
            .response_post("v1/chat/completions", &payload)
            .await?;
        let response = handle_status_openai_compat(response).await?;
        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);
            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);

            let mut collected_text = String::new();
            let mut last_usage = None;

            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;

                if use_emulation {
                    // In emulation mode, collect text for tool execution later
                    if let Some(ref msg) = message {
                        collected_text.push_str(&msg.as_concat_text());
                    }
                    last_usage = usage;
                } else {
                    // In native mode, yield directly
                    yield (message, usage);
                }
            }

            // If in emulation mode, execute tools and yield augmented message
            if use_emulation && !collected_text.is_empty() {
                let augmented_text = ToolExecutor::execute_tool_calls(&collected_text).await;
                let augmented_message = Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    vec![MessageContent::text(augmented_text)],
                );
                yield (Some(augmented_message), last_usage);
            }
        }))
    }
}
