//! PostHog telemetry - fires once per session creation.

use crate::config::paths::Paths;
use crate::config::{get_enabled_extensions, Config};
use crate::session::SessionManager;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use uuid::Uuid;

const POSTHOG_API_KEY: &str = "phc_RyX5CaY01VtZJCQyhSR5KFh6qimUy81YwxsEpotAftT";

/// Config key for telemetry opt-out preference
pub const TELEMETRY_ENABLED_KEY: &str = "GOOSE_TELEMETRY_ENABLED";

static TELEMETRY_DISABLED_BY_ENV: Lazy<AtomicBool> = Lazy::new(|| {
    std::env::var("GOOSE_TELEMETRY_OFF")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
        .into()
});

/// Check if telemetry is enabled.
///
/// Returns false if:
/// - GOOSE_TELEMETRY_OFF environment variable is set to "1" or "true"
/// - GOOSE_TELEMETRY_ENABLED config value is set to false
///
/// Returns true otherwise (telemetry is opt-out, enabled by default)
pub fn is_telemetry_enabled() -> bool {
    if TELEMETRY_DISABLED_BY_ENV.load(Ordering::Relaxed) {
        return false;
    }

    let config = Config::global();
    config
        .get_param::<bool>(TELEMETRY_ENABLED_KEY)
        .unwrap_or(true)
}

// ============================================================================
// Installation Tracking
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallationData {
    installation_id: String,
    first_seen: DateTime<Utc>,
    session_count: u32,
}

impl Default for InstallationData {
    fn default() -> Self {
        Self {
            installation_id: Uuid::new_v4().to_string(),
            first_seen: Utc::now(),
            session_count: 0,
        }
    }
}

fn installation_file_path() -> std::path::PathBuf {
    Paths::state_dir().join("telemetry_installation.json")
}

fn load_or_create_installation() -> InstallationData {
    let path = installation_file_path();

    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<InstallationData>(&contents) {
            return data;
        }
    }

    let data = InstallationData::default();
    save_installation(&data);
    data
}

fn save_installation(data: &InstallationData) {
    let path = installation_file_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, json);
    }
}

fn increment_session_count() -> InstallationData {
    let mut data = load_or_create_installation();
    data.session_count += 1;
    save_installation(&data);
    data
}

// ============================================================================
// Platform Info
// ============================================================================

fn get_platform_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
    #[cfg(target_os = "linux")]
    {
        fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("VERSION_ID="))
                    .map(|line| {
                        line.trim_start_matches("VERSION_ID=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn detect_install_method() -> String {
    let exe_path = std::env::current_exe().ok();

    if let Some(path) = exe_path {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains("homebrew") || path_str.contains("/opt/homebrew") {
            return "homebrew".to_string();
        }
        if path_str.contains(".cargo") {
            return "cargo".to_string();
        }
        if path_str.contains("applications") || path_str.contains(".app") {
            return "desktop".to_string();
        }
    }

    if std::env::var("GOOSE_DESKTOP").is_ok() {
        return "desktop".to_string();
    }

    "binary".to_string()
}

// ============================================================================
// Session Context (set by CLI/Desktop at startup)
// ============================================================================

static SESSION_INTERFACE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static SESSION_IS_RESUMED: AtomicBool = AtomicBool::new(false);

pub fn set_session_context(interface: &str, is_resumed: bool) {
    if let Ok(mut iface) = SESSION_INTERFACE.lock() {
        *iface = Some(interface.to_string());
    }
    SESSION_IS_RESUMED.store(is_resumed, Ordering::Relaxed);
}

fn get_session_interface() -> String {
    SESSION_INTERFACE
        .lock()
        .ok()
        .and_then(|i| i.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_session_is_resumed() -> bool {
    SESSION_IS_RESUMED.load(Ordering::Relaxed)
}

// ============================================================================
// Telemetry Events
// ============================================================================

pub fn emit_session_started() {
    if !is_telemetry_enabled() {
        return;
    }

    let installation = increment_session_count();

    tokio::spawn(async move {
        let _ = send_session_event(&installation).await;
    });
}

pub fn emit_error(error_type: &str) {
    if !is_telemetry_enabled() {
        return;
    }

    let installation = load_or_create_installation();
    let error_type = error_type.to_string();

    tokio::spawn(async move {
        let _ = send_error_event(&installation, &error_type).await;
    });
}

async fn send_error_event(installation: &InstallationData, error_type: &str) -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new("error", &installation.installation_id);

    event.insert_prop("error_type", error_type).ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();
    event.insert_prop("interface", get_session_interface()).ok();
    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();

    if let Some(platform_version) = get_platform_version() {
        event.insert_prop("platform_version", platform_version).ok();
    }

    let config = Config::global();
    if let Ok(provider) = config.get_param::<String>("GOOSE_PROVIDER") {
        event.insert_prop("provider", provider).ok();
    }
    if let Ok(model) = config.get_param::<String>("GOOSE_MODEL") {
        event.insert_prop("model", model).ok();
    }

    client.capture(event).await.map_err(|e| format!("{:?}", e))
}

async fn send_session_event(installation: &InstallationData) -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new("session_started", &installation.installation_id);

    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();

    if let Some(platform_version) = get_platform_version() {
        event.insert_prop("platform_version", platform_version).ok();
    }

    event
        .insert_prop("install_method", detect_install_method())
        .ok();

    event.insert_prop("interface", get_session_interface()).ok();

    event
        .insert_prop("is_resumed", get_session_is_resumed())
        .ok();

    event
        .insert_prop("session_number", installation.session_count)
        .ok();
    let days_since_install = (Utc::now() - installation.first_seen).num_days();
    event
        .insert_prop("days_since_install", days_since_install)
        .ok();

    let config = Config::global();
    if let Ok(provider) = config.get_param::<String>("GOOSE_PROVIDER") {
        event.insert_prop("provider", provider).ok();
    }
    if let Ok(model) = config.get_param::<String>("GOOSE_MODEL") {
        event.insert_prop("model", model).ok();
    }

    let extensions = get_enabled_extensions();
    event.insert_prop("extensions_count", extensions.len()).ok();
    let extension_names: Vec<String> = extensions.iter().map(|e| e.name()).collect();
    event.insert_prop("extensions", extension_names).ok();

    if let Ok(insights) = SessionManager::get_insights().await {
        event
            .insert_prop("total_sessions", insights.total_sessions)
            .ok();
        event
            .insert_prop("total_tokens", insights.total_tokens)
            .ok();
    }

    client.capture(event).await.map_err(|e| format!("{:?}", e))
}
