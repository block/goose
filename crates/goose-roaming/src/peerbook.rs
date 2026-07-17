//! A user-managed address book of remote agents you can connect to.
//!
//! Unlike [`crate::Directory`] (which records connections that actually
//! happened), the [`PeerBook`] holds saved remotes you *may* connect to,
//! addressed by a friendly nickname. Each entry stores the complete invite
//! token — it is the outbound credential — so it is persisted with restrictive
//! permissions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RoamingError;
use crate::invite::{Scope, SignedInvite};

/// A single saved remote agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    /// Friendly nickname used to `connect <name>`.
    pub name: String,
    /// The complete invite token (the outbound credential).
    pub invite: String,
    /// Cached from the invite for display without re-decoding.
    pub endpoint_id: String,
    pub scope: Scope,
    pub expires_at_ms: u64,
    /// Whether this credential is bearer (no client-key binding).
    pub bearer: bool,
    pub added_ms: u64,
}

/// A persisted map of nickname -> saved remote.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeerBook {
    peers: BTreeMap<String, PeerRecord>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl PeerBook {
    /// Load the peer book from `path`, or start empty if it does not exist.
    /// Mutations are flushed back to `path`.
    pub fn load(path: PathBuf) -> Result<Self, RoamingError> {
        let mut book = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<PeerBook>(&bytes).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => PeerBook::default(),
            Err(e) => return Err(RoamingError::Io(e)),
        };
        book.path = Some(path);
        Ok(book)
    }

    /// Save a remote under `name`, decoding `invite` to cache display fields.
    /// Returns an error if the invite is malformed. Overwrites an existing
    /// entry with the same name (used to refresh an expired credential).
    pub fn save(&mut self, name: &str, invite: &str, now_ms: u64) -> Result<(), RoamingError> {
        let decoded = SignedInvite::decode(invite)?;
        let record = PeerRecord {
            name: name.to_string(),
            invite: invite.to_string(),
            endpoint_id: decoded.claims.audience.to_string(),
            scope: decoded.claims.scope,
            expires_at_ms: decoded.claims.expires_at_ms,
            bearer: decoded.claims.allowed_client_keys.is_empty(),
            added_ms: now_ms,
        };
        self.peers.insert(name.to_string(), record);
        self.flush()
    }

    /// Remove a saved remote. Returns whether it existed.
    pub fn remove(&mut self, name: &str) -> Result<bool, RoamingError> {
        let existed = self.peers.remove(name).is_some();
        if existed {
            self.flush()?;
        }
        Ok(existed)
    }

    /// Rename a saved remote. Returns an error if `from` is missing or `to`
    /// already exists.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), RoamingError> {
        if self.peers.contains_key(to) {
            return Err(RoamingError::Invite(format!("peer `{to}` already exists")));
        }
        let mut record = self
            .peers
            .remove(from)
            .ok_or_else(|| RoamingError::Invite(format!("no peer named `{from}`")))?;
        record.name = to.to_string();
        self.peers.insert(to.to_string(), record);
        self.flush()
    }

    /// Look up a saved remote by nickname.
    pub fn get(&self, name: &str) -> Option<&PeerRecord> {
        self.peers.get(name)
    }

    /// All saved remotes, sorted by nickname.
    pub fn list(&self) -> Vec<&PeerRecord> {
        self.peers.values().collect()
    }

    fn flush(&self) -> Result<(), RoamingError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| RoamingError::Invite(format!("serialize peer book: {e}")))?;
        write_private(path, &json)
    }
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &[u8]) -> Result<(), RoamingError> {
    use std::os::unix::fs::PermissionsExt;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &[u8]) -> Result<(), RoamingError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::RoamingIdentity;
    use crate::invite::InviteClaims;

    fn make_invite(scope: Scope) -> String {
        let host = RoamingIdentity::generate();
        let claims = InviteClaims {
            version: 1,
            audience: host.public_key(),
            relay_urls: vec!["https://relay.example./".into()],
            scope,
            allowed_client_keys: vec![],
            token_id: "tok".into(),
            not_before_ms: 0,
            expires_at_ms: 10_000,
            single_use: false,
        };
        SignedInvite::sign(host.secret_key(), claims)
            .encode()
            .unwrap()
    }

    #[test]
    fn save_get_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut book = PeerBook::load(dir.path().join("peers.json")).unwrap();
        let invite = make_invite(Scope::Control);

        book.save("work", &invite, 1_000).unwrap();
        let rec = book.get("work").unwrap();
        assert_eq!(rec.name, "work");
        assert!(matches!(rec.scope, Scope::Control));
        assert!(rec.bearer);

        assert!(book.remove("work").unwrap());
        assert!(book.get("work").is_none());
        assert!(!book.remove("work").unwrap());
    }

    #[test]
    fn rename_rules() {
        let dir = tempfile::tempdir().unwrap();
        let mut book = PeerBook::load(dir.path().join("peers.json")).unwrap();
        book.save("a", &make_invite(Scope::Observe), 1).unwrap();
        book.save("b", &make_invite(Scope::Observe), 1).unwrap();

        assert!(book.rename("a", "b").is_err()); // target exists
        assert!(book.rename("missing", "c").is_err()); // source missing
        book.rename("a", "c").unwrap();
        assert!(book.get("a").is_none());
        assert_eq!(book.get("c").unwrap().name, "c");
    }

    #[test]
    fn persists_across_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        {
            let mut book = PeerBook::load(path.clone()).unwrap();
            book.save("work", &make_invite(Scope::Control), 1).unwrap();
        }
        let book = PeerBook::load(path).unwrap();
        assert_eq!(book.list().len(), 1);
        assert_eq!(book.get("work").unwrap().name, "work");
    }

    #[test]
    fn rejects_malformed_invite() {
        let dir = tempfile::tempdir().unwrap();
        let mut book = PeerBook::load(dir.path().join("peers.json")).unwrap();
        assert!(book.save("bad", "not-a-token", 1).is_err());
    }
}
