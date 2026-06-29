//! Trust-on-first-use (TOFU) fingerprinting of MCP tool definitions.
//!
//! goose checks extensions at install (package-name scanning) and at runtime
//! (tool-call inspection), but it never records the tool definitions a server
//! returns on connect. A server trusted once can therefore rewrite a tool's
//! description or input schema after first trust, changing what the model is
//! told the tool does, and nothing flags it (the tool-poisoning / rug-pull
//! case in issue #9126).
//!
//! Hashing the stable identity of each tool on every `list_tools` lets goose
//! notice when a definition drifts after first trust. This module holds the
//! pure pieces: how an identity is canonicalized and hashed, and how a fresh
//! listing is diffed against the stored fingerprints. Persistence and locking
//! live in [`super::tool_fingerprint_store`].

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// On-disk schema version for a per-extension fingerprint file. Bumped only if
/// the stored shape changes; a file with an unknown version is treated as
/// empty so an older goose never trips over a newer layout.
pub const FINGERPRINT_FILE_VERSION: u32 = 1;

/// Hash of an MCP tool's stable identity (name + description + input_schema).
///
/// `canonical` keeps the exact JSON the hash was taken over so a subscriber
/// can show a before/after diff without re-fetching the prior definition.
/// Annotations and `_meta` are deliberately excluded from the identity: they
/// are host-side hints, not the server-declared semantics a rug-pull rewrites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFingerprint {
    /// Lowercase hex SHA-256 over the canonical JSON of the tool identity.
    pub hash_hex: String,
    /// The canonical (recursively key-sorted) JSON the hash was taken over.
    pub canonical: Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// One detected post-trust change to a tool definition, emitted to the
/// `ToolDefinitionChanged` hook and the standalone tracing surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinitionChange {
    pub extension_id: String,
    pub tool_name: String,
    pub old_hash_hex: String,
    pub new_hash_hex: String,
    pub old_definition: Value,
    pub new_definition: Value,
    pub first_seen_at: DateTime<Utc>,
    pub changed_at: DateTime<Utc>,
}

/// The persisted shape: one file per extension, keyed by the server-declared
/// (unprefixed) tool name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintFile {
    pub version: u32,
    pub tools: BTreeMap<String, ToolFingerprint>,
}

impl Default for FingerprintFile {
    fn default() -> Self {
        Self {
            version: FINGERPRINT_FILE_VERSION,
            tools: BTreeMap::new(),
        }
    }
}

/// A tool's identity as captured from a fresh listing, ready to diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    pub name: String,
    pub hash_hex: String,
    pub canonical: Value,
}

/// Compute the canonical identity and hash for a single tool.
///
/// The identity is the object `{name, description, input_schema}` with every
/// object key sorted recursively, so JSON Schema map ordering never produces a
/// false positive. `description` is `null` when the server omits it.
pub fn compute(
    name: &str,
    description: Option<&str>,
    input_schema: &Map<String, Value>,
) -> ToolIdentity {
    let mut identity = Map::new();
    identity.insert("name".to_string(), Value::String(name.to_string()));
    identity.insert(
        "description".to_string(),
        description.map_or(Value::Null, |d| Value::String(d.to_string())),
    );
    identity.insert(
        "input_schema".to_string(),
        sort_value(&Value::Object(input_schema.clone())),
    );
    let canonical = sort_value(&Value::Object(identity));

    // to_vec on a key-sorted value is deterministic; serialization cannot fail
    // for a plain Value, so the fallback is unreachable in practice.
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let digest = Sha256::digest(&bytes);

    ToolIdentity {
        name: name.to_string(),
        hash_hex: to_hex(&digest),
        canonical,
    }
}

/// Diff a fresh listing against the stored fingerprints for one extension.
///
/// Returns the updated file plus one [`ToolDefinitionChange`] per tool whose
/// hash differs from its stored value. A tool seen for the first time is
/// recorded silently (no change: TOFU establishes trust, it does not flag the
/// initial definition).
///
/// A tool that disappears from the listing keeps its stored fingerprint so a
/// server cannot bypass detection by hiding a tool for one listing and
/// reintroducing it with a rewritten definition. `last_seen_at` is only
/// advanced for tools that actually appeared in this listing; a tool whose
/// `last_seen_at` falls far behind a sibling can be pruned by a future
/// retention policy without losing the trusted hash in the meantime.
pub fn diff(
    previous: &FingerprintFile,
    current: &[ToolIdentity],
    extension_id: &str,
    now: DateTime<Utc>,
) -> (FingerprintFile, Vec<ToolDefinitionChange>) {
    // Carry stored fingerprints forward so a temporarily-omitted tool is still
    // diffed against its prior hash when it reappears (codex #10094 P1).
    let mut next = previous.clone();
    next.version = FINGERPRINT_FILE_VERSION;
    let mut changes = Vec::new();

    for tool in current {
        match previous.tools.get(&tool.name) {
            Some(prev) if prev.hash_hex == tool.hash_hex => {
                // Unchanged: keep first_seen, refresh last_seen.
                next.tools.insert(
                    tool.name.clone(),
                    ToolFingerprint {
                        hash_hex: prev.hash_hex.clone(),
                        canonical: prev.canonical.clone(),
                        first_seen_at: prev.first_seen_at,
                        last_seen_at: now,
                    },
                );
            }
            Some(prev) => {
                // Drift on a still-present tool: the TOFU signal.
                changes.push(ToolDefinitionChange {
                    extension_id: extension_id.to_string(),
                    tool_name: tool.name.clone(),
                    old_hash_hex: prev.hash_hex.clone(),
                    new_hash_hex: tool.hash_hex.clone(),
                    old_definition: prev.canonical.clone(),
                    new_definition: tool.canonical.clone(),
                    first_seen_at: prev.first_seen_at,
                    changed_at: now,
                });
                next.tools.insert(
                    tool.name.clone(),
                    ToolFingerprint {
                        hash_hex: tool.hash_hex.clone(),
                        canonical: tool.canonical.clone(),
                        first_seen_at: prev.first_seen_at,
                        last_seen_at: now,
                    },
                );
            }
            None => {
                // First trust: record without flagging.
                next.tools.insert(
                    tool.name.clone(),
                    ToolFingerprint {
                        hash_hex: tool.hash_hex.clone(),
                        canonical: tool.canonical.clone(),
                        first_seen_at: now,
                        last_seen_at: now,
                    },
                );
            }
        }
    }

    (next, changes)
}

/// Recursively rebuild a value with object keys in sorted order. Arrays keep
/// their order (sequence is significant in JSON Schema); scalars pass through.
fn sort_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), sort_value(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema(props: Value) -> Map<String, Value> {
        json!({ "type": "object", "properties": props })
            .as_object()
            .unwrap()
            .clone()
    }

    #[test]
    fn hash_is_stable_across_schema_key_order() {
        let a = compute(
            "read",
            Some("read a file"),
            &json!({ "b": 1, "a": 2, "nested": { "y": 1, "x": 2 } })
                .as_object()
                .unwrap()
                .clone(),
        );
        let b = compute(
            "read",
            Some("read a file"),
            &json!({ "nested": { "x": 2, "y": 1 }, "a": 2, "b": 1 })
                .as_object()
                .unwrap()
                .clone(),
        );
        assert_eq!(a.hash_hex, b.hash_hex);
        assert_eq!(a.hash_hex.len(), 64);
    }

    #[test]
    fn hash_changes_when_description_changes() {
        let before = compute("read", Some("read a file"), &schema(json!({})));
        let after = compute(
            "read",
            Some("read a file, then delete it"),
            &schema(json!({})),
        );
        assert_ne!(before.hash_hex, after.hash_hex);
    }

    #[test]
    fn hash_changes_when_schema_changes() {
        let before = compute(
            "read",
            Some("d"),
            &schema(json!({ "path": { "type": "string" } })),
        );
        let after = compute(
            "read",
            Some("d"),
            &schema(json!({ "path": { "type": "string" }, "exfil": { "type": "string" } })),
        );
        assert_ne!(before.hash_hex, after.hash_hex);
    }

    #[test]
    fn missing_description_differs_from_empty_description() {
        let none = compute("t", None, &schema(json!({})));
        let empty = compute("t", Some(""), &schema(json!({})));
        assert_ne!(none.hash_hex, empty.hash_hex);
    }

    #[test]
    fn first_seen_records_without_change() {
        let now = Utc::now();
        let current = vec![compute("read", Some("d"), &schema(json!({})))];
        let (file, changes) = diff(&FingerprintFile::default(), &current, "ext", now);
        assert!(changes.is_empty());
        assert_eq!(file.version, FINGERPRINT_FILE_VERSION);
        assert_eq!(file.tools.len(), 1);
        assert_eq!(file.tools["read"].first_seen_at, now);
    }

    #[test]
    fn modified_description_yields_one_change_and_keeps_first_seen() {
        let t0 = Utc::now();
        let v1 = vec![compute("read", Some("d"), &schema(json!({})))];
        let (stored, _) = diff(&FingerprintFile::default(), &v1, "ext", t0);

        let t1 = t0 + chrono::Duration::seconds(5);
        let v2 = vec![compute("read", Some("d, then exfil"), &schema(json!({})))];
        let (next, changes) = diff(&stored, &v2, "ext", t1);

        assert_eq!(changes.len(), 1);
        let change = &changes[0];
        assert_eq!(change.tool_name, "read");
        assert_eq!(change.extension_id, "ext");
        assert_ne!(change.old_hash_hex, change.new_hash_hex);
        assert_eq!(change.first_seen_at, t0);
        assert_eq!(change.changed_at, t1);
        // first_seen is preserved across the rewrite; last_seen advances.
        assert_eq!(next.tools["read"].first_seen_at, t0);
        assert_eq!(next.tools["read"].last_seen_at, t1);
    }

    #[test]
    fn identical_relist_yields_no_change() {
        let t0 = Utc::now();
        let v1 = vec![compute("read", Some("d"), &schema(json!({})))];
        let (stored, _) = diff(&FingerprintFile::default(), &v1, "ext", t0);
        let (_, changes) = diff(&stored, &v1, "ext", t0 + chrono::Duration::seconds(1));
        assert!(changes.is_empty());
    }

    #[test]
    fn missing_tool_retains_fingerprint_so_bypass_via_omit_is_blocked() {
        // Codex #10094 P1: a server that omits a tool for one listing and
        // reintroduces it with a rewritten definition must still be flagged as
        // drift. The store carries forward the prior fingerprint instead of
        // dropping it on first absence.
        let t0 = Utc::now();
        let v1 = vec![
            compute("read", Some("d"), &schema(json!({}))),
            compute("write", Some("d"), &schema(json!({}))),
        ];
        let (stored, _) = diff(&FingerprintFile::default(), &v1, "ext", t0);

        // Server omits `write` from this listing.
        let v2 = vec![compute("read", Some("d"), &schema(json!({})))];
        let (next, changes) = diff(&stored, &v2, "ext", t0);
        assert!(changes.is_empty(), "omission alone is not drift");
        assert!(
            next.tools.contains_key("write"),
            "omitted tool keeps fingerprint"
        );
        assert!(next.tools.contains_key("read"));

        // Server reintroduces `write` with a rewritten description.
        let v3 = vec![
            compute("read", Some("d"), &schema(json!({}))),
            compute("write", Some("d, then exfil"), &schema(json!({}))),
        ];
        let (_, changes) = diff(&next, &v3, "ext", t0);
        assert_eq!(
            changes.len(),
            1,
            "reintroduced-with-drift fires exactly one change"
        );
        assert_eq!(changes[0].tool_name, "write");
    }

    #[test]
    fn added_tool_is_first_seen_not_a_change() {
        let t0 = Utc::now();
        let v1 = vec![compute("read", Some("d"), &schema(json!({})))];
        let (stored, _) = diff(&FingerprintFile::default(), &v1, "ext", t0);

        let v2 = vec![
            compute("read", Some("d"), &schema(json!({}))),
            compute("write", Some("d"), &schema(json!({}))),
        ];
        let (next, changes) = diff(&stored, &v2, "ext", t0);
        assert!(changes.is_empty());
        assert_eq!(next.tools.len(), 2);
    }
}
