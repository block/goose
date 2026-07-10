# Plan: Sonar Remote Session Gateway

**Goal:** Add production-safe remote Goose control over Sonar/Marmot groups, with one authenticated MLS group bound to one Goose session and a shareable group usable from Sonar today and richer Goose-mobile clients later.

**Chosen approach (Approach B):** Run the evolving Sonar Core/MDK stack in a pinned, bundled bridge process and expose it to Goose through a versioned JSON-lines protocol. Goose owns authorization, session routing, and agent execution; Sonar Core owns Nostr identity, SQLCipher-backed MLS state, relay synchronization, and authenticated sender identity.

Directly linking `sonar-core` into `goose` is not viable at the pinned revisions: Goose's `sqlx` graph links `libsqlite3-sys 0.30`, while Sonar Core's persistent MDK store links `libsqlite3-sys 0.35`. Cargo forbids two crates with `links = "sqlite3"` in one binary. The process boundary preserves durable MLS state without an unsafe dependency fork or a workspace-wide SQLite migration.

## Affected files

- `integrations/sonar-bridge/Cargo.toml` and `Cargo.lock` — independent Rust build island with Sonar Core pinned to commit `ec6a1841eaa42550eed52918158ad18ac5912807`.
- `integrations/sonar-bridge/src/main.rs` — serialized `SonarClient` actor, protected identity/database initialization, live relay drain/reconciliation, durable inbound-command ledger, group acceptance, and JSON-lines transport.
- `integrations/sonar-bridge/README.md` — build, identity backup, protocol, and operational guidance.
- `crates/goose/src/gateway/sonar/` — supervised child-process transport, bounded protocol parsing, authenticated controller filtering, per-group serialization, and mapping of Sonar messages to Goose gateway requests.
- `crates/goose/src/gateway/mod.rs` — feature-gated `sonar` gateway registration.
- `crates/goose/src/gateway/pairing.rs` — pairing records that can target an existing session and enforce one Sonar group per session.
- `crates/goose/src/gateway/handler.rs` — attach a paired group to an existing session without overwriting that session's model, extensions, project, or mode; acknowledge inbound commands only after execution finishes.
- `crates/goose/src/gateway/manager.rs` — validate session-targeted pairing requests and expose bridge health/identity in status.
- `crates/goose/Cargo.toml` and `crates/goose-cli/Cargo.toml` — `sonar-gateway` feature wiring without linking Sonar Core into the Goose binary.
- `crates/goose-cli/src/cli.rs` and `crates/goose-cli/src/commands/gateway.rs` — Sonar bridge/home/relay/controller options plus `gateway pair sonar --session-id <id>`.
- `documentation/docs/experimental/remote-access/sonar-gateway.md` and `documentation/docs/experimental/remote-access/index.md` — secure setup, group sharing, controller policy, recovery, and limitations.

## Protocol and trust model

- The bridge is the sole owner of `SonarClient`; every sync, drain, invite acceptance, send, and ledger mutation runs through one serialized actor.
- Bridge stdout is protocol-only; diagnostics go to stderr. Goose parses bounded lines and rejects unknown protocol versions or message types.
- `ChatMessage.sender` from Sonar Core is the authenticated identity. Goose never trusts an npub claimed inside message content.
- The gateway requires an explicit controller npub allowlist. MLS membership alone grants visibility, not local tool-execution authority. Shared group members remain observers until locally added to the allowlist.
- A one-time Goose pairing code binds the Sonar group ID to a session. `--session-id` binds an existing session; omitting it preserves the existing gateway behavior of creating a dedicated session.
- The bridge persists inbound state before delivery. A command left `executing` by a crash is never replayed automatically; the group receives an interrupted/resubmit response after recovery.
- Existing transcript history is checkpointed when a group is first observed, so enabling the gateway cannot execute stale messages from relay backfill.
- Plain authorized text is the Sonar-compatible prompt path. The JSON-lines bridge protocol is versioned so a later Goose-mobile client can add typed prompt/steer/cancel and structured run events without changing MLS transport.

## Edge cases

- Bridge missing, wrong version, malformed output, oversized line, early exit, stderr flood, or shutdown timeout.
- No relay quorum, reconnect, duplicate relay delivery, new group backfill, and partial send failure.
- Existing session missing/deleted between code creation and pairing.
- Two groups attempting to bind the same session, or one group attempting to rebind another session.
- Unauthorized, removed, or newly invited group members sending prompts.
- Concurrent messages from multiple controllers in one group; process in deterministic group order.
- Local and remote prompts racing. The first slice will register gateway runs with the existing `AgentManager` busy/cancel registry and fail closed with a busy response; cross-process/independent-ACP-agent arbitration remains explicitly unsupported rather than silently concurrent.
- Crash after command acceptance but before/after agent side effects; never replay ambiguous work automatically.
- Identity/database key loss; docs must state that MLS history cannot be recovered from relays without the bridge state backup.

## Test plan

- Bridge unit tests for config permissions, protocol round trips, size/version rejection, first-seen group checkpointing, duplicate suppression, and interrupted-command recovery.
- Goose unit tests using a fake bridge process for ready/message/ack/error/exit paths and controller filtering.
- Pairing tests for legacy new-session codes, existing-session attachment, expiry, gateway mismatch, and one-group-per-session enforcement.
- Gateway handler tests that an attached existing session keeps its provider/model/extensions/mode unchanged.
- Feature-off build to prove Sonar Core is not pulled into Goose's dependency graph.
- `cargo fmt --all` in both workspaces.
- Targeted root tests, `cargo build`, and `cargo clippy --all-targets -- -D warnings` as required by `AGENTS.md`.
- `cargo test` and `cargo clippy` for `integrations/sonar-bridge` with the pinned Sonar dependency.
- Manual two-identity smoke test when relay credentials/network are available: add the Goose npub to a Sonar group, pair it to a session, send a prompt, share the group, verify observer denial, allowlist the second npub, and verify ordered replies.

## Conventions

- Use `anyhow::Result` in Goose, keep error context actionable, and avoid self-evident comments.
- Add dependencies with `cargo add`; keep both lockfiles consistent.
- Keep Sonar secrets and MLS state out of `Session.extension_data` and session export.
- Pin git revisions exactly; do not follow Sonar or MDK branches.
- Run `cargo fmt`; do not merge without clippy.
- Do not resurrect removed REST/OpenAPI code or import desktop generated OpenAPI clients.

## Non-goals for this PR

- BLE mesh transport; the gateway is Nostr-relay/Marmot only.
- Editing the Sonar or `goose-mobile` repositories.
- Rendering rich ACP tool/thinking/permission events in mobile UI.
- Remote mutation of Goose's global permission policy; group messages cannot create `AlwaysAllow` rules.
- Automatic approval of arbitrary MLS members as Goose controllers.
- Solving multi-process arbitration between a separately launched CLI gateway and a separately launched desktop backend. Documentation will require one Goose backend owner for a remotely controlled session.

## Open risks and follow-ups

- Sonar currently makes all MLS group members admins. The explicit Goose controller allowlist prevents new members from executing commands, but group admins can still change who can read future group traffic; the shared-audience warning must be prominent.
- The proprietary Goose-mobile fork should consume a future versioned control envelope and ACP event projection rather than exposing raw tool results or model thinking in a shared group.
- A later upstream Sonar storage feature or SQLite dependency alignment may allow replacing the bridge with an in-process adapter without changing Goose's gateway-facing protocol.

**Estimated size:** L (>200 LOC), spanning two Rust build islands, gateway routing, persistence semantics, CLI, tests, and documentation.
