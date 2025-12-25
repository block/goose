//! PostHog telemetry - fires once per session creation.

use crate::config::paths::Paths;
use crate::config::{get_enabled_extensions, Config};
use crate::session::session_manager::CURRENT_SCHEMA_VERSION;
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

fn is_dev_mode() -> bool {
    cfg!(debug_assertions)
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

#[derive(Default, Clone)]
pub struct ErrorContext {
    pub component: Option<String>,
    pub action: Option<String>,
    pub error_message: Option<String>,
}

pub fn emit_error(error_type: &str, error_message: &str) {
    emit_error_with_context(
        error_type,
        ErrorContext {
            error_message: Some(error_message.to_string()),
            ..Default::default()
        },
    );
}

pub fn emit_error_with_context(error_type: &str, context: ErrorContext) {
    if !is_telemetry_enabled() {
        return;
    }

    let installation = load_or_create_installation();
    let error_type = error_type.to_string();

    tokio::spawn(async move {
        let _ = send_error_event(&installation, &error_type, context).await;
    });
}

pub fn emit_custom_slash_command_used() {
    if !is_telemetry_enabled() {
        return;
    }

    let installation = load_or_create_installation();

    tokio::spawn(async move {
        let _ = send_custom_slash_command_event(&installation).await;
    });
}

async fn send_error_event(
    installation: &InstallationData,
    error_type: &str,
    context: ErrorContext,
) -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new("error", &installation.installation_id);

    event.insert_prop("error_type", error_type).ok();
    event
        .insert_prop("error_category", classify_error(error_type))
        .ok();
    event.insert_prop("source", "backend").ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();
    event.insert_prop("interface", get_session_interface()).ok();
    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();

    if let Some(component) = &context.component {
        event.insert_prop("component", component.as_str()).ok();
    }
    if let Some(action) = &context.action {
        event.insert_prop("action", action.as_str()).ok();
    }
    if let Some(error_message) = &context.error_message {
        let sanitized = sanitize_string(error_message);
        event.insert_prop("error_message", sanitized).ok();
    }

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

async fn send_custom_slash_command_event(installation: &InstallationData) -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event =
        posthog_rs::Event::new("custom_slash_command_used", &installation.installation_id);

    event.insert_prop("source", "backend").ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();
    event.insert_prop("interface", get_session_interface()).ok();
    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();

    if let Some(platform_version) = get_platform_version() {
        event.insert_prop("platform_version", platform_version).ok();
    }

    client.capture(event).await.map_err(|e| format!("{:?}", e))
}

async fn send_session_event(installation: &InstallationData) -> Result<(), String> {
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new("session_started", &installation.installation_id);

    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();
    event.insert_prop("is_dev", is_dev_mode()).ok();

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

    if let Ok(mode) = config.get_param::<String>("GOOSE_MODE") {
        event.insert_prop("setting_mode", mode).ok();
    }
    if let Ok(max_turns) = config.get_param::<i64>("GOOSE_MAX_TURNS") {
        event.insert_prop("setting_max_turns", max_turns).ok();
    }

    if let Ok(lead_model) = config.get_param::<String>("GOOSE_LEAD_MODEL") {
        event.insert_prop("setting_lead_model", lead_model).ok();
    }
    if let Ok(lead_provider) = config.get_param::<String>("GOOSE_LEAD_PROVIDER") {
        event
            .insert_prop("setting_lead_provider", lead_provider)
            .ok();
    }
    if let Ok(lead_turns) = config.get_param::<i64>("GOOSE_LEAD_TURNS") {
        event.insert_prop("setting_lead_turns", lead_turns).ok();
    }
    if let Ok(lead_failure_threshold) = config.get_param::<i64>("GOOSE_LEAD_FAILURE_THRESHOLD") {
        event
            .insert_prop("setting_lead_failure_threshold", lead_failure_threshold)
            .ok();
    }
    if let Ok(lead_fallback_turns) = config.get_param::<i64>("GOOSE_LEAD_FALLBACK_TURNS") {
        event
            .insert_prop("setting_lead_fallback_turns", lead_fallback_turns)
            .ok();
    }

    let extensions = get_enabled_extensions();
    event.insert_prop("extensions_count", extensions.len()).ok();
    let extension_names: Vec<String> = extensions.iter().map(|e| e.name()).collect();
    event.insert_prop("extensions", extension_names).ok();

    event
        .insert_prop("db_schema_version", CURRENT_SCHEMA_VERSION)
        .ok();

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

// ============================================================================
// Error Classification
// ============================================================================
pub fn classify_error(error: &str) -> &'static str {
    let error_lower = error.to_lowercase();

    if error_lower.contains("network") || error_lower.contains("fetch") {
        return "network_error";
    }
    if error_lower.contains("timeout") {
        return "timeout";
    }
    if error_lower.contains("rate") && error_lower.contains("limit") {
        return "rate_limit";
    }
    if error_lower.contains("auth")
        || error_lower.contains("unauthorized")
        || error_lower.contains("401")
    {
        return "auth_error";
    }
    if error_lower.contains("permission") || error_lower.contains("403") {
        return "permission_error";
    }
    if error_lower.contains("not found") || error_lower.contains("404") {
        return "not_found";
    }
    if error_lower.contains("provider") {
        return "provider_error";
    }
    if error_lower.contains("config") {
        return "config_error";
    }
    if error_lower.contains("extension") {
        return "extension_error";
    }
    if error_lower.contains("database") || error_lower.contains("db") || error_lower.contains("sql")
    {
        return "database_error";
    }
    if error_lower.contains("migration") {
        return "migration_error";
    }
    if error_lower.contains("render") || error_lower.contains("react") {
        return "render_error";
    }
    if error_lower.contains("chunk") || error_lower.contains("module") {
        return "module_error";
    }

    "unknown_error"
}

// ============================================================================
// Privacy Sanitization
// ============================================================================

use regex::Regex;
use std::sync::LazyLock;

static SENSITIVE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // File paths with usernames (Unix)
        Regex::new(r"/Users/[^/\s]+").unwrap(),
        Regex::new(r"/home/[^/\s]+").unwrap(),
        // File paths with usernames (Windows)
        Regex::new(r"(?i)C:\\Users\\[^\\\s]+").unwrap(),
        // API keys and tokens (common patterns)
        Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
        Regex::new(r"pk-[a-zA-Z0-9]{20,}").unwrap(),
        Regex::new(r"(?i)key[_-]?[a-zA-Z0-9]{16,}").unwrap(),
        Regex::new(r"(?i)token[_-]?[a-zA-Z0-9]{16,}").unwrap(),
        Regex::new(r"(?i)bearer\s+[a-zA-Z0-9._-]+").unwrap(),
        // Email addresses
        Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        // URLs with auth info
        Regex::new(r"https?://[^:]+:[^@]+@").unwrap(),
        // UUIDs (might be session/user IDs in error messages)
        Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
            .unwrap(),
    ]
});

fn sanitize_string(s: &str) -> String {
    let mut result = s.to_string();
    for pattern in SENSITIVE_PATTERNS.iter() {
        result = pattern.replace_all(&result, "[REDACTED]").to_string();
    }
    result
}

fn sanitize_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(sanitize_string(&s)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sanitize_value).collect())
        }
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, sanitize_value(v)))
                .collect(),
        ),
        other => other,
    }
}

// ============================================================================
// Generic Event API (for frontend)
// ============================================================================
pub async fn emit_event(
    event_name: &str,
    mut properties: std::collections::HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    if !is_telemetry_enabled() {
        return Ok(());
    }

    let installation = load_or_create_installation();
    let client = posthog_rs::client(POSTHOG_API_KEY).await;
    let mut event = posthog_rs::Event::new(event_name, &installation.installation_id);

    event.insert_prop("os", std::env::consts::OS).ok();
    event.insert_prop("arch", std::env::consts::ARCH).ok();
    event.insert_prop("version", env!("CARGO_PKG_VERSION")).ok();
    event.insert_prop("interface", "desktop").ok();
    event.insert_prop("source", "ui").ok();

    if let Some(platform_version) = get_platform_version() {
        event.insert_prop("platform_version", platform_version).ok();
    }

    if event_name == "error_occurred" || event_name == "app_crashed" {
        if let Some(serde_json::Value::String(error_type)) = properties.get("error_type") {
            let classified = classify_error(error_type);
            properties.insert(
                "error_category".to_string(),
                serde_json::Value::String(classified.to_string()),
            );
        }
    }

    for (key, value) in properties {
        let key_lower = key.to_lowercase();
        if key_lower.contains("key")
            || key_lower.contains("token")
            || key_lower.contains("secret")
            || key_lower.contains("password")
            || key_lower.contains("credential")
        {
            continue;
        }
        let sanitized_value = sanitize_value(value);
        event.insert_prop(&key, sanitized_value).ok();
    }

    client.capture(event).await.map_err(|e| format!("{:?}", e))
}
