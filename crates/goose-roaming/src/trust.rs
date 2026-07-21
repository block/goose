//! Local access-control state: which peer keys this node accepts inbound
//! connections from, what each may do, and which keys are revoked.
//!
//! Trust is a **mutual, public-key allowlist**. A peer is identified by the key
//! iroh's QUIC-TLS handshake authenticated, and is admitted only if that key is
//! on this node's allowlist. There is no bearer/token mode: sharing a
//! [`crate::ConnectionCard`] grants nothing until the recipient explicitly
//! accepts the sender's key.
//!
//! This is deliberately local, unsigned admin state: it lives on the host under
//! the user's control. Authentication of *who* a peer is comes from the
//! transport; this layer decides *whether* they are authorized and with what
//! [`Scope`].

use std::collections::{BTreeMap, HashSet};

use iroh::EndpointId;
use serde::{Deserialize, Serialize};

use crate::scope::Scope;

/// Persisted trust state: the inbound allowlist (key -> granted scope) plus
/// revocations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustBook {
    /// Peer keys allowed to connect, each mapped to the scope they are granted.
    allowed: BTreeMap<String, Scope>,
    /// Peer keys that are refused regardless of anything else.
    revoked_keys: HashSet<String>,
}

impl TrustBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept inbound connections from `key`, granting `scope`. Re-accepting an
    /// already-allowed key updates its scope.
    pub fn accept(&mut self, key: &EndpointId, scope: Scope) {
        self.allowed.insert(key_str(key), scope);
    }

    /// Stop accepting `key` and record it as revoked so it cannot be re-added by
    /// a stale card automatically.
    pub fn revoke_key(&mut self, key: &EndpointId) {
        let s = key_str(key);
        self.allowed.remove(&s);
        self.revoked_keys.insert(s);
    }

    /// Whether `key` is allowed to connect (on the allowlist and not revoked).
    pub fn is_allowed(&self, key: &EndpointId) -> bool {
        let s = key_str(key);
        !self.revoked_keys.contains(&s) && self.allowed.contains_key(&s)
    }

    /// The scope granted to `key`, if it is allowed.
    pub fn scope_for(&self, key: &EndpointId) -> Option<Scope> {
        let s = key_str(key);
        if self.revoked_keys.contains(&s) {
            return None;
        }
        self.allowed.get(&s).copied()
    }

    pub fn is_key_revoked(&self, key: &EndpointId) -> bool {
        self.revoked_keys.contains(&key_str(key))
    }

    /// Allowed peer keys with their granted scope, sorted by key.
    pub fn allowed_keys(&self) -> Vec<(String, Scope)> {
        self.allowed.iter().map(|(k, s)| (k.clone(), *s)).collect()
    }

    /// Revoked peer keys, sorted.
    pub fn revoked_key_list(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.revoked_keys.iter().cloned().collect();
        keys.sort();
        keys
    }

    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

fn key_str(key: &EndpointId) -> String {
    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    #[test]
    fn allowlist_gates_and_carries_scope() {
        let mut book = TrustBook::new();
        let key = SecretKey::generate().public();
        assert!(!book.is_allowed(&key));
        assert_eq!(book.scope_for(&key), None);

        book.accept(&key, Scope::Observe);
        assert!(book.is_allowed(&key));
        assert_eq!(book.scope_for(&key), Some(Scope::Observe));

        // Re-accepting updates the scope.
        book.accept(&key, Scope::Control);
        assert_eq!(book.scope_for(&key), Some(Scope::Control));
    }

    #[test]
    fn revocation_removes_and_blocks() {
        let mut book = TrustBook::new();
        let key = SecretKey::generate().public();
        book.accept(&key, Scope::Control);
        book.revoke_key(&key);
        assert!(!book.is_allowed(&key));
        assert!(book.is_key_revoked(&key));
        assert_eq!(book.scope_for(&key), None);
    }

    #[test]
    fn persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trust.json");
        let key = SecretKey::generate().public();
        {
            let mut book = TrustBook::new();
            book.accept(&key, Scope::Attach);
            book.save(&path).unwrap();
        }
        let reloaded = TrustBook::load(&path).unwrap();
        assert_eq!(reloaded.scope_for(&key), Some(Scope::Attach));
    }
}
