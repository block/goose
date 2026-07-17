//! An out-of-band directory of roaming peers.
//!
//! There is deliberately **no gossip**: the directory is built purely from
//! connections this node observes. Inbound connections are recorded when a peer
//! is authorized; outbound connections are recorded when this node dials a
//! remote agent. This gives `goose roam list`-style visibility without any
//! ambient network discovery.

use std::collections::HashMap;
use std::sync::Arc;

use iroh::EndpointId;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::invite::Scope;

/// Which way a connection was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// A remote peer connected to us.
    Inbound,
    /// We connected to a remote peer.
    Outbound,
}

/// A single directory entry describing a peer we have interacted with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEntry {
    pub endpoint_id: String,
    pub label: Option<String>,
    pub direction: Direction,
    pub scope: Scope,
    /// Best-effort agent id reported during the handshake.
    pub agent_id: Option<String>,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    /// Whether a session is currently active with this peer.
    pub connected: bool,
}

/// A shared, in-memory directory of peers.
#[derive(Clone, Default)]
pub struct Directory {
    inner: Arc<Mutex<HashMap<String, PeerEntry>>>,
}

impl Directory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the start of a connection, creating or updating the entry.
    pub async fn record_connect(
        &self,
        endpoint_id: EndpointId,
        label: Option<String>,
        direction: Direction,
        scope: Scope,
        agent_id: Option<String>,
        now_ms: u64,
    ) {
        let key = endpoint_id.to_string();
        let mut map = self.inner.lock().await;
        map.entry(key.clone())
            .and_modify(|e| {
                e.last_seen_ms = now_ms;
                e.connected = true;
                e.scope = scope;
                if label.is_some() {
                    e.label = label.clone();
                }
                if agent_id.is_some() {
                    e.agent_id = agent_id.clone();
                }
            })
            .or_insert(PeerEntry {
                endpoint_id: key,
                label,
                direction,
                scope,
                agent_id,
                first_seen_ms: now_ms,
                last_seen_ms: now_ms,
                connected: true,
            });
    }

    /// Record that a connection with a peer has ended.
    pub async fn record_disconnect(&self, endpoint_id: EndpointId, now_ms: u64) {
        let key = endpoint_id.to_string();
        let mut map = self.inner.lock().await;
        if let Some(entry) = map.get_mut(&key) {
            entry.connected = false;
            entry.last_seen_ms = now_ms;
        }
    }

    /// Snapshot the directory, most-recently-seen first.
    pub async fn list(&self) -> Vec<PeerEntry> {
        let map = self.inner.lock().await;
        let mut entries: Vec<PeerEntry> = map.values().cloned().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.last_seen_ms));
        entries
    }

    /// Number of peers currently connected.
    pub async fn connected_count(&self) -> usize {
        self.inner
            .lock()
            .await
            .values()
            .filter(|e| e.connected)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    #[tokio::test]
    async fn records_and_lists() {
        let dir = Directory::new();
        let peer = SecretKey::generate().public();
        dir.record_connect(
            peer,
            Some("laptop".into()),
            Direction::Inbound,
            Scope::Control,
            Some("agent-1".into()),
            1_000,
        )
        .await;

        let list = dir.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label.as_deref(), Some("laptop"));
        assert!(list[0].connected);
        assert_eq!(dir.connected_count().await, 1);

        dir.record_disconnect(peer, 2_000).await;
        assert_eq!(dir.connected_count().await, 0);
        assert_eq!(dir.list().await[0].last_seen_ms, 2_000);
    }
}
