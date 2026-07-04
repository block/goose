//! Seam for feeding retrieved memory into the Context Manager.
//!
//! The `RetrievedMemory` slot in a [`super::packet::ContextPacket`] is fed
//! from a [`MemorySource`]. Two implementations ship today:
//! [`NoopMemorySource`] (recalls nothing) and [`FileMemorySource`], which
//! recalls entries from a local JSONL file ranked by keyword overlap with
//! the trailing user message. Both are fully internal — no external
//! services, no MCP.

use std::path::PathBuf;

use rmcp::model::Role;
use serde::Deserialize;

use crate::config::paths::Paths;
use crate::config::Config;
use crate::conversation::message::Message;

/// One unit of recalled context, ready to be rendered into the packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItem {
    pub content: String,
    /// Where this memory came from (e.g. "session:abc123", "note"), shown
    /// alongside the content so the model can weigh its provenance.
    pub source: String,
}

/// What a memory backend gets to look at when deciding what to recall.
#[derive(Debug, Clone)]
pub struct MemoryQuery<'a> {
    pub session_id: &'a str,
    /// The conversation about to be sent, most recent last. Backends will
    /// typically key retrieval off the trailing user message.
    pub messages: &'a [Message],
    /// Hard token ceiling for the slot; returning more than fits is fine —
    /// the Context Manager enforces the budget and records the overflow.
    pub reserved_tokens: usize,
}

/// A source of retrieved memory. Implementations must be cheap and
/// infallible from the caller's perspective: this runs on the hot path in
/// front of every provider call, so blocking I/O or fallible lookups belong
/// behind caching inside the implementation, not in the signature.
pub trait MemorySource: Send + Sync {
    fn retrieve(&self, query: &MemoryQuery<'_>) -> Vec<MemoryItem>;
}

/// Default source: recalls nothing. Keeps the `RetrievedMemory` slot empty
/// until a real backend lands.
pub struct NoopMemorySource;

impl MemorySource for NoopMemorySource {
    fn retrieve(&self, _query: &MemoryQuery<'_>) -> Vec<MemoryItem> {
        Vec::new()
    }
}

/// Cap on recalled items per call, applied before the packet's token budget.
/// Keeps a huge memory file from turning every retrieval into a large sort
/// result the packet then has to throw away.
const MAX_RECALLED_ITEMS: usize = 16;

/// Only words at least this long participate in keyword matching. Four
/// filters the highest-frequency stopwords ("the", "and", "did", "for")
/// without a stopword list; occasional false matches on words like "what"
/// only cost budgeted slot space, never correctness.
const MIN_KEYWORD_LEN: usize = 4;

#[derive(Deserialize)]
struct StoredMemory {
    content: String,
    #[serde(default = "default_memory_source_label")]
    source: String,
}

fn default_memory_source_label() -> String {
    "memory".to_string()
}

/// File-backed memory: recalls entries from a JSONL file, one object per
/// line (`{"content": "...", "source": "..."}`, `source` optional), ranked
/// by keyword overlap with the trailing user message. The file is meant to
/// be human- or agent-maintained, like a `.goslinghints` for durable facts.
///
/// Deliberately forgiving: a missing file recalls nothing (so this is safe
/// to wire in by default), malformed lines are skipped, and the file is
/// small enough that a synchronous read per provider call is acceptable at
/// MVP scale.
pub struct FileMemorySource {
    path: PathBuf,
}

impl FileMemorySource {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Resolves the memory file from `GOSLING_MEMORY_FILE` (env or config),
    /// falling back to `memories.jsonl` in the gosling config dir.
    pub fn from_config() -> Self {
        let path = Config::global()
            .get_param::<String>("GOSLING_MEMORY_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Paths::in_config_dir("memories.jsonl"));
        Self::new(path)
    }

    fn load(&self) -> Vec<StoredMemory> {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<StoredMemory>(line).ok())
            .collect()
    }
}

fn keywords(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= MIN_KEYWORD_LEN)
        .map(str::to_string)
        .collect()
}

/// The text of the most recent user message that actually contains text —
/// tool responses share the user role but carry no `Text` content, so they
/// fall through to the real user turn before them.
fn trailing_user_text(messages: &[Message]) -> Option<String> {
    messages.iter().rev().find_map(|m| {
        if m.role != Role::User {
            return None;
        }
        let text: Vec<&str> = m.content.iter().filter_map(|c| c.as_text()).collect();
        if text.is_empty() {
            None
        } else {
            Some(text.join("\n"))
        }
    })
}

impl MemorySource for FileMemorySource {
    fn retrieve(&self, query: &MemoryQuery<'_>) -> Vec<MemoryItem> {
        let Some(user_text) = trailing_user_text(query.messages) else {
            return Vec::new();
        };
        let query_words = keywords(&user_text);
        if query_words.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(usize, MemoryItem)> = self
            .load()
            .into_iter()
            .filter_map(|stored| {
                let overlap = keywords(&stored.content).intersection(&query_words).count();
                if overlap == 0 {
                    return None;
                }
                Some((
                    overlap,
                    MemoryItem {
                        content: stored.content,
                        source: stored.source,
                    },
                ))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored
            .into_iter()
            .take(MAX_RECALLED_ITEMS)
            .map(|(_, item)| item)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn query_messages(user_text: &str) -> Vec<Message> {
        vec![
            Message::user().with_text("earlier turn"),
            Message::assistant().with_text("earlier reply"),
            Message::user().with_text(user_text),
        ]
    }

    fn write_memory_file(dir: &tempfile::TempDir, lines: &[&str]) -> PathBuf {
        let path = dir.path().join("memories.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn noop_source_recalls_nothing() {
        let query = MemoryQuery {
            session_id: "test",
            messages: &[],
            reserved_tokens: 1_000,
        };
        assert!(NoopMemorySource.retrieve(&query).is_empty());
    }

    #[test]
    fn file_source_recalls_by_keyword_overlap() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_memory_file(
            &dir,
            &[
                r#"{"content": "The database schema uses UUIDv7 primary keys", "source": "session:a"}"#,
                r#"{"content": "User prefers dark mode in the desktop app"}"#,
                r#"{"content": "Deploys go through the staging environment first", "source": "note"}"#,
            ],
        );
        let source = FileMemorySource::new(path);

        let messages = query_messages("what did we decide about the database schema?");
        let recalled = source.retrieve(&MemoryQuery {
            session_id: "test",
            messages: &messages,
            reserved_tokens: 1_000,
        });

        assert_eq!(recalled.len(), 1, "only the schema memory overlaps");
        assert!(recalled[0].content.contains("UUIDv7"));
        assert_eq!(recalled[0].source, "session:a");
    }

    #[test]
    fn file_source_ranks_stronger_overlap_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_memory_file(
            &dir,
            &[
                r#"{"content": "staging deploys run nightly"}"#,
                r#"{"content": "staging database deploys need schema review"}"#,
            ],
        );
        let source = FileMemorySource::new(path);

        let messages = query_messages("how do staging database deploys handle the schema?");
        let recalled = source.retrieve(&MemoryQuery {
            session_id: "test",
            messages: &messages,
            reserved_tokens: 1_000,
        });

        assert_eq!(recalled.len(), 2);
        assert!(
            recalled[0].content.contains("schema review"),
            "the memory sharing more keywords should rank first"
        );
    }

    #[test]
    fn file_source_missing_file_and_bad_lines_are_harmless() {
        let dir = tempfile::tempdir().unwrap();
        let missing = FileMemorySource::new(dir.path().join("nope.jsonl"));
        let messages = query_messages("anything at all here");
        let query = MemoryQuery {
            session_id: "test",
            messages: &messages,
            reserved_tokens: 1_000,
        };
        assert!(missing.retrieve(&query).is_empty());

        let path = write_memory_file(
            &dir,
            &[
                "not json at all",
                r#"{"wrong_field": true}"#,
                r#"{"content": "valid memory about anything"}"#,
            ],
        );
        let source = FileMemorySource::new(path);
        let recalled = source.retrieve(&query);
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].source, "memory", "default source label applies");
    }

    #[test]
    fn file_source_returns_nothing_without_user_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_memory_file(&dir, &[r#"{"content": "some stored fact"}"#]);
        let source = FileMemorySource::new(path);

        let messages = vec![Message::assistant().with_text("assistant only")];
        let recalled = source.retrieve(&MemoryQuery {
            session_id: "test",
            messages: &messages,
            reserved_tokens: 1_000,
        });
        assert!(recalled.is_empty());
    }
}
