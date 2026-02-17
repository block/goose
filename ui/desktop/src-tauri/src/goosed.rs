use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::settings::ExternalGoosedConfig;

pub struct GoosedState {
    pub port: Mutex<Option<u16>>,
    pub secret_key: Mutex<String>,
    pub base_url: Mutex<Option<String>>,
    pub process: Mutex<Option<Child>>,
}

impl Default for GoosedState {
    fn default() -> Self {
        let secret_key = uuid::Uuid::new_v4().to_string();
        Self {
            port: Mutex::new(None),
            secret_key: Mutex::new(secret_key),
            base_url: Mutex::new(None),
            process: Mutex::new(None),
        }
    }
}

pub fn find_goosed_binary(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Check GOOSED_BINARY env var first
    if let Ok(env_path) = std::env::var("GOOSED_BINARY") {
        let p = PathBuf::from(&env_path);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!("GOOSED_BINARY path does not exist: {}", env_path));
    }

    let binary_name = if cfg!(target_os = "windows") {
        "goosed.exe"
    } else {
        "goosed"
    };

    // Check sidecar path (bundled with Tauri)
    let resource_dir = app.path().resource_dir().ok();
    if let Some(ref dir) = resource_dir {
        let sidecar = dir.join("binaries").join(binary_name);
        if sidecar.is_file() {
            return Ok(sidecar);
        }
    }

    // Development paths
    let cwd = std::env::current_dir().unwrap_or_default();
    let possible_paths = vec![
        cwd.join("src").join("bin").join(binary_name),
        cwd.join("..").join("..").join("target").join("release").join(binary_name),
        cwd.join("..").join("..").join("target").join("debug").join(binary_name),
        // From the ui/desktop/src-tauri directory
        cwd.join("..").join("..").join("..").join("target").join("release").join(binary_name),
        cwd.join("..").join("..").join("..").join("target").join("debug").join(binary_name),
        // From the ui/desktop directory
        cwd.join("src").join("bin").join(binary_name),
    ];

    for p in &possible_paths {
        if p.is_file() {
            return Ok(p.canonicalize().unwrap_or_else(|_| p.clone()));
        }
    }

    Err(format!(
        "goosed binary not found. Searched: {:?}",
        possible_paths
    ))
}

fn build_goosed_env(port: u16, secret_key: &str, binary_path: &PathBuf) -> HashMap<String, String> {
    let home_dir = dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("GOOSE_PORT".to_string(), port.to_string());
    env.insert("GOOSE_SERVER__SECRET_KEY".to_string(), secret_key.to_string());
    env.insert("HOME".to_string(), home_dir.clone());

    // Windows-specific env vars
    #[cfg(target_os = "windows")]
    {
        env.insert(
            "USERPROFILE".to_string(),
            home_dir.clone(),
        );
        if let Ok(appdata) = std::env::var("APPDATA") {
            env.insert("APPDATA".to_string(), appdata);
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            env.insert("LOCALAPPDATA".to_string(), localappdata);
        }
    }

    // Add binary directory to PATH
    if let Some(bin_dir) = binary_path.parent() {
        let path_key = if cfg!(target_os = "windows") { "Path" } else { "PATH" };
        let current_path = std::env::var(path_key).unwrap_or_default();
        env.insert(
            path_key.to_string(),
            format!(
                "{}{}{}",
                bin_dir.to_string_lossy(),
                std::path::MAIN_SEPARATOR,
                current_path
            ),
        );
    }

    env
}

pub async fn start_goosed(
    app: &tauri::AppHandle,
    state: &GoosedState,
    external_goosed: Option<&ExternalGoosedConfig>,
) -> Result<String, String> {
    // Check for external goosed configuration
    if let Some(ext) = external_goosed {
        if ext.enabled && !ext.url.is_empty() {
            let url = ext.url.trim_end_matches('/').to_string();
            log::info!("Using external goosed backend at {}", url);
            *state.base_url.lock().unwrap() = Some(url.clone());
            if !ext.secret.is_empty() {
                *state.secret_key.lock().unwrap() = ext.secret.clone();
            }
            return Ok(url);
        }
    }

    // Check for env-based external backend
    if std::env::var("GOOSE_EXTERNAL_BACKEND").is_ok() {
        let port = std::env::var("GOOSE_PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}", port);
        log::info!("Using external goosed backend from env at {}", url);
        *state.base_url.lock().unwrap() = Some(url.clone());
        return Ok(url);
    }

    let goosed_path = find_goosed_binary(app)?;
    let port = portpicker::pick_unused_port().ok_or("Failed to find available port")?;
    let secret_key = state.secret_key.lock().unwrap().clone();

    let working_dir = dirs::home_dir().unwrap_or_default();
    let base_url = format!("http://127.0.0.1:{}", port);

    log::info!(
        "Starting goosed from: {} on port {} in dir {}",
        goosed_path.display(),
        port,
        working_dir.display()
    );

    let goosed_env = build_goosed_env(port, &secret_key, &goosed_path);

    let mut cmd = Command::new(&goosed_path);
    cmd.arg("agent")
        .current_dir(&working_dir)
        .envs(&goosed_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn goosed: {}", e))?;

    // Pipe stdout logging
    if let Some(stdout) = child.stdout.take() {
        let port_copy = port;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[goosed:{}] {}", port_copy, line);
            }
        });
    }

    // Pipe stderr logging
    if let Some(stderr) = child.stderr.take() {
        let port_copy = port;
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if is_fatal_error(&line) {
                    log::error!("[goosed:{}] FATAL: {}", port_copy, line);
                } else {
                    log::warn!("[goosed:{}] {}", port_copy, line);
                }
            }
        });
    }

    // Wait for server to be ready
    let client = reqwest::Client::new();
    let status_url = format!("{}/status", base_url);
    let timeout = std::time::Duration::from_secs(10);
    let interval = std::time::Duration::from_millis(100);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err("Timed out waiting for goosed to start".to_string());
        }
        match client.get(&status_url).send().await {
            Ok(resp) if resp.status().is_success() => break,
            _ => tokio::time::sleep(interval).await,
        }
    }

    *state.port.lock().unwrap() = Some(port);
    *state.base_url.lock().unwrap() = Some(base_url.clone());
    *state.process.lock().unwrap() = Some(child);

    log::info!("goosed server ready on port {}", port);
    Ok(base_url)
}

fn is_fatal_error(line: &str) -> bool {
    let patterns = ["panicked at", "RUST_BACKTRACE", "fatal error"];
    patterns.iter().any(|p| line.contains(p))
}

pub fn stop_goosed(state: &GoosedState) {
    if let Some(mut child) = state.process.lock().unwrap().take() {
        log::info!("Terminating goosed server");
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/pid", &child.id().unwrap_or(0).to_string(), "/f", "/t"])
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = child.start_kill();
        }
    }
}
