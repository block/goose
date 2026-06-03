//! Importers for non-goose session formats.
//!
//! Goose's native session export is a JSON-serialized [`crate::session::Session`].
//! These submodules let users also import sessions exported by other coding
//! agents — currently:
//!
//! - **Claude Code** (`.jsonl` files under `~/.claude/projects/...`)
//! - **Codex** (`.jsonl` rollouts under `~/.codex/sessions/YYYY/MM/DD/...`)
//! - **Pi** (`.jsonl` files under `~/.pi/agent/sessions/...`)
//!
//! The strategy is to convert any supported foreign format into goose's
//! native [`Session`] JSON, then hand it off to the existing
//! `SessionManager::import_session` pipeline.

use anyhow::Result;

pub mod claude_code;
pub mod codex;
pub mod pi;

/// Detected import source format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// Native goose session export — a JSON object representing a `Session`.
    Goose,
    /// Claude Code `.jsonl` transcript (one JSON object per line, no header).
    ClaudeCode,
    /// Codex (OpenAI) `.jsonl` rollout file. First line is `{"type":"session_meta",...}`.
    Codex,
    /// Pi-mono `.jsonl` transcript (first line is `{"type":"session",...}` header).
    Pi,
}

/// Sniff the format of an import payload.
///
/// We peek at the first non-blank line:
/// - If it parses as a JSON object whose top-level has `working_dir`/`workingDir`
///   and a `conversation` (or `messages`) field, it's goose.
/// - If the *first* line is `{"type":"session", ...}` it's pi.
/// - If it's a JSON-Lines stream with per-line `type` fields like
///   `user`/`assistant`/`attachment`, it's Claude Code.
pub fn detect_format(content: &str) -> ImportFormat {
    let first_line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(first_line) {
        // Codex rollouts always start with `{"type":"session_meta",...}`.
        if v.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
            return ImportFormat::Codex;
        }
        // Pi sessions start with a `{"type":"session",...}` header. Older
        // fixtures lack `version` but always have `cwd` + `id`.
        if v.get("type").and_then(|t| t.as_str()) == Some("session")
            && (v.get("version").is_some() || (v.get("cwd").is_some() && v.get("id").is_some()))
        {
            return ImportFormat::Pi;
        }
        // Claude Code lines always include a sessionId; goose's native JSON is
        // a single multi-line object whose first *parsed* line is `{` only.
        if v.is_object()
            && v.get("sessionId").is_some()
            && (v.get("type").is_some() || v.get("uuid").is_some())
        {
            return ImportFormat::ClaudeCode;
        }
    }

    // Goose's pretty-printed export starts with `{` and *eventually* contains
    // a full Session object — try to parse the entire payload.
    if serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("working_dir")
                .or_else(|| v.get("workingDir"))
                .cloned()
        })
        .is_some()
    {
        return ImportFormat::Goose;
    }

    // Fallback: if every non-blank line is a JSON object with a `type` and
    // a `sessionId`, treat it as Claude Code.
    let mut saw_claude_marker = false;
    for line in content.lines().filter(|l| !l.trim().is_empty()).take(5) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("sessionId").is_some() {
                saw_claude_marker = true;
            }
        }
    }
    if saw_claude_marker {
        return ImportFormat::ClaudeCode;
    }

    ImportFormat::Goose
}

/// Convert any supported foreign format to a goose-native session JSON string.
///
/// For [`ImportFormat::Goose`] the input is returned unchanged.
pub fn convert_to_goose_session_json(content: &str) -> Result<String> {
    match detect_format(content) {
        ImportFormat::Goose => Ok(content.to_string()),
        ImportFormat::ClaudeCode => claude_code::convert(content),
        ImportFormat::Codex => codex::convert(content),
        ImportFormat::Pi => pi::convert(content),
    }
}

/// Squeeze a string down to a short session-name candidate: take the first
/// non-empty line and cap it at ~80 chars.
pub(crate) fn summarize_first_line(s: &str) -> String {
    let line = s.lines().find(|l| !l.trim().is_empty()).unwrap_or(s).trim();
    if line.chars().count() <= 80 {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(77).collect();
        format!("{}...", truncated)
    }
}
