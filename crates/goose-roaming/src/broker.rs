//! Multi-client session broker (design skeleton).
//!
//! # Why a broker
//!
//! A goose agent speaks ACP to exactly one client: `serve(agent, stream)` owns a
//! single `ConnectionTo<Client>` and sends `session/update` notifications and
//! `session/request_permission` requests straight to it. To let several remote
//! peers watch and drive one live session (e.g. a phone attaching to a session
//! running on a laptop), we don't make the agent multi-client — we put a broker
//! in front, exactly like `paseo`'s daemon does with its `AgentManager`.
//!
//! The broker is the *single* ACP client to the local agent, and re-serves ACP
//! to N roaming peers. It applies three routing rules:
//!
//! | ACP message                     | direction        | rule                          |
//! |---------------------------------|------------------|-------------------------------|
//! | `session/update` (notification) | agent → peers    | **fan out** to all subscribers|
//! | `session/prompt`, `.../steer`   | peer → agent     | **funnel** (serialized)       |
//! | `session/request_permission`,   | agent → peer     | **route to the controller**   |
//! | fs/terminal requests            |                  | only (never a passive watcher)|
//!
//! This module holds the transport-neutral core — the subscriber set, the
//! controller slot, and the routing decisions — with no dependency on the ACP
//! crate or on iroh, so the policy is unit-testable in isolation. The actual
//! ACP wire plumbing (being a client to the agent, a server to peers) is a thin
//! adapter layered on top; see [`SessionBroker`]'s method docs for the seams
//! that adapter drives.
//!
//! # Status
//!
//! Routing policy ([`Router`]) is implemented and tested, and multi-client
//! fan-out is proven over real iroh transport (see the crate's `end_to_end`
//! test `multiple_clients_share_one_session`). The remaining work is the ACP
//! wire adapter, planned below.
//!
//! # Wire adapter plan (the "act as remote ACP" layer)
//!
//! The broker lives entirely on top of goose — goose core is untouched; from
//! goose's view the broker is just another ACP client over a byte stream. The
//! adapter has two halves:
//!
//! **Agent-facing half — one ACP client to the live local agent.** Runs
//! `agent_client_protocol::Client` over an in-memory duplex whose other end is
//! goose's `acp::server::serve(agent, ..)`. Starts one session (`session/new`,
//! or `session/load` to attach to an existing one), then owns it for the
//! session's life. It:
//!   * receives `session/update` notifications and pushes each into the
//!     broadcast channel (fan-out source),
//!   * submits prompts/steer that peers funnel in,
//!   * answers `session/request_permission` by forwarding to the current
//!     controller peer and relaying the reply (or a safe default via
//!     [`Route::Drop`]).
//!
//! **Peer-facing half — an ACP agent façade per accepted iroh stream.** Each
//! roaming peer runs `roam connect` (an ACP *client*), so the broker must answer
//! `initialize` and `session/*` on that stream. It:
//!   * replies to `initialize` with the shared agent's capabilities,
//!   * on `session/new`|`session/load`, attaches the peer via
//!     [`SessionBroker::attach_peer`] and starts subscribe-before-replay:
//!     register → buffer live updates → replay persisted history → flush buffer
//!     → go live,
//!   * forwards inbound `session/prompt`/steer to the agent-facing half, gated
//!     by [`Router::accept_steer`],
//!   * delivers permission requests only to the controller
//!     ([`Router::route_permission_request`]).
//!
//! Everything ACP-specific stays in the integration layer (goose-cli or a small
//! `goose-roaming-acp` crate) so this crate remains ACP-free; the broker core
//! here only owns the transport-neutral routing decisions.

use std::collections::HashMap;

use crate::invite::Scope;

/// Opaque id for a connected peer (one roaming ACP client).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(pub u64);

/// What a connected peer is allowed to do with the shared session.
///
/// Derived from the invite [`Scope`] at attach time. Only one subscriber may
/// hold [`Role::Controller`] at a time (the machine that owns the session, or
/// whoever it has handed control to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// May receive `session/update` only. Cannot prompt, steer, or answer
    /// permission/fs requests.
    Observer,
    /// May also send prompts and steering input.
    Steerer,
    /// May do everything, and is the sole recipient of permission/fs requests.
    Controller,
}

impl Role {
    /// Map an invite scope to a broker role.
    pub fn from_scope(scope: Scope) -> Role {
        match scope {
            Scope::Control => Role::Controller,
            Scope::Attach => Role::Steerer,
            Scope::Observe => Role::Observer,
        }
    }

    fn can_steer(self) -> bool {
        matches!(self, Role::Steerer | Role::Controller)
    }

    fn is_controller(self) -> bool {
        matches!(self, Role::Controller)
    }
}

/// Where a message the broker is routing should go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Deliver to every attached subscriber (e.g. `session/update`).
    Broadcast,
    /// Deliver to exactly one subscriber (e.g. `session/request_permission`
    /// goes to the current controller).
    To(SubscriberId),
    /// Drop: no valid recipient (e.g. a permission request with no controller
    /// attached — the broker must answer it itself with a safe default).
    Drop,
}

/// The reason an inbound peer message was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The peer's role doesn't permit this action.
    NotPermitted,
    /// The peer isn't attached to this session.
    Unknown,
}

/// Transport-neutral routing policy for one shared session.
///
/// This is the testable heart of the broker: it knows who is attached, their
/// roles, and who currently controls the session, and it answers "where does
/// this message go?" It performs no I/O.
#[derive(Debug, Default)]
pub struct Router {
    subscribers: HashMap<SubscriberId, Role>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a peer with the role derived from its invite scope. If it is a
    /// controller, it becomes *the* controller, demoting any previous one to
    /// steerer (last-controller-wins; handoff policy can refine this later).
    pub fn attach(&mut self, id: SubscriberId, role: Role) {
        if role.is_controller() {
            for existing in self.subscribers.values_mut() {
                if existing.is_controller() {
                    *existing = Role::Steerer;
                }
            }
        }
        self.subscribers.insert(id, role);
    }

    /// Detach a peer (disconnected).
    pub fn detach(&mut self, id: SubscriberId) {
        self.subscribers.remove(&id);
    }

    pub fn is_attached(&self, id: SubscriberId) -> bool {
        self.subscribers.contains_key(&id)
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// The current controller, if one is attached.
    pub fn controller(&self) -> Option<SubscriberId> {
        self.subscribers
            .iter()
            .find(|(_, role)| role.is_controller())
            .map(|(id, _)| *id)
    }

    /// Route an agent → peers `session/update` notification: always broadcast.
    pub fn route_notification(&self) -> Route {
        Route::Broadcast
    }

    /// Route an agent → peer request that needs a human decision
    /// (`session/request_permission`, fs/terminal): only the controller may
    /// answer. If nobody controls the session, the caller must supply a safe
    /// default (e.g. deny) — signalled by [`Route::Drop`].
    pub fn route_permission_request(&self) -> Route {
        match self.controller() {
            Some(id) => Route::To(id),
            None => Route::Drop,
        }
    }

    /// Decide whether an inbound peer `session/prompt` or `session/steer` is
    /// allowed. Steering is serialized by the caller (the broker holds a single
    /// input lease onto the agent); this only checks permission.
    pub fn accept_steer(&self, from: SubscriberId) -> Result<(), Refused> {
        match self.subscribers.get(&from) {
            None => Err(Refused::Unknown),
            Some(role) if role.can_steer() => Ok(()),
            Some(_) => Err(Refused::NotPermitted),
        }
    }
}

/// The stateful broker that owns one live session and drives the ACP wire on
/// both sides. Wraps a [`Router`] with the I/O seams a wire adapter implements.
///
/// This is the skeleton; the ACP plumbing is intentionally unimplemented and
/// marked at each seam so the shape is reviewable before we build it.
pub struct SessionBroker {
    router: Router,
    next_id: u64,
}

impl SessionBroker {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            next_id: 0,
        }
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Allocate a fresh [`SubscriberId`] for a newly accepted peer.
    pub fn next_subscriber_id(&mut self) -> SubscriberId {
        let id = SubscriberId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Seam: attach an accepted roaming peer's ACP client stream to the shared
    /// session.
    ///
    /// The wire adapter will:
    /// 1. allocate a [`SubscriberId`] and `router.attach(id, Role::from_scope)`,
    /// 2. run `session/load` against the local agent for the target session and
    ///    replay it to the peer (subscribe-before-replay: register first, buffer
    ///    live events, replay history, flush buffer, then go live),
    /// 3. forward this peer's `session/update`s per [`Router::route_notification`].
    pub fn attach_peer(&mut self, _scope: Scope) -> SubscriberId {
        // Skeleton: id + policy only. Wire replay/forwarding is the adapter's job.
        let id = self.next_subscriber_id();
        self.router.attach(id, Role::from_scope(_scope));
        id
    }

    pub fn detach_peer(&mut self, id: SubscriberId) {
        self.router.detach(id);
    }
}

impl Default for SessionBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (SubscriberId, SubscriberId, SubscriberId) {
        (SubscriberId(1), SubscriberId(2), SubscriberId(3))
    }

    #[test]
    fn scope_maps_to_role() {
        assert_eq!(Role::from_scope(Scope::Control), Role::Controller);
        assert_eq!(Role::from_scope(Scope::Attach), Role::Steerer);
        assert_eq!(Role::from_scope(Scope::Observe), Role::Observer);
    }

    #[test]
    fn notifications_broadcast() {
        let mut r = Router::new();
        let (a, b, _) = ids();
        r.attach(a, Role::Controller);
        r.attach(b, Role::Observer);
        assert_eq!(r.route_notification(), Route::Broadcast);
    }

    #[test]
    fn permission_requests_go_to_controller_only() {
        let mut r = Router::new();
        let (a, b, _) = ids();
        r.attach(a, Role::Observer);
        r.attach(b, Role::Controller);
        assert_eq!(r.route_permission_request(), Route::To(b));
    }

    #[test]
    fn permission_request_drops_with_no_controller() {
        let mut r = Router::new();
        let (a, _, _) = ids();
        r.attach(a, Role::Observer);
        assert_eq!(r.route_permission_request(), Route::Drop);
    }

    #[test]
    fn observers_cannot_steer() {
        let mut r = Router::new();
        let (a, b, c) = ids();
        r.attach(a, Role::Observer);
        r.attach(b, Role::Steerer);
        r.attach(c, Role::Controller);
        assert_eq!(r.accept_steer(a), Err(Refused::NotPermitted));
        assert_eq!(r.accept_steer(b), Ok(()));
        assert_eq!(r.accept_steer(c), Ok(()));
    }

    #[test]
    fn unknown_subscriber_cannot_steer() {
        let r = Router::new();
        assert_eq!(r.accept_steer(SubscriberId(99)), Err(Refused::Unknown));
    }

    #[test]
    fn last_controller_wins_demotes_previous() {
        let mut r = Router::new();
        let (a, b, _) = ids();
        r.attach(a, Role::Controller);
        assert_eq!(r.controller(), Some(a));
        r.attach(b, Role::Controller);
        assert_eq!(r.controller(), Some(b));
        // The old controller is demoted, not detached, and can still steer.
        assert_eq!(r.accept_steer(a), Ok(()));
    }

    #[test]
    fn detach_removes_subscriber() {
        let mut r = Router::new();
        let (a, _, _) = ids();
        r.attach(a, Role::Controller);
        assert!(r.is_attached(a));
        r.detach(a);
        assert!(!r.is_attached(a));
        assert_eq!(r.controller(), None);
    }

    #[test]
    fn broker_allocates_unique_ids_and_attaches() {
        let mut b = SessionBroker::new();
        let first = b.attach_peer(Scope::Control);
        let second = b.attach_peer(Scope::Observe);
        assert_ne!(first, second);
        assert_eq!(b.router().subscriber_count(), 2);
        assert_eq!(b.router().controller(), Some(first));
    }
}
