use chrono::{DateTime, Utc};
use goose::session::session_manager::{SessionManager, SessionType};
use serde::Serialize;
use std::path::Path;

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

/// Env var set for the duration of a review run. Subprocesses inherit it
/// and tag hidden sessions as `{prefix}:{label}` via `--name`.
pub const GOOSE_REVIEW_SESSION_PREFIX_ENV: &str = "GOOSE_REVIEW_SESSION_PREFIX";

#[derive(Debug, Clone, Serialize)]
pub struct ReviewUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewSessionEntry {
    pub id: String,
    pub status: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: String,
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
    pub findings_seen: usize,
    pub findings_emitted: usize,
    pub findings_suppressed: usize,
    pub min_severity: String,
    pub usage: ReviewUsage,
    pub sessions: Vec<ReviewSessionEntry>,
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
            findings_seen: 0,
            findings_emitted: 0,
            findings_suppressed: 0,
            min_severity: String::new(),
            usage: ReviewUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
            },
            sessions: Vec::new(),
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

pub async fn load_review_sessions(
    prefix: &str,
    since: DateTime<Utc>,
    working_dir: &Path,
) -> Vec<ReviewSessionEntry> {
    let session_manager = SessionManager::instance();
    let sessions = match session_manager
        .list_sessions_by_types(&[SessionType::Hidden])
        .await
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    // Subprocess sessions are stamped to second precision in SQLite while
    // `since` carries sub-second nanos from Utc::now(); subtract a small
    // buffer so sessions created in the same second as review start match.
    let since_cutoff = since - chrono::Duration::seconds(2);

    let mut out = Vec::new();
    for session in sessions {
        if session.working_dir != working_dir {
            continue;
        }
        if session.created_at < since_cutoff {
            continue;
        }
        let name_matches = session.name.starts_with(prefix);
        let review_scoped = session.name.starts_with("goose-review:");
        let tokens = i64::from(session.accumulated_input_tokens.unwrap_or(0))
            + i64::from(session.accumulated_output_tokens.unwrap_or(0));
        if !name_matches && !(review_scoped || tokens > 0) {
            continue;
        }
        let input = i64::from(session.accumulated_input_tokens.unwrap_or(0));
        let output = i64::from(session.accumulated_output_tokens.unwrap_or(0));
        let model = session
            .model_config
            .as_ref()
            .map(|m| m.model_name.clone())
            .unwrap_or_default();
        out.push(ReviewSessionEntry {
            id: session.id,
            status: "ok".to_string(),
            input_tokens: input,
            output_tokens: output,
            model,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn aggregate_usage(sessions: &[ReviewSessionEntry]) -> ReviewUsage {
    let mut input_tokens = 0i64;
    let mut output_tokens = 0i64;
    for s in sessions {
        input_tokens += s.input_tokens;
        output_tokens += s.output_tokens;
    }
    ReviewUsage {
        input_tokens,
        output_tokens,
        total_tokens: input_tokens + output_tokens,
    }
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
}
