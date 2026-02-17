use crate::goosed::GoosedState;
use crate::settings::{Settings, SettingsState};
use crate::wakelock::WakelockState;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

// ── Settings ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, SettingsState>) -> Result<Settings, String> {
    let settings = state.0.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    state: tauri::State<'_, SettingsState>,
) -> Result<bool, String> {
    settings.save()?;
    *state.0.lock().map_err(|e| e.to_string())? = settings;
    Ok(true)
}

// ── Goosed connection ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_secret_key(state: tauri::State<'_, GoosedState>) -> Result<String, String> {
    let key = state.secret_key.lock().map_err(|e| e.to_string())?;
    Ok(key.clone())
}

#[tauri::command]
pub fn get_goosed_host_port(state: tauri::State<'_, GoosedState>) -> Result<Option<String>, String> {
    let url = state.base_url.lock().map_err(|e| e.to_string())?;
    Ok(url.clone())
}

// ── File operations ───────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResponse {
    pub file: String,
    pub file_path: String,
    pub error: Option<String>,
    pub found: bool,
}

#[tauri::command]
pub fn read_file(file_path: String) -> FileResponse {
    match fs::read_to_string(&file_path) {
        Ok(content) => FileResponse {
            file: content,
            file_path,
            error: None,
            found: true,
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileResponse {
            file: String::new(),
            file_path,
            error: None,
            found: false,
        },
        Err(e) => FileResponse {
            file: String::new(),
            file_path,
            error: Some(e.to_string()),
            found: false,
        },
    }
}

#[tauri::command]
pub fn write_file(file_path: String, content: String) -> Result<bool, String> {
    if let Some(parent) = PathBuf::from(&file_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&file_path, content).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn ensure_directory(dir_path: String) -> Result<bool, String> {
    fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn list_files(dir_path: String, extension: Option<String>) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(&dir_path).map_err(|e| e.to_string())?;
    let mut files: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ref ext) = extension {
            if path.extension().and_then(|e| e.to_str()) == Some(ext.as_str()) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files.push(name.to_string());
                }
            }
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            files.push(name.to_string());
        }
    }
    files.sort();
    Ok(files)
}

#[tauri::command]
pub fn open_directory_in_explorer(directory_path: String) -> Result<bool, String> {
    open::that(&directory_path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn select_file_or_directory(
    app: tauri::AppHandle,
    default_path: Option<String>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    if let Some(ref path) = default_path {
        builder = builder.set_directory(path);
    }
    let result = builder.blocking_pick_file();
    Ok(result.map(|p| p.to_string()))
}

// ── Window management ─────────────────────────────────────────────────

#[tauri::command]
pub async fn create_chat_window(
    app: tauri::AppHandle,
    query: Option<String>,
    dir: Option<String>,
    version: Option<String>,
    resume_session_id: Option<String>,
    view_type: Option<String>,
    recipe_deeplink: Option<String>,
) -> Result<(), String> {
    let label = format!("chat-{}", uuid::Uuid::new_v4());
    let mut url_parts: Vec<String> = Vec::new();

    if let Some(ref q) = query {
        url_parts.push(format!("initialQuery={}", urlencoding(q)));
    }
    if let Some(ref d) = dir {
        url_parts.push(format!("dir={}", urlencoding(d)));
    }
    if let Some(ref v) = version {
        url_parts.push(format!("version={}", urlencoding(v)));
    }
    if let Some(ref id) = resume_session_id {
        url_parts.push(format!("resumeSessionId={}", urlencoding(id)));
    }
    if let Some(ref vt) = view_type {
        url_parts.push(format!("viewType={}", urlencoding(vt)));
    }
    if let Some(ref rd) = recipe_deeplink {
        url_parts.push(format!("recipeDeeplink={}", urlencoding(rd)));
    }

    let url = if url_parts.is_empty() {
        "/".to_string()
    } else {
        format!("/?{}", url_parts.join("&"))
    };

    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App(url.into()))
        .title("Goose")
        .inner_size(750.0, 730.0)
        .min_inner_size(560.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn urlencoding(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
}

// ── System state ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn set_wakelock(
    enable: bool,
    state: tauri::State<'_, WakelockState>,
) -> Result<bool, String> {
    crate::wakelock::set_wakelock_platform(&state, enable)
}

#[tauri::command]
pub fn get_wakelock_state(state: tauri::State<'_, WakelockState>) -> bool {
    *state.enabled.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
pub fn set_spellcheck(
    enable: bool,
    state: tauri::State<'_, SettingsState>,
) -> Result<bool, String> {
    let mut settings = state.0.lock().map_err(|e| e.to_string())?;
    settings.spellcheck_enabled = enable;
    settings.save()?;
    Ok(enable)
}

#[tauri::command]
pub fn get_spellcheck_state(state: tauri::State<'_, SettingsState>) -> Result<bool, String> {
    let settings = state.0.lock().map_err(|e| e.to_string())?;
    Ok(settings.spellcheck_enabled)
}

// ── Config ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(
    app: tauri::AppHandle,
    goosed_state: tauri::State<'_, GoosedState>,
) -> HashMap<String, serde_json::Value> {
    let mut config: HashMap<String, serde_json::Value> = HashMap::new();

    // Version
    config.insert(
        "GOOSE_VERSION".to_string(),
        serde_json::Value::String(app.package_info().version.to_string()),
    );

    // Insert goosed base_url as GOOSE_API_HOST (takes precedence over env var)
    if let Ok(url) = goosed_state.base_url.lock() {
        if let Some(ref url) = *url {
            config.insert(
                "GOOSE_API_HOST".to_string(),
                serde_json::Value::String(url.clone()),
            );
        }
    }

    // Environment-based config (only insert if not already set by goosed state)
    for key in &[
        "GOOSE_API_HOST",
        "GOOSE_WORKING_DIR",
        "GOOSE_DEFAULT_PROVIDER",
        "GOOSE_DEFAULT_MODEL",
        "GOOSE_BASE_URL_SHARE",
        "GOOSE_PREDEFINED_MODELS",
        "GOOSE_TUNNEL",
        "SECURITY_ML_MODEL_MAPPING",
    ] {
        if !config.contains_key(*key) {
            if let Ok(val) = std::env::var(key) {
                config.insert(key.to_string(), serde_json::Value::String(val));
            }
        }
    }

    // Alpha flag
    if std::env::var("ALPHA").unwrap_or_default() == "true" {
        config.insert(
            "ALPHA".to_string(),
            serde_json::Value::Bool(true),
        );
    }

    config
}

// ── Ollama ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_for_ollama() -> bool {
    match reqwest::get("http://127.0.0.1:11434/api/tags").await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// ── Metadata ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_metadata(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

// ── Extensions ────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_allowed_extensions() -> Vec<String> {
    // Return allowed extensions list from config or default
    Vec::new()
}

// ── Recent directories ───────────────────────────────────────────────

#[tauri::command]
pub fn add_recent_dir(dir: String) -> Result<bool, String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    let recent_file = config_dir.join("Goose").join("recent_dirs.json");

    let mut dirs: Vec<String> = if recent_file.exists() {
        let content = fs::read_to_string(&recent_file).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Remove if already exists, then prepend
    dirs.retain(|d| d != &dir);
    dirs.insert(0, dir);
    dirs.truncate(10); // Keep max 10 recent dirs

    if let Some(parent) = recent_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(&dirs).map_err(|e| e.to_string())?;
    fs::write(&recent_file, content).map_err(|e| e.to_string())?;

    Ok(true)
}

// ── Recipe tracking ──────────────────────────────────────────────────

#[tauri::command]
pub fn has_accepted_recipe_before(recipe: serde_json::Value) -> Result<bool, String> {
    let hash = recipe_hash(&recipe);
    let accepted_file = recipe_accepted_file();

    if !accepted_file.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&accepted_file).map_err(|e| e.to_string())?;
    let hashes: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
    Ok(hashes.contains(&hash))
}

#[tauri::command]
pub fn record_recipe_hash(recipe: serde_json::Value) -> Result<bool, String> {
    let hash = recipe_hash(&recipe);
    let accepted_file = recipe_accepted_file();

    let mut hashes: Vec<String> = if accepted_file.exists() {
        let content = fs::read_to_string(&accepted_file).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    if !hashes.contains(&hash) {
        hashes.push(hash);
    }

    if let Some(parent) = accepted_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(&hashes).map_err(|e| e.to_string())?;
    fs::write(&accepted_file, content).map_err(|e| e.to_string())?;
    Ok(true)
}

fn recipe_hash(recipe: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_string(recipe).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    hex::encode(hasher.finalize())
}

fn recipe_accepted_file() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    config_dir.join("Goose").join("accepted_recipes.json")
}

// ── App management (GooseApps) ───────────────────────────────────────

#[tauri::command]
pub async fn launch_app(app: tauri::AppHandle, goose_app: serde_json::Value) -> Result<(), String> {
    let name = goose_app
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("app");
    let url = goose_app
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("/");

    let label = format!("app-{}", name.replace(' ', "-").to_lowercase());

    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().map_err(|e: url::ParseError| e.to_string())?))
        .title(name)
        .inner_size(800.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn refresh_app(app: tauri::AppHandle, goose_app: serde_json::Value) -> Result<(), String> {
    let name = goose_app
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("app");
    let label = format!("app-{}", name.replace(' ', "-").to_lowercase());

    if let Some(window) = app.get_webview_window(&label) {
        // Reload by evaluating JS
        let _ = window.eval("window.location.reload()");
    }
    Ok(())
}

#[tauri::command]
pub fn close_app(app: tauri::AppHandle, app_name: String) -> Result<(), String> {
    let label = format!("app-{}", app_name.replace(' ', "-").to_lowercase());
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Logging from frontend ────────────────────────────────────────────

#[tauri::command]
pub fn log_from_frontend(message: String) {
    log::info!("[frontend] {}", message);
}

// ── Open in Chrome ──────────────────────────────────────────────────

#[tauri::command]
pub fn open_in_chrome(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Google Chrome", &url])
            .spawn()
            .or_else(|_| {
                // Fallback to default browser
                std::process::Command::new("open").arg(&url).spawn()
            })
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "chrome", &url])
            .spawn()
            .or_else(|_| {
                std::process::Command::new("cmd")
                    .args(["/c", "start", &url])
                    .spawn()
            })
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("google-chrome")
            .arg(&url)
            .spawn()
            .or_else(|_| {
                std::process::Command::new("xdg-open").arg(&url).spawn()
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Restart ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

// ── Deep link handling ───────────────────────────────────────────────

pub fn handle_deep_link(app: &tauri::AppHandle, url: &str) {
    log::info!("Handling deep link: {}", url);

    // Parse the deep link URL and emit appropriate events
    if url.starts_with("goose://bot/") || url.starts_with("goose://recipe/") {
        let _ = app.emit("deep-link-recipe", url);
    } else if url.starts_with("goose://extension/") {
        let _ = app.emit("add-extension", url);
    } else if url.starts_with("goose://sessions/") {
        let _ = app.emit("open-shared-session", url);
    } else {
        let _ = app.emit("deep-link", url);
    }
}
