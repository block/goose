use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::Serialize;

use crate::agents::platform_extensions::PLATFORM_EXTENSIONS;
use crate::agents::{Agent, ExtensionConfig};
use crate::config::Config;
use crate::session::{
    EnabledExtensionsState, ExtensionState, Session, SessionManager, SessionType,
};

const CLAW_EXTENSIONS: &[&str] = &["developer", "orchestrator"];

fn platform_ext_config(name: &str) -> Option<ExtensionConfig> {
    let def = PLATFORM_EXTENSIONS.get(name)?;
    Some(ExtensionConfig::Platform {
        name: def.name.to_string(),
        display_name: Some(def.display_name.to_string()),
        description: def.description.to_string(),
        bundled: None,
        available_tools: Vec::new(),
    })
}

pub async fn ensure_session(session_manager: &SessionManager) -> Result<Session> {
    let claw_sessions = session_manager
        .list_sessions_by_types(&[SessionType::Claw])
        .await?;

    if let Some(session) = claw_sessions.into_iter().next() {
        return Ok(session);
    }

    let working_dir = PathBuf::from(
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string()),
    );

    let config = Config::global();
    let current_mode = config.get_goose_mode().unwrap_or_default();

    let session = session_manager
        .create_session(
            working_dir,
            "Active Agent".to_string(),
            SessionType::Claw,
            current_mode,
        )
        .await?;

    let extensions: Vec<_> = CLAW_EXTENSIONS
        .iter()
        .filter_map(|name| platform_ext_config(name))
        .collect();

    let extensions_state = EnabledExtensionsState::new(extensions);
    let mut extension_data = session.extension_data.clone();
    extensions_state.to_extension_data(&mut extension_data)?;

    session_manager
        .update(&session.id)
        .user_provided_name("Active Agent")
        .extension_data(extension_data)
        .apply()
        .await?;

    session_manager.get_session(&session.id, false).await
}

#[derive(Serialize)]
struct SessionInfo {
    id: String,
    name: String,
    updated: String,
}

#[derive(Serialize)]
struct RecentFile {
    path: String,
    modified: String,
}

#[derive(Serialize)]
struct NestFile {
    name: String,
    content: Option<String>,
}

#[derive(Serialize)]
struct ClawContext {
    sessions: Vec<SessionInfo>,
    recent_files: Vec<RecentFile>,
    nest: Vec<NestFile>,
}

async fn gather_recent_sessions(session_manager: &SessionManager) -> Vec<SessionInfo> {
    let sessions = session_manager.list_sessions().await.unwrap_or_default();

    sessions
        .into_iter()
        .take(10)
        .map(|s| SessionInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            updated: s.updated_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect()
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "__pycache__",
    ".git",
    ".hg",
    "Library",
    ".Trash",
    "AppData",
    ".cache",
    ".npm",
    ".cargo",
];

const SKIP_EXTENSIONS: &[&str] = &["pyc", "pyo", "o", "class"];

fn gather_recent_files() -> Vec<RecentFile> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    let root = std::path::Path::new(&home);
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);

    let mut files: Vec<(std::time::SystemTime, String)> = Vec::new();
    walk_dir(root, 0, 4, cutoff, &mut files);
    files.sort_by(|a, b| b.0.cmp(&a.0));

    files
        .into_iter()
        .take(10)
        .map(|(mtime, path)| {
            let modified = mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| {
                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            RecentFile { path, modified }
        })
        .collect()
}

fn walk_dir(
    dir: &std::path::Path,
    depth: u32,
    max_depth: u32,
    cutoff: std::time::SystemTime,
    results: &mut Vec<(std::time::SystemTime, String)>,
) {
    if depth > max_depth {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') && depth > 0 {
            continue;
        }

        let path = entry.path();

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            walk_dir(&path, depth + 1, max_depth, cutoff, results);
        } else if file_type.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SKIP_EXTENSIONS.contains(&ext) {
                    continue;
                }
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if mtime > cutoff {
                        results.push((mtime, path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }
}

fn gather_nest() -> Vec<NestFile> {
    use crate::agents::platform_extensions::orchestrator::{nest_dir, NEST_PATHS};

    let dir = nest_dir();
    let mut files = Vec::new();

    for pattern in NEST_PATHS {
        if let Some(prefix) = pattern.strip_suffix('*') {
            // Directory pattern — list files, include path only (no content).
            let subdir = dir.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&subdir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            files.push(NestFile {
                                name: format!("{}{}", prefix, name),
                                content: None,
                            });
                        }
                    }
                }
            }
        } else {
            // Exact file — include full content.
            let path = dir.join(pattern);
            let content = std::fs::read_to_string(&path).ok();
            if content.is_some() {
                files.push(NestFile {
                    name: pattern.to_string(),
                    content,
                });
            }
        }
    }

    files
}

pub async fn setup_agent(
    agent: &Arc<Agent>,
    session: &Session,
    session_manager: &SessionManager,
) -> Result<()> {
    agent.restore_provider_from_session(session).await?;
    agent.load_extensions_from_session(session).await;

    let sessions = gather_recent_sessions(session_manager).await;
    let recent_files = gather_recent_files();
    let nest = gather_nest();

    let context = ClawContext {
        sessions,
        recent_files,
        nest,
    };

    let prompt = crate::prompt_template::render_template("active_agent.md", &context)
        .unwrap_or_else(|_| {
            "You are an active agent. Proactively share relevant updates.".to_string()
        });
    agent.override_system_prompt(prompt).await;

    Ok(())
}
