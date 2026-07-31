//! The self-contained session export envelope: the session itself, the
//! subagent sessions it spawned, and any large tool outputs that were
//! spilled to local files. Plain `Session` JSON deserializes into it, so
//! older exports remain importable.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agents::large_response_handler::{spilled_file_path, SPILL_FILE_PREFIX};
use crate::conversation::message::MessageContentBlock;
use crate::conversation::Conversation;
use crate::session::Session;
use rmcp::model::ContentBlock;

const SUBAGENT_SESSION_META_KEY: &str = "subagent_session_id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    #[serde(flatten)]
    pub session: Session,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_sessions: Vec<Session>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<SessionArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionArtifact {
    pub path: String,
    pub sha256: String,
    pub content: String,
}

pub fn collect_artifacts<'a>(
    sessions: impl Iterator<Item = &'a Session>,
    artifacts_dir: &Path,
    spills_dir: &Path,
) -> Vec<SessionArtifact> {
    let sessions: Vec<&Session> = sessions.collect();
    let session_ids: HashSet<&str> = sessions.iter().map(|session| session.id.as_str()).collect();
    let mut artifacts: BTreeMap<String, SessionArtifact> = BTreeMap::new();
    for session in sessions {
        let Some(conversation) = &session.conversation else {
            continue;
        };
        for path in spilled_paths(conversation) {
            if artifacts.contains_key(&path) {
                continue;
            }
            if !is_exportable_spill_file(Path::new(&path), artifacts_dir, spills_dir, &session_ids)
            {
                tracing::warn!("Skipping non-goose spill pointer in export: {path}");
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let sha256 = sha256_hex(&content);
                    artifacts.insert(
                        path.clone(),
                        SessionArtifact {
                            path,
                            sha256,
                            content,
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!("Skipping unreadable spilled tool output {path}: {e}");
                }
            }
        }
    }
    artifacts.into_values().collect()
}

/// Map each artifact's original path to its content-addressed local copy
/// under `dir`. The hash is recomputed from the content rather than trusted
/// from the import, so a crafted `sha256` field cannot escape `dir`.
pub fn artifact_destinations(
    artifacts: &[SessionArtifact],
    dir: &Path,
) -> BTreeMap<String, String> {
    artifacts
        .iter()
        .map(|artifact| {
            let file = dir.join(format!("{}.txt", sha256_hex(&artifact.content)));
            (artifact.path.clone(), file.to_string_lossy().into_owned())
        })
        .collect()
}

/// Write bundled artifacts to their [`artifact_destinations`] under `dir`.
pub fn write_artifacts(artifacts: &[SessionArtifact], dir: &Path) -> Result<()> {
    if artifacts.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    for artifact in artifacts {
        let file = dir.join(format!("{}.txt", sha256_hex(&artifact.content)));
        let up_to_date =
            std::fs::read_to_string(&file).is_ok_and(|existing| existing == artifact.content);
        if !up_to_date {
            let mut temp = tempfile::NamedTempFile::new_in(dir)?;
            std::io::Write::write_all(&mut temp, artifact.content.as_bytes())?;
            temp.persist(&file)?;
        }
    }
    Ok(())
}

/// Point `_meta.subagent_session_id` references at the freshly created child
/// sessions after an import assigns new ids.
pub fn rewrite_subagent_session_ids(
    conversation: Conversation,
    id_map: &HashMap<String, String>,
) -> Conversation {
    if id_map.is_empty() {
        return conversation;
    }
    let messages = conversation.into_iter().map(|mut message| {
        for content in &mut message.content {
            let MessageContentBlock::ToolResponse(response) = content else {
                continue;
            };
            let Ok(result) = &mut response.tool_result else {
                continue;
            };
            let Some(meta) = result.meta.as_mut() else {
                continue;
            };
            let Some(serde_json::Value::String(id)) = meta.0.get_mut(SUBAGENT_SESSION_META_KEY)
            else {
                continue;
            };
            if let Some(new_id) = id_map.get(id.as_str()) {
                *id = new_id.clone();
            }
        }
        message
    });
    Conversation::new_unvalidated(messages)
}

/// Only bundle files goose itself wrote for the sessions being exported:
/// spill files inside those sessions' own spill directories, or previously
/// imported artifacts. A crafted pointer in a tool response must not be able
/// to pull arbitrary local files, including another session's spills, into an
/// export.
fn is_exportable_spill_file(
    path: &Path,
    artifacts_dir: &Path,
    spills_dir: &Path,
    session_ids: &HashSet<&str>,
) -> bool {
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    let Some(name) = canonical.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if !name.ends_with(".txt") {
        return false;
    }
    let within = |dir: &Path| {
        dir.canonicalize()
            .is_ok_and(|dir| canonical.starts_with(&dir))
    };
    let in_own_spill_dir = name.starts_with(SPILL_FILE_PREFIX)
        && canonical
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|dir_name| dir_name.to_str())
            .is_some_and(|dir_name| session_ids.contains(dir_name))
        && within(spills_dir);
    in_own_spill_dir || (is_sha256_file_name(name) && within(artifacts_dir))
}

/// File names of imported artifacts under `artifacts_dir` that a message's
/// content blocks point at, used to garbage-collect artifact files once no
/// message references them.
pub(crate) fn artifact_file_names(content_json: &str, artifacts_dir: &Path) -> Vec<String> {
    let Ok(blocks) = serde_json::from_str::<Vec<MessageContentBlock>>(content_json) else {
        return Vec::new();
    };
    blocks
        .iter()
        .filter_map(|content| match content {
            MessageContentBlock::ToolResponse(response) => response.tool_result.as_ref().ok(),
            _ => None,
        })
        .flat_map(|result| &result.content)
        .filter_map(|block| spilled_file_path(&block.as_text()?.text))
        .filter_map(|path| {
            let path = Path::new(path);
            let name = path.file_name()?.to_str()?;
            (path.parent() == Some(artifacts_dir) && is_sha256_file_name(name))
                .then(|| name.to_string())
        })
        .collect()
}

fn is_sha256_file_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 68 && bytes[..64].iter().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(content: &str) -> String {
    Sha256::digest(content.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn rewrite_spilled_paths(
    conversation: Conversation,
    paths: &BTreeMap<String, String>,
) -> Conversation {
    if paths.is_empty() {
        return conversation;
    }
    let messages = conversation.into_iter().map(|mut message| {
        for content in &mut message.content {
            let MessageContentBlock::ToolResponse(response) = content else {
                continue;
            };
            let Ok(result) = &mut response.tool_result else {
                continue;
            };
            for block in &mut result.content {
                let Some(replaced) = block.as_text().and_then(|text| {
                    let old_path = spilled_file_path(&text.text)?;
                    let new_path = paths.get(old_path)?;
                    Some(text.text.replace(old_path, new_path))
                }) else {
                    continue;
                };
                *block = ContentBlock::text(replaced);
            }
        }
        message
    });
    Conversation::new_unvalidated(messages)
}

pub(crate) fn spilled_paths(conversation: &Conversation) -> Vec<String> {
    conversation
        .messages()
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContentBlock::ToolResponse(response) => response.tool_result.as_ref().ok(),
            _ => None,
        })
        .flat_map(|result| &result.content)
        .filter_map(|block| spilled_file_path(&block.as_text()?.text).map(|path| path.to_string()))
        .collect()
}
