//! Local access-control state: who may connect, what's revoked, and which
//! single-use invites have already been redeemed.
//!
//! This is deliberately local, unsigned admin state (like a sibling production
//! iroh project's trust store). It is trusted because it lives on the host
//! under the user's
//! control. Authentication of *who* a peer is comes for free from iroh's
//! QUIC-TLS handshake; this layer decides *whether they are authorized*.

use std::collections::HashSet;

use iroh::EndpointId;
use serde::{Deserialize, Serialize};

/// How strictly the host admits inbound connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrustPolicy {
    /// Any peer holding a valid invite may connect (bearer mode).
    #[default]
    Bearer,
    /// Only peers whose key is on the allowlist may connect, in addition to
    /// presenting a valid invite.
    Allowlist,
}

/// Persisted trust state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustBook {
    pub policy: TrustPolicy,
    /// Client keys explicitly permitted under [`TrustPolicy::Allowlist`].
    allowed: HashSet<String>,
    /// Client keys that are refused regardless of invite.
    revoked_keys: HashSet<String>,
    /// Token ids that are refused (revoked or already-redeemed single-use).
    revoked_tokens: HashSet<String>,
}

impl TrustBook {
    pub fn new(policy: TrustPolicy) -> Self {
        Self {
            policy,
            ..Default::default()
        }
    }

    /// Override the admission policy, preserving allow/revoke state. Used when a
    /// persisted book is reloaded and this run's `share` flags set the policy.
    pub fn set_policy(&mut self, policy: TrustPolicy) {
        self.policy = policy;
    }

    pub fn allow(&mut self, key: &EndpointId) {
        self.allowed.insert(key_str(key));
    }

    pub fn revoke_key(&mut self, key: &EndpointId) {
        self.revoked_keys.insert(key_str(key));
    }

    pub fn revoke_token(&mut self, token_id: &str) {
        self.revoked_tokens.insert(token_id.to_string());
    }

    pub fn is_allowed(&self, key: &EndpointId) -> bool {
        match self.policy {
            TrustPolicy::Bearer => true,
            TrustPolicy::Allowlist => self.allowed.contains(&key_str(key)),
        }
    }

    pub fn is_key_revoked(&self, key: &EndpointId) -> bool {
        self.revoked_keys.contains(&key_str(key))
    }

    /// Client keys explicitly allowed (empty under bearer policy), sorted.
    pub fn allowed_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.allowed.iter().cloned().collect();
        keys.sort();
        keys
    }

    /// Client keys that are refused regardless of invite, sorted.
    pub fn revoked_key_list(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.revoked_keys.iter().cloned().collect();
        keys.sort();
        keys
    }

    pub fn is_token_revoked(&self, token_id: &str) -> bool {
        self.revoked_tokens.contains(token_id)
    }

    /// Mark a single-use token redeemed. Returns `false` if it was already
    /// redeemed (i.e. this redemption should be refused).
    pub fn redeem_single_use(&mut self, token_id: &str) -> bool {
        self.revoked_tokens.insert(token_id.to_string())
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
    fn bearer_allows_all() {
        let book = TrustBook::new(TrustPolicy::Bearer);
        assert!(book.is_allowed(&SecretKey::generate().public()));
    }

    #[test]
    fn allowlist_gates() {
        let mut book = TrustBook::new(TrustPolicy::Allowlist);
        let key = SecretKey::generate().public();
        assert!(!book.is_allowed(&key));
        book.allow(&key);
        assert!(book.is_allowed(&key));
    }

    #[test]
    fn single_use_redeems_once() {
        let mut book = TrustBook::default();
        assert!(book.redeem_single_use("tok"));
        assert!(!book.redeem_single_use("tok"));
        assert!(book.is_token_revoked("tok"));
    }

    #[test]
    fn revocation() {
        let mut book = TrustBook::default();
        let key = SecretKey::generate().public();
        book.revoke_key(&key);
        assert!(book.is_key_revoked(&key));
    }

    #[test]
    fn persists_pairing_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trust.json");
        let key = SecretKey::generate().public();

        {
            let mut book = TrustBook::new(TrustPolicy::Allowlist);
            assert!(book.redeem_single_use("tok"));
            book.allow(&key);
            book.save(&path).unwrap();
        }

        let reloaded = TrustBook::load(&path).unwrap();
        // Pinned key survives, and the consumed single-use token stays consumed.
        assert!(reloaded.is_allowed(&key));
        assert!(reloaded.is_token_revoked("tok"));
        assert_eq!(reloaded.allowed_keys(), vec![key.to_string()]);
    }

    #[test]
    fn set_policy_preserves_state() {
        let mut book = TrustBook::new(TrustPolicy::Allowlist);
        let key = SecretKey::generate().public();
        book.allow(&key);
        book.set_policy(TrustPolicy::Bearer);
        // Policy changed, but the pinned key is retained.
        assert_eq!(book.allowed_keys(), vec![key.to_string()]);
    }
}
