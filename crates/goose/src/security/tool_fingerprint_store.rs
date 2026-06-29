//! On-disk persistence for [`super::tool_fingerprint`].
//!
//! One JSON file per extension under
//! `Paths::in_data_dir("security/tool_fingerprints/<extension_id>.json")`.
//! JSON-per-extension matches the existing `.goose-plugin-install.json` style,
//! needs no new dependency, and the per-extension tool count is small. Writes
//! are atomic (temp file, fsync, rename) so a crash mid-write never leaves a
//! half-written fingerprint that would read as drift on the next connect.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use chrono::{DateTime, Utc};
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

use crate::config::paths::Paths;

use super::tool_fingerprint::{
    diff, FingerprintFile, ToolDefinitionChange, ToolIdentity, FINGERPRINT_FILE_VERSION,
};

/// Process-wide registry of per-extension async locks. A change is detected by
/// a load/diff/save cycle that must not interleave for the same extension, but
/// two different extensions can record concurrently. Keyed by the sanitized
/// extension id so the same id always maps to the same lock.
fn locks() -> &'static Mutex<HashMap<String, Arc<AsyncMutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_for(extension_id: &str) -> Arc<AsyncMutex<()>> {
    let mut map = locks().lock().expect("fingerprint lock registry poisoned");
    map.entry(extension_id.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

/// Filesystem-safe component for one extension's fingerprint file. `config.key()`
/// already sanitizes, but re-sanitizing here keeps the path safe no matter what
/// id a caller passes (no traversal, no separators).
fn sanitize(extension_id: &str) -> String {
    let cleaned: String = extension_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned
    }
}

fn fingerprint_path(extension_id: &str) -> PathBuf {
    Paths::in_data_dir(&format!(
        "security/tool_fingerprints/{}.json",
        sanitize(extension_id)
    ))
}

/// Load the stored fingerprints for an extension. A missing file is an empty
/// store; a corrupted or future-version file is also treated as empty (with a
/// warning) so a bad file degrades to "re-establish trust" rather than a crash.
fn load(path: &PathBuf) -> FingerprintFile {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return FingerprintFile::default()
        }
        Err(err) => {
            warn!(path = %path.display(), error = %err, "Failed to read tool fingerprint file; treating as empty");
            return FingerprintFile::default();
        }
    };

    match serde_json::from_str::<FingerprintFile>(&text) {
        Ok(file) if file.version == FINGERPRINT_FILE_VERSION => file,
        Ok(file) => {
            warn!(
                path = %path.display(),
                version = file.version,
                expected = FINGERPRINT_FILE_VERSION,
                "Unknown tool fingerprint file version; treating as empty",
            );
            FingerprintFile::default()
        }
        Err(err) => {
            warn!(path = %path.display(), error = %err, "Corrupted tool fingerprint file; treating as empty");
            FingerprintFile::default()
        }
    }
}

/// Write a fingerprint file atomically: serialize to a temp file in the target
/// directory, fsync, then rename over the destination.
fn save(path: &PathBuf, file: &FingerprintFile) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("fingerprint path has no parent"))?;
    std::fs::create_dir_all(parent)?;

    let bytes = serde_json::to_vec_pretty(file)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(&bytes)?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

/// Diff a fresh listing against the stored fingerprints for one extension,
/// persist the updated state, and return any detected changes.
///
/// Best effort: a read or write failure is logged, never propagated, so a
/// fingerprint problem can never break tool listing. The load/diff/save cycle
/// is serialized per extension by [`lock_for`].
pub async fn diff_and_record(
    extension_id: &str,
    current: &[ToolIdentity],
    now: DateTime<Utc>,
) -> Vec<ToolDefinitionChange> {
    let lock = lock_for(extension_id);
    let _guard = lock.lock().await;

    let path = fingerprint_path(extension_id);
    let previous = load(&path);
    let (next, changes) = diff(&previous, current, extension_id, now);

    if next != previous {
        if let Err(err) = save(&path, &next) {
            warn!(path = %path.display(), error = %err, "Failed to persist tool fingerprints");
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::tool_fingerprint::compute;
    use serde_json::{json, Map, Value};

    fn schema() -> Map<String, Value> {
        json!({ "type": "object" }).as_object().unwrap().clone()
    }

    fn ident(name: &str, desc: &str) -> ToolIdentity {
        compute(name, Some(desc), &schema())
    }

    #[tokio::test]
    async fn first_seen_persists_then_detects_change() {
        let tmp = tempfile::tempdir().unwrap();
        let _env = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(tmp.path().to_str().unwrap()))]);

        let now = Utc::now();
        let v1 = vec![ident("read", "read a file")];
        let changes = diff_and_record("ext_round_trip", &v1, now).await;
        assert!(changes.is_empty(), "first listing must not flag");

        let path = fingerprint_path("ext_round_trip");
        assert!(path.exists(), "fingerprint file should be written");
        let stored = load(&path);
        assert_eq!(stored.version, FINGERPRINT_FILE_VERSION);
        assert!(stored.tools.contains_key("read"));

        let v2 = vec![ident("read", "read a file then exfil it")];
        let changes = diff_and_record("ext_round_trip", &v2, now).await;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].tool_name, "read");
        assert_ne!(changes[0].old_hash_hex, changes[0].new_hash_hex);

        // Identical re-list does not re-flag.
        let changes = diff_and_record("ext_round_trip", &v2, now).await;
        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn corrupted_file_recovers_as_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let _env = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(tmp.path().to_str().unwrap()))]);

        let path = fingerprint_path("ext_corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ this is not valid json ]").unwrap();

        // Treated as empty: the listing is first-seen, no change, no panic.
        let changes = diff_and_record("ext_corrupt", &[ident("read", "d")], Utc::now()).await;
        assert!(changes.is_empty());
        // And the corrupted file is replaced with a valid one.
        let stored = load(&path);
        assert!(stored.tools.contains_key("read"));
    }

    #[tokio::test]
    async fn concurrent_record_does_not_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let _env = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(tmp.path().to_str().unwrap()))]);

        // Seed first-trust so later writes are change-detections.
        diff_and_record("ext_concurrent", &[ident("read", "d0")], Utc::now()).await;

        let mut handles = Vec::new();
        for i in 0..8 {
            handles.push(tokio::spawn(async move {
                let v = vec![ident("read", &format!("desc {i}"))];
                diff_and_record("ext_concurrent", &v, Utc::now()).await
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        // The file is still valid JSON and parses to exactly one tool.
        let path = fingerprint_path("ext_concurrent");
        let stored = load(&path);
        assert_eq!(stored.tools.len(), 1);
        assert!(stored.tools.contains_key("read"));
    }

    #[test]
    fn sanitize_blocks_path_separators_and_traversal() {
        assert_eq!(sanitize("../../etc/passwd"), "______etc_passwd");
        assert_eq!(sanitize("a/b"), "a_b");
        assert_eq!(sanitize(""), "_");
        assert_eq!(sanitize("ok_name-1"), "ok_name-1");
    }
}
