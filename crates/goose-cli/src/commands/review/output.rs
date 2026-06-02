use chrono::{DateTime, Utc};
use goose::session::session_manager::{SessionManager, SessionType};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use anyhow::Result;

use super::orchestrator::Finding;

/// How `goose review` formats stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewOutputFormat {
    #[default]
    Jsonl,
    Json,
}

impl ReviewOutputFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "jsonl" => Ok(Self::Jsonl),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unknown review output format {other:?}; expected jsonl or json"
            )),
        }
    }
}

/// Prefix for hidden session names passed to review subprocesses via `-n`.
pub const GOOSE_REVIEW_SESSION_NAME_PREFIX: &str = "goose-review:";

#[derive(Debug, Clone, Serialize)]
pub struct ReviewUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewSessionEntry {
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubprocessFailure {
    pub label: String,
    pub reason: String,
}

/// Top-level stdout document for `goose review --format json`.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewResultDocument {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub version: u32,
    pub status: String,
    pub review_id: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub range: Option<String>,
    pub working_dir: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub checks_discovered: usize,
    pub checks_run: usize,
    pub checks_failed: usize,
    pub findings_seen: usize,
    pub findings_emitted: usize,
    pub findings_suppressed: usize,
    pub min_severity: String,
    pub usage: ReviewUsage,
    pub sessions: Vec<ReviewSessionEntry>,
    pub subprocess_failures: Vec<SubprocessFailure>,
    pub findings: Vec<Finding>,
}

impl ReviewResultDocument {
    pub fn new(review_id: String, started_at: DateTime<Utc>) -> Self {
        Self {
            doc_type: "goose_review_result".to_string(),
            version: 1,
            status: "ok".to_string(),
            review_id,
            started_at: started_at.to_rfc3339(),
            finished_at: String::new(),
            duration_ms: 0,
            range: None,
            working_dir: String::new(),
            provider: None,
            model: None,
            checks_discovered: 0,
            checks_run: 0,
            checks_failed: 0,
            findings_seen: 0,
            findings_emitted: 0,
            findings_suppressed: 0,
            min_severity: String::new(),
            usage: ReviewUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                complete: true,
            },
            sessions: Vec::new(),
            subprocess_failures: Vec::new(),
            findings: Vec::new(),
        }
    }
}

pub fn sanitize_session_label(label: &str) -> String {
    let mut sanitized: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        sanitized = "session".to_string();
    }
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let suffix = format!("_{:08x}", hasher.finish() as u32);
    let max_base = 120usize.saturating_sub(suffix.len());
    if sanitized.len() > max_base {
        sanitized.truncate(max_base);
        while !sanitized.is_empty() && !sanitized.is_char_boundary(sanitized.len()) {
            sanitized.pop();
        }
    }
    format!("{sanitized}{suffix}")
}

pub fn review_session_name(review_id: &str, label: &str) -> Option<String> {
    if review_id.is_empty() {
        return None;
    }
    Some(format!(
        "{GOOSE_REVIEW_SESSION_NAME_PREFIX}{review_id}:{}",
        sanitize_session_label(label)
    ))
}

async fn hidden_sessions_by_name() -> Result<HashMap<String, String>> {
    let session_manager = SessionManager::instance();
    let sessions = session_manager
        .list_sessions_by_types(&[SessionType::Hidden])
        .await?;
    Ok(sessions
        .into_iter()
        .map(|s| (s.name, s.id))
        .collect())
}

/// Resolve hidden review subprocess session names to ids after orchestration.
pub async fn resolve_review_session_ids(names: &[String]) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }

    let mut by_name = hidden_sessions_by_name().await.unwrap_or_else(|e| {
        eprintln!("goose review: failed to list hidden sessions: {e}");
        HashMap::new()
    });

    let mut ids = Vec::with_capacity(names.len());
    let mut missing = Vec::new();
    for name in names {
        if let Some(id) = by_name.get(name) {
            ids.push(id.clone());
        } else {
            missing.push(name.clone());
        }
    }

    if missing.is_empty() {
        return ids;
    }

    for attempt in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        by_name = hidden_sessions_by_name().await.unwrap_or(by_name);
        missing.retain(|name| {
            if let Some(id) = by_name.get(name) {
                ids.push(id.clone());
                false
            } else {
                true
            }
        });
        if missing.is_empty() {
            break;
        }
        if attempt + 1 >= 20 {
            for name in &missing {
                eprintln!("goose review: session not found for subprocess name {name}");
            }
        }
    }

    ids
}

pub fn record_subprocess_session_name(
    session_names: &std::sync::Mutex<Vec<String>>,
    name: Option<String>,
) {
    let Some(name) = name else { return };
    if let Ok(mut names) = session_names.lock() {
        if !names.iter().any(|existing| existing == &name) {
            names.push(name);
        }
    }
}

pub async fn load_review_sessions_by_ids(ids: &[String]) -> Vec<ReviewSessionEntry> {
    if ids.is_empty() {
        return Vec::new();
    }

    let session_manager = SessionManager::instance();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let session = match session_manager.get_session(id, false).await {
            Ok(s) => s,
            Err(e) => {
                out.push(ReviewSessionEntry {
                    id: id.clone(),
                    status: "missing".to_string(),
                    error: Some(e.to_string()),
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
                    provider: None,
                });
                continue;
            }
        };
        let input = session
            .accumulated_input_tokens
            .or(session.input_tokens)
            .map(i64::from);
        let output = session
            .accumulated_output_tokens
            .or(session.output_tokens)
            .map(i64::from);
        let model = session
            .model_config
            .as_ref()
            .map(|m| m.model_name.clone());
        let provider = session.provider_name.clone();
        let status = if input.is_some() && output.is_some() {
            "ok".to_string()
        } else {
            "incomplete".to_string()
        };
        out.push(ReviewSessionEntry {
            id: session.id,
            status,
            error: None,
            input_tokens: input,
            output_tokens: output,
            model,
            provider,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn aggregate_usage(sessions: &[ReviewSessionEntry], sessions_expected: usize) -> ReviewUsage {
    let mut input_tokens = 0i64;
    let mut output_tokens = 0i64;
    let mut complete = true;
    if sessions_expected > 0 && sessions.len() < sessions_expected {
        complete = false;
    }
    for s in sessions {
        if s.status != "ok" {
            complete = false;
        }
        match s.input_tokens {
            Some(i) => input_tokens += i,
            None => complete = false,
        }
        match s.output_tokens {
            Some(o) => output_tokens += o,
            None => complete = false,
        }
    }
    ReviewUsage {
        input_tokens,
        output_tokens,
        total_tokens: input_tokens + output_tokens,
        complete,
    }
}

pub fn review_document_status(
    subprocess_failures: &[SubprocessFailure],
    sessions: &[ReviewSessionEntry],
    usage: &ReviewUsage,
    subprocess_attempted: usize,
    sessions_expected: usize,
) -> String {
    if subprocess_attempted > 0 && subprocess_failures.len() >= subprocess_attempted {
        return "error".to_string();
    }
    if sessions_expected > 0 && sessions.len() < sessions_expected {
        return "partial".to_string();
    }
    let sessions_ok = sessions.iter().all(|s| s.status == "ok");
    if subprocess_failures.is_empty() && sessions_ok && usage.complete {
        return "ok".to_string();
    }
    "partial".to_string()
}

pub fn emit_review_json(doc: &ReviewResultDocument) {
    match serde_json::to_string(doc) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("goose review: failed to serialize JSON output: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_session_label_replaces_spaces() {
        let sanitized = sanitize_session_label("check 'security'");
        assert!(sanitized.starts_with("check__security_"));
        assert!(sanitized.len() <= 120);
    }

    #[test]
    fn sanitize_session_label_distinct_after_truncation() {
        let long_a = format!("main:{}", "a".repeat(200));
        let long_b = format!("main:{}", "b".repeat(200));
        assert_ne!(sanitize_session_label(&long_a), sanitize_session_label(&long_b));
    }

    #[test]
    fn review_session_name_includes_review_id_and_label() {
        let name = review_session_name("20260528_120000_000001", "main:foo.rs").unwrap();
        assert!(name.starts_with("goose-review:20260528_120000_000001:main_foo.rs_"));
        assert!(review_session_name("", "main:foo.rs").is_none());
    }

    #[test]
    fn review_output_format_parse() {
        assert_eq!(
            ReviewOutputFormat::parse("jsonl").unwrap(),
            ReviewOutputFormat::Jsonl
        );
        assert_eq!(
            ReviewOutputFormat::parse("json").unwrap(),
            ReviewOutputFormat::Json
        );
        assert!(ReviewOutputFormat::parse("yaml").is_err());
    }

    #[test]
    fn review_document_status_partial_on_failures() {
        let failures = vec![SubprocessFailure {
            label: "check 'security'".to_string(),
            reason: "timeout".to_string(),
        }];
        assert_eq!(
            review_document_status(&failures, &[], &ReviewUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                complete: false,
            }, 2, 0),
            "partial"
        );
    }

    #[test]
    fn review_document_status_partial_when_sessions_missing() {
        let sessions = vec![ReviewSessionEntry {
            id: "a".to_string(),
            status: "ok".to_string(),
            error: None,
            input_tokens: Some(1),
            output_tokens: Some(1),
            model: None,
            provider: None,
        }];
        assert_eq!(
            review_document_status(&[], &sessions, &ReviewUsage {
                input_tokens: 1,
                output_tokens: 1,
                total_tokens: 2,
                complete: false,
            }, 3, 3),
            "partial"
        );
    }

    #[test]
    fn review_document_status_error_when_all_subprocesses_fail() {
        let failures = vec![
            SubprocessFailure {
                label: "a".to_string(),
                reason: "timeout".to_string(),
            },
            SubprocessFailure {
                label: "b".to_string(),
                reason: "exit".to_string(),
            },
        ];
        assert_eq!(
            review_document_status(&failures, &[], &ReviewUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                complete: false,
            }, 2, 0),
            "error"
        );
    }
}
