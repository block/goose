use chrono::{DateTime, Utc};
use goose::session::session_manager::{SessionManager, SessionType};
use serde::Serialize;

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

/// Machine-readable session id line written to stderr by `goose run
/// --no-session -n goose-review:…` subprocesses.
pub const GOOSE_SESSION_ID_MARKER: &str = "goose-session-id:";

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
    let mut out: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.len() > 120 {
        out.truncate(120);
        while !out.is_empty() && !out.is_char_boundary(out.len()) {
            out.pop();
        }
    }
    if out.is_empty() {
        "session".to_string()
    } else {
        out
    }
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

pub async fn find_hidden_session_id_by_name(name: &str) -> Option<String> {
    let session_manager = SessionManager::instance();
    let sessions = session_manager
        .list_sessions_by_types(&[SessionType::Hidden])
        .await
        .ok()?;
    sessions
        .into_iter()
        .find(|s| s.name == name)
        .map(|s| s.id)
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
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn parse_session_id_from_stderr(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        let line = line.trim();
        if let Some(id) = line.strip_prefix(GOOSE_SESSION_ID_MARKER) {
            let id = id.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

pub fn record_review_session_id(session_ids: &std::sync::Mutex<Vec<String>>, id: Option<String>) {
    let Some(id) = id else { return };
    if let Ok(mut ids) = session_ids.lock() {
        if !ids.iter().any(|existing| existing == &id) {
            ids.push(id);
        }
    }
}

pub async fn record_subprocess_session_id(
    session_ids: &std::sync::Mutex<Vec<String>>,
    session_name: Option<&str>,
    stderr: &str,
) {
    if let Some(id) = parse_session_id_from_stderr(stderr) {
        record_review_session_id(session_ids, Some(id));
        return;
    }
    if let Some(name) = session_name {
        if let Some(id) = find_hidden_session_id_by_name(name).await {
            record_review_session_id(session_ids, Some(id));
        }
    }
}

pub fn aggregate_usage(sessions: &[ReviewSessionEntry]) -> ReviewUsage {
    let mut input_tokens = 0i64;
    let mut output_tokens = 0i64;
    let mut complete = true;
    for s in sessions {
        if s.status != "ok" {
            complete = false;
        }
        match (s.input_tokens, s.output_tokens) {
            (Some(i), Some(o)) => {
                input_tokens += i;
                output_tokens += o;
            }
            _ => complete = false,
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
) -> String {
    let sessions_ok = sessions.iter().all(|s| s.status == "ok");
    if subprocess_failures.is_empty() && sessions_ok && usage.complete {
        return "ok".to_string();
    }
    if subprocess_attempted > 0 && subprocess_failures.len() >= subprocess_attempted {
        return "error".to_string();
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
        assert_eq!(
            sanitize_session_label("check 'security'"),
            "check__security_"
        );
    }

    #[test]
    fn review_session_name_includes_review_id_and_label() {
        assert_eq!(
            review_session_name("20260528_120000_000001", "main:foo.rs").as_deref(),
            Some("goose-review:20260528_120000_000001:main_foo.rs")
        );
        assert!(review_session_name("", "main:foo.rs").is_none());
    }

    #[test]
    fn parse_session_id_from_stderr_reads_marker_line() {
        let stderr = "progress...\ngoose-session-id:20260601_33\n";
        assert_eq!(
            parse_session_id_from_stderr(stderr).as_deref(),
            Some("20260601_33")
        );
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
            }, 2),
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
            }, 2),
            "error"
        );
    }
}
