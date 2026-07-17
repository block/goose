# Roaming Agents — Design & Research

> Branch: `micn/roaming-agents`
> Status: **Research / early design** (no implementation yet)
> Author: research pass with the `expert` (codex) second opinion + study of the sibling `mesh-llm` project

## 1. What we're trying to build

Give goose agents a **peer-to-peer presence on the internet** using [iroh](https://iroh.computer) (n0's QUIC-based p2p library, v1.0). iroh gives us NAT traversal via public/self-hosted relays and a self-certifying identity (each endpoint *is* an ed25519 key). Deep in goose, this becomes an SDK-like building block we can surface in several ways:

1. **Share a running agent across the wire** — a remote ACP client (goose desktop, another goose, any ACP client) connects to and drives an agent running on your machine, through a relay, without opening ports.
2. **Agent-to-agent collaboration** — from inside the agent loop, an agent can look up its *other* agents (same user's fleet, or trusted peers), send messages, steer them, and collaborate.
3. **Long-lived listeners** — CLI options so `goose acp` / `goose serve` can hold an iroh connection open while the agent loop runs.

The core realization: **goose already speaks ACP over an arbitrary byte stream**, so an iroh bidirectional stream is a drop-in transport. The hard parts are *security, identity, session ownership, and lifecycle* — not the wire.

---

## 2. Why this is a natural fit for goose

goose's ACP layer is transport-agnostic at exactly the right seam:

```rust
// crates/goose/src/acp/server.rs
pub fn serve<R, W>(agent: Arc<GooseAcpAgent>, read: R, write: W)
    -> impl Future<Output = Result<()>>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{ /* runs the JSON-RPC ACP protocol over any byte stream */ }
```

Today `serve` is driven over:
- **stdio** (`goose acp`, `run()` in `server.rs`), and
- **WebSocket/HTTP** via axum with token auth (`goose serve`, `crates/goose/src/acp/transport/mod.rs`).

The **client** side is equally pluggable:

```rust
// crates/goose/src/acp/provider.rs
AcpProvider::connect_with_transport(name, mode, config, transport /* : ConnectTo<Client> */)
```

So goose can be **both** an ACP agent (server) and an ACP client. An iroh bi-stream `(SendStream, RecvStream)` implements `AsyncWrite`/`AsyncRead`, so it plugs directly into both seams. iroh streams are ordered, reliable, flow-controlled QUIC streams — a strictly cleaner substrate than stdio for JSON-RPC.

**Config / key storage** already exists: `Config` with system-keyring-backed secrets and a config dir (`~/.config/goose`), plus `Paths` helpers. Persisting a node key is straightforward.

---

## 3. What we learned from `mesh-llm` (../deez)

`mesh-llm` already runs iroh 1.0 in production for distributed inference. Key patterns worth copying (file citations are into `/Users/micn/Development/deez`):

### Transport
- **iroh 1.0 API surface**: `EndpointId` / `EndpointAddr` / `TransportAddr` (not the old `NodeId`/`NodeAddr`), `presets::Minimal`, `RelayConfig`/`RelayMap`.
- **Endpoint bind** (`mesh/mod.rs:2110`):
  ```rust
  Endpoint::builder(iroh::endpoint::presets::Minimal)
      .secret_key(secret_key)
      .alpns(vec![ALPN_V1.to_vec(), STAGE_ALPN_V2.to_vec()])
      .transport_config(startup_transport_config())
      .relay_mode(relay_mode_for_startup(relay))
      .bind().await
  ```
- **Multiple ALPNs**, branch on `accepting.alpn()` to route subprotocols.
- **Custom relay map** — they never use n0's default relays; they point at self-hosted relays (`mesh/mod.rs:596`) and support **per-relay auth tokens** (`with_auth_token`, `mesh/mod.rs:654`) for zero-trust gating.
- **QUIC tuning matters** for long silent request/response cycles: `keep_alive_interval(10s)`, `max_idle_timeout(300s)`, and **per-path** timers (iroh 1.0 multipath tears down idle paths independently; the last path closing kills a connection mid-work). `mesh/mod.rs:2060`.
- **Stream multiplexing**: every bi-stream starts with a 1-byte type tag, then u32-LE length-prefixed `prost` frames. `protocol/mod.rs:607`.

### Identity & security
- **NodeId == ed25519 public key**, self-certifying at the QUIC-TLS handshake — iroh proves the peer holds that key. This is *the* security primitive. (`node_key.rs`)
- **Two key layers**: transport node key (anonymous, per endpoint) vs. **owner key** (human/tenant identity, ed25519 sign + X25519 encrypt). Owner key **signs a certificate binding owner ↔ node** (`ownership.rs`).
- **Signed bootstrap token** (`requirements.rs:44`): base64url payload with `EndpointAddr`(s) + relays + `expires_at` + issuer pubkey + ed25519 signature over **domain-tagged, length-prefixed canonical bytes** (`b"mesh-llm-bootstrap-token-v1:"`). Offline-verifiable, TTL-revocable.
- **Access control**: `TrustPolicy { Off | PreferOwned | RequireOwned | Allowlist }` + a local `TrustStore` (allowed/revoked keys). Because iroh authenticates the *client's* key at handshake, the host can allowlist **by client public key at accept-time**.
- **Anti-replay admission proof** (`DirectNodeAdmissionProof`, ±30s skew) for the general bearer-token case.
- **Sign-then-encrypt envelopes** (`crypto_box` X25519 + XSalsa20-Poly1305) for confidential owner→owner control messages (`envelope.rs`).

---

## 4. Expert review — corrections that shaped the design

An independent model review (codex, read-only) flagged several issues with the naive first design. These are folded into the design below:

1. **Authorization ≠ authentication.** iroh proves *who* the peer is (`remote_id()`); the app must still decide *what they may do*. Enforce a capability model: `token.audience == our EndpointId`, `token.subject == connection.remote_id()`, plus resource/scope checks. Do this in an **`after_handshake` hook** (runs for both inbound and outbound — check direction).
2. **"Only issuer key may connect" is wrong** — if the host issues the token, that would mean only the host can connect. Bind tokens to the *client's* key, or pin the client key on first redemption.
3. **The ±30s signed proof is largely redundant** — TLS already proves current key possession. Replay protection should come from **binding the capability to `remote_id`** (replay by another key fails naturally) or **one-time redemption** for bearer codes. Don't over-engineer.
4. **Don't call bearer mode "PSK"** — it isn't part of the TLS key exchange. It's a bearer capability.
5. **Never ship relay auth tokens inside an invite.** Share relay URLs only; each endpoint fetches its own relay credential. Also avoid dumping every direct `EndpointAddr` (leaks LAN/private addrs, goes stale) — prefer relay URL + let holepunching upgrade.
6. **Disable 0-RTT** for ACP / any mutating op (iroh warns 0-RTT data is replayable).
7. **The biggest security concern is goose's permission model.** `handle_tool_permission_request` (`server.rs:1907`) sends tool permission choices — including `AllowAlways` — to the connected ACP client and trusts the response. **A full ACP invite is effectively remote shell access.** Peer-agent messaging must get a *much narrower* scope and must NOT inherit permission-approval authority.
8. **Don't share one connection-scoped `GooseAcpAgent` across clients**, and don't claim to share a *live* session while actually spinning independent session runtimes. Introduce a `SessionCoordinator` with a stable `AgentId` (distinct from `EndpointId`), one active runtime per session, and a single-controller lease + observers.
9. **Use iroh's `Router`** rather than a hand-rolled accept loop; supervise tasks; await `Router::shutdown` on teardown. Bound stream counts / receive windows / idle timeouts / task counts.
10. **One node key per running process.** Don't load one global persisted key into multiple concurrent goose processes (ambiguous address/relay ownership). Either a single device-level roaming daemon that hosts multiple agents, or per-process keys certified by a separate device/user identity.
11. **`AcpProvider` client needs hardening before internet use**: it runs transports on a dedicated OS thread with its own current-thread runtime (`provider.rs:177`), `Drop` synchronously joins that thread and sends remote `session/close` without a timeout (`provider.rs:673`, `:1173`) — a stalled peer hangs destruction. It also eagerly creates a session and can't attach to an invited live session (`provider.rs:257`) — needs explicit new-vs-load modes.
12. **Public relays are rate-limited** — n0 recommends dedicated relays for production (like mesh-llm's self-hosted ones).

---

## 5. Proposed architecture

### 5.1 Layering

```text
goose-roaming (new crate / module)  ── the SDK construct
  ├─ RoamingIdentity      persisted ed25519 node key (0600, optional keyring)
  ├─ RoamingEndpoint      owns the iroh Endpoint + Router + relay config
  │    ├─ after_handshake policy hook (key allowlist / revocation / direction)
  │    └─ Router
  │         ├─ ALPN "goose-acp/1"      → capability check → agent factory → serve()
  │         └─ ALPN "goose-roaming/1"  → pairing, presence, directory, mailbox
  ├─ Invite / Capability  signed, TTL'd, audience+subject-bound tokens
  └─ TrustBook            local allow/revoke of client keys + token IDs

SessionCoordinator (local, shared)
  ├─ stable AgentId (≠ EndpointId)
  ├─ one active runtime per session
  ├─ single-controller lease
  └─ observer/subscriber notifications
```

### 5.2 Sharing an agent (server side)

- `goose acp --roam` / `goose serve --roam`: bind the endpoint, start the `Router`, print a **signed invite** (base64url).
- On the `goose-acp/1` ALPN: run the `after_handshake` capability check, then `accept_bi()`, and hand the `(recv, send)` pair to the **existing** `serve<R,W>(agent, recv, send)`. The iroh bi-stream replaces stdio/websocket verbatim.
- **Per-connection agent** from a factory (never a shared one). Enforce client-key allowlist and capability scope at accept.

### 5.3 Connecting as a client

- `goose session --connect <INVITE>`: decode invite → verify signature/TTL/audience → dial `EndpointAddr` over `goose-acp/1` → `open_bi()` → feed the stream to the ACP **client** transport, so the remote agent becomes the local provider.
- Desktop: a small **local loopback ↔ iroh sidecar** in Rust is the cleanest incremental path — Electron keeps its existing WebSocket ACP client unchanged; Rust owns iroh. Treat it as an adapter, not the core protocol.
- Add explicit **new-session vs load-session (attach)** connection modes; harden `AcpProvider` shutdown with cancellation + timeouts first.

### 5.4 Agent-to-agent messaging

- Separate ALPN `goose-roaming/1` with a small message protocol (presence, directory lookup, send-message, optional mailbox). **Do not** reuse the full ACP invite for this — it carries permission-approval authority.
- Expose to the model as narrowly-scoped platform tools, e.g. `platform__roaming_list_agents`, `platform__roaming_send_message`, `platform__roaming_steer`. These make scoped calls, never full ACP tool-permission calls.
- Directory/discovery: start with an explicit trusted-peer book; defer gossip until membership/privacy/revocation semantics are defined.
- Guard against agent-to-agent **loops / fan-out / mutual waits** (runaway cost).

### 5.5 Security defaults (revised)

- **Default**: authenticated ACP transport only. Token is **host-signed, client-key-bound** capability; connection rejected unless `remote_id()` matches an allowlisted / token-bound key.
- **First-time pairing**: short-lived, **single-use** bearer code that **pins the authenticated client `EndpointId` on redemption**.
- **Open reusable bearer capability**: explicit opt-in lower-security mode.
- **No unauthenticated mode** without a conspicuous `--dangerously-*` flag (mirrors existing `--dangerously-unauthenticated` on `goose serve`).
- Token fields: version, domain-separation tag, issuer, audience, subject, resource, scopes, `nbf`, `exp`, token-id. Sign **canonical bytes**, not reserialized JSON. Persist revocations by token-id and client key.
- **Disable 0-RTT.** Share relay **URLs** only, never relay credentials. Don't log invites/addresses/session metadata.

### 5.6 Lifecycle

- Single owner responsible for ordered shutdown: stop accepting → drain/cancel ACP handlers → finish streams → `Router::shutdown` → persist trust state.
- Bound QUIC stream counts, receive windows, idle timeouts, task counts (worst-case memory ∝ streams × window).
- Copy mesh-llm's keep-alive + per-path idle timeout tuning for long silent turns.

---

## 6. MVP scope (recommended)

Ship narrow, defer the ambitious parts until semantics are explicit:

1. `goose-roaming` module: persisted key, `RoamingEndpoint` on `Router`, one ALPN `goose-acp/1`.
2. **Authenticated ACP transport only** — `goose acp --roam` prints an invite; `goose session --connect <INVITE>` dials it.
3. Per-connection agent factory + client-key **allowlist / trusted-peer book** + single-use pairing codes.
4. Explicit new-vs-load session behavior; hardened `AcpProvider` shutdown (cancellation + timeouts).
5. Strict resource limits + ordered shutdown.

**Defer**: gossip discovery, durable messaging/mailbox, simultaneous live-session control, model-facing peer tools (add later by making scoped ACP/roaming calls), sign-then-encrypt envelopes.

---

## 7. Example / theoretical usages

```bash
# Share the agent running here; prints a single-use, client-key-pinned invite
goose acp --roam --pair
#   invite: goose+roam://Aghk...<base64url signed capability>...

# On another machine / another goose: drive that agent over the relay
goose session --connect 'goose+roam://Aghk...'

# Restrict who can connect (allowlist client public keys)
goose serve --roam --allow-key z6Mk...pubkey1 --allow-key z6Mk...pubkey2

# Lower-security shareable bearer link (opt in explicitly)
goose acp --roam --bearer --ttl 1h
```

Theoretical use cases this unlocks:
- **Remote control of a home/work agent** from a laptop or phone-side ACP client without port-forwarding.
- **A fleet of user-owned agents** that discover and delegate to each other (e.g. a "coordinator" agent steering specialized workers on different machines).
- **Pair-agenting**: two people's agents connect to collaborate on a shared task with scoped messaging.
- **Ephemeral share links** for a colleague to observe/assist a running session.
- **CI / build agents** that expose themselves to a central orchestrator over relays.

---

## 8. Open questions / risks to resolve before building

- Exact **capability/scope model** — what does "observe" vs "control" vs "message" mean concretely against ACP methods? (Ties directly into the permission-approval hole in `server.rs:1907`.)
- **Session attach** semantics — can a roaming client join a live local session, or only spawn new ones? `SessionCoordinator` + single-controller lease design.
- **Relay operations** — do we self-host (like mesh-llm's `iroh.link` relays) or lean on n0's rate-limited defaults for a first cut?
- **Desktop integration** — sidecar vs native transport in the Electron app.
- **Key/process model** — device-level daemon vs per-process keys.

---

## 11. Live `roam connect` VERIFIED + core ACP keyring hang (root-caused)

`roam connect` is now a working thin ACP client UI onto the host's agent and was
verified end to end: two processes over the real n0 relay, `initialize` →
`session/new` → prompt "7×6" → **remote agent replied `42`** → rendered locally.

### The hang that blocked it (NOT a roaming bug)

Initial testing hung at `session/new`. Root-caused via staged instrumentation +
expert review to a **goose-core** issue on the ACP session-init path that
`goose run` never exercises (so it's unrelated to roaming, and reproduces with
goose's own `crates/goose-sdk/examples/acp_client.rs` over plain stdio):

```
handle_new_session
  -> prepare_acp_session_agent
     -> maybe_refresh_provider_inventory_with_agent   (server.rs)
        -> find_entry_for_provider -> describe_provider
           -> inventory_identity()  (providers/inventory/registrations.rs)
              -> config.get_secret("ANTHROPIC_CUSTOM_HEADERS")   <-- BLOCKS
```

The anthropic inventory-identity closure does a **synchronous keychain read**
for `ANTHROPIC_CUSTOM_HEADERS`. On an unsigned dev binary spawned with piped
stdio, the macOS keychain ACL prompt can't be answered, so the read blocks
forever and `session/new` never returns.

**Route-around (the intended mechanism, not a hack):** goose's secret lookup
order is **env var → keyring → `secrets.yaml`** (`config/base.rs:848`,
`get_secret`). `GOOSE_DISABLE_KEYRING=1` (a documented first-class env var,
`documentation/docs/guides/environment-variables.md:323`) swaps the keyring step
for the non-blocking `secrets.yaml` file, so the inventory-identity closure never
touches the blocking keychain and `session/new` returns instantly. It does **not**
drop the API key: env vars are still checked first regardless. `goose run` avoids
the hang only incidentally — `ANTHROPIC_API_KEY` is in env, so it short-circuits
before the keyring; `ANTHROPIC_CUSTOM_HEADERS` is absent from env, so the ACP path
falls through to the keychain.

There is no narrower "skip inventory refresh" env var
(checked `should_refresh_inventory_for_session_init` + the inventory module);
`GOOSE_DISABLE_KEYRING` is the correct knob. The roaming host should set/document
it until core follow-up #1 (below) moves the secret reads off the hot path.

### FIXED (core, benefits every ACP client)

The keychain read is now **bounded by a 3s timeout on a dedicated thread**
(`config/base.rs read_keyring_password_with_timeout`), falling back to the
existing file storage on timeout. One place, fixes all callers; `get_secret`
stays sync (no ripple). The provider-inventory refresh was also made
fire-and-forget off the `session/new` path (`acp/server.rs
spawn_provider_inventory_refresh`) as defence-in-depth. Verified: the official
`acp_client` example AND `roam connect` both complete with the keyring **enabled**
(no `GOOSE_DISABLE_KEYRING`), remote agent replying correctly over the n0 relay.

### Remaining follow-ups (core, separate from roaming)
1. There are **no start/end log markers** around provider-restore, bulk
   extension load, or inventory refresh — add them; verbosity alone couldn't
   localize this.
2. Expert also flagged (not the cause here, but real): MCP stdio `initialize`
   is an unbounded `client.serve(transport).await` — the configured extension
   timeout only guards later requests, so an MCP server that starts but never
   answers `initialize` can hang session setup too.

### Next roaming steps unchanged
`serve_with_policy` + scope enforcement (§9), then `roam__list_peers` +
`roam__delegate` (§10). Consider defaulting the roaming host to tolerate the
keychain issue (or documenting `GOOSE_DISABLE_KEYRING`) until follow-up #1 lands.

## Appendix: key file references

**goose (this repo)**
- ACP `serve<R,W>` seam: `crates/goose/src/acp/server.rs` (~line 3133)
- ACP client transport: `crates/goose/src/acp/provider.rs` (`connect_with_transport`, `spawn`, `Drop`)
- HTTP/WS transport + token auth: `crates/goose/src/acp/transport/mod.rs`, `.../auth.rs`
- `goose serve` command: `crates/goose-cli/src/cli.rs` (~line 1392)
- Tool permission handling (the sharp edge): `crates/goose/src/acp/server.rs:1907`
- Config/secrets + keyring: `crates/goose/src/config/base.rs`; paths: `crates/goose/src/config/paths.rs`

## 9. Connect-side architecture (post-implementation research)

After landing the transport + host-side hosting, a second expert review corrected
a topology mistake I was about to make on the *connect* side. Recording it here
so the next implementation session starts from the right model.

### The topology error to avoid

Tempting but **wrong**: on `roam connect`, wrap the remote agent as a *provider*
(`AcpProvider::connect_with_transport`) for a fresh **local** session. That
creates a confusing double agent-loop — the connecting side would run its own
agent loop and tool execution locally, with the remote reduced to a model-like
backend. That defeats the entire premise ("share *your* agent — its tools, its
working dir, its shell — across the wire").

### Correct model: connect = thin ACP client UI onto the host's agent

- The **host** runs the real agent loop via `serve()` (`on_prompt` →
  `agent.reply(...)`, `server.rs:2550`). Its tools, filesystem, working dir.
- The **connecting client** is just an ACP *client UI*: it speaks
  `initialize` → `session/new` → `session/prompt` over the stream and renders
  `session/update` notifications to the terminal. No second local `Agent`.
- `AcpProvider::connect_with_transport` is therefore the **wrong** tool here.
  Build a small ACP-native terminal client instead (skeleton already exists at
  `crates/goose-sdk/examples/acp_client.rs`).

### Crate placement (revised)

- `goose-roaming`: **pure transport** — iroh endpoint, invites, authorization,
  raw authorized streams. No `AcpProvider`, no UI, no session policy.
- **Move `GooseAcpBridge` OUT of `goose-roaming` into `goose-cli`.** It is the
  only thing forcing the transport crate to depend on `goose`. If a second
  non-CLI consumer appears later, extract a small `goose-roaming-acp`
  integration crate — *not* a provider.
- `goose-cli/src/commands/roam/client.rs`: builds `ByteStreams`, runs the ACP
  `Client`, renders updates, answers permission requests. ACP types in the CLI
  are not a layering leak — the CLI is the composition/presentation boundary.
  Add a direct `agent-client-protocol` dep rather than leaning on `goose`'s.
- `goose` core: transport-independent host access policy + enforcement. Still
  **no iroh dependency**.

### Host-context requirements on connect

- Advertise **no** client filesystem / terminal capabilities. Otherwise goose
  replaces host developer operations with callbacks to the *connecting* machine
  (`server.rs:1085`) — the opposite of what we want.
- Do **not** use the connector's local cwd. `session/load` currently rewrites
  the hosted session's working dir when the supplied cwd differs
  (`server.rs:1162`). The host must impose the `share` cwd and preserve it on
  resume.

### Scope: don't advertise what isn't enforced (decision)

A later review corrected the framing here: a shared agent is given to a
**trusted, authenticated** peer, and a `Control` invite is *intentionally*
SSH-equivalent — the peer driving the agent and approving its tool use is the
feature, not a gap. There is nothing to defend against for `Control`.

The only real risk is **false security**: advertising `Observe`/`Attach` as if
they were enforced when they are not. So the near-term surface is simplified to
**`Control` only** — the CLI `--scope` flag is removed and `roam share` always
grants full control with a clear warning. The richer `Scope` enum stays in the
library for when a session coordinator can actually enforce observe/attach
(§9 roadmap steps 3–4). The `serve_with_policy` workstream is therefore dropped
from the near-term plan; it only mattered for enforcing lesser scopes.

For **delegation** (§10) the thing that still matters is not authz (the peer is
trusted) but **loop/cost safety** — that guardrail is independent of scopes and
is retained.

### (Deferred) Scope enforcement — only if lesser scopes are ever shipped

- Pass the handshake's granted scope into **host-side** ACP enforcement.
  Client-side cancellation is only defense-in-depth.
- Do **not** teach core about iroh's `Scope`. Introduce a transport-neutral
  `AcpConnectionPolicy { session_access, workspace, mutation_access,
  permission_access }` and a `serve_with_policy(...)`, keeping `serve(...)` as
  the unrestricted compatibility path.
- Enforce in two places:
  1. **Incoming method dispatch**: `new`, `load`, `prompt`, `cancel`, `close`,
     config/mode changes.
  2. **Permission generation**: unauthorized clients must *never receive* the
     permission request; resolve it host-side as deny/cancel. For limited
     controllers, omit `AllowAlways`/`RejectAlways`. This cannot be done with a
     byte-stream wrapper — `handle_tool_permission_request` sends a reverse ACP
     request and consumes its response directly (`server.rs:1907`).

Eventual scope matrix:

| Scope | Sessions | Prompt | Permissions |
|---|---|---|---|
| Control | share-scoped new/resume | yes | explicitly configured full authority |
| Attach | one token-bound session, with lease | yes | prefer once-only |
| Observe | one token-bound subscription | no | never routed |

`Attach`/`Observe` invites also need a **signed session/share resource** in the
claims (today `InviteClaims` carries only a scope, `invite.rs:44`). Without
resource binding, "attach" degrades into "load any host session".

### Session semantics: defer true live-attach

Ship in this order:

1. **New host-side session, `Control` only** (minimum useful release):
   "connect to the host process and create a durable session using the host's
   provider, tools, filesystem, and configured share directory."
2. **Disconnected-session resume** with a process-wide exclusive lease keyed by
   session id (`LoadSession` only after the old controller disconnects). Do
   *not* allow unrestricted `list`/`load` — the server uses the normal goose
   data dir, so an unfiltered roaming client could enumerate unrelated host
   sessions.
3. **Session coordinator**: one controller + notification fan-out (substantial
   refactor — notifications currently go straight to the prompting connection).
4. **Live observers + controller handoff.**

Until step 3, **hide/reject `Observe` and `Attach`** — an observer connection
has no live session to observe yet.

### Immediate next implementation slice

> CLI thin ACP client · host-selected workspace · `Control`-only new session ·
> no `AcpProvider` · no live-attach claims yet · `GooseAcpBridge` moved to CLI.

---

## 10. Peer registry & model tool surface (research)

Third expert review, on two threads the user raised: richer peer tracking
(both directions) and letting the *agent loop* reach other agents. Guiding
principle: **crisp, minimal surfaces over completeness.**

### Thread A — peer registry commands

Three logical stores, kept separate (do not fold together):

| Store | Owns | Backs |
|---|---|---|
| `PeerBook` | user-managed **outbound** contacts + their saved credential | `peers` |
| `TrustBook` / `GrantBook` | **inbound** authorization: allowed client keys + issued-invite metadata | `grants` |
| `Directory` | **observed** connection facts only (already built) | `connections` |

A saved-but-never-connected peer has no `Directory` entry, so it can't live
there — hence a dedicated `PeerBook`.

Command surface:

```text
goose roam peers                       # list saved remotes I can connect to
goose roam peers save <name> <invite>  # add or refresh a contact's credential
goose roam peers remove <name>
goose roam peers rename <name> <new>

goose roam connect <name-or-invite>    # connect by nickname OR raw token

goose roam grants                      # invites I've issued + allowed client keys
goose roam grants revoke <grant-id>    # accepts invite:7da2… or key:ab91…

goose roam connections                 # live/observed connections (was: list)
```

- `roam list` stays as a compatibility alias for `connections`.
- `save` (not `add`) — naturally adds *or* refreshes an expired credential.
- Minting stays part of `share`; no separate `grants create`.
- Grant IDs are typed: `invite:<id>` / `key:<fingerprint>`; `revoke` takes either.

Storage rules:
- `PeerBook` stores the **complete invite** (it's the outbound credential).
- The issuer stores invite **metadata only** (token-id, scope, expiry, allowed
  keys, redeemed/revoked) — *not* the printable token.
- All: atomic replace, `0600` files, `0700` dirs, lock if multi-process.

Credential security:
- `0600` is fine for v1 (≈ ssh key / API token). Keychain later.
- **Client-key-bound** credentials are the normal saved form.
- Saving a **bearer** credential must warn + require `--allow-bearer`.
- `peers` always shows scope + expiry. A long-lived bearer `control` credential
  is effectively remote-shell access.

Naming: **peer** = saved remote contact; **agent** = the remote agent reached
after connecting; **endpoint id** = security fingerprint; **node** =
implementation/diagnostics only. Avoid "contact".

### Thread B — model tool surface (agent-to-agent)

Runtime-injected `roam` platform extension with exactly **two stable tools** —
**not** a tool/skill per peer:

```text
roam__list_peers()          # lazy: names, descriptions, scope, expiry,
                            # connected/last-seen, endpoint fingerprint
roam__delegate(peer, task)  # resolve name -> open ACP -> fresh remote session
                            # -> one prompt -> await final response -> close
```

- The "named like skills" idea is preserved: peers are **named capability
  records discovered lazily** via `list_peers`, then addressed by name via
  `delegate`. No per-peer schema churn, naming collisions, or prompt bloat.
- `list_peers` must **not** claim reachability — a saved address is not
  presence; only dialing proves it.
- `delegate` (better name than send/ask): one-shot request/response covering
  both questions and sub-tasks; maps onto the ACP `session/prompt` we already
  have. Durable conversations/steering deferred.
- **Rejected** alternatives: dynamic per-peer skills (confuse instructions with
  capabilities, go stale); surfacing the remote's *tools* as an MCP extension
  (transitive tool federation — obscures which machine executes, bypasses the
  remote agent's policy boundary — a separate future feature).

Injection architecture (matches §4 / earlier advice):
- Add a **per-agent runtime platform-extension registry** whose factory is
  `Arc<dyn Fn(...)>`, threaded through `AgentConfig` / `AcpServerFactoryConfig`
  — *not* a global mutable registry, not a core `PeerMessenger` trait.
  (`platform_extensions/mod.rs:271` currently uses static strings + a
  non-capturing fn pointer.)
- CLI registers a `RoamExtensionClient` capturing `Arc<RoamingConnector>`. The
  adapter lives in `goose-cli`; core only sees its existing MCP/platform-client
  abstraction and never imports iroh.

Permission & loop safety:
- Caller: `list_peers` read-only; `delegate` is open-world + **approval-required
  like shell** (approval shows resolved peer name, authenticated endpoint id,
  scope, task preview).
- Permissions are keyed by **tool name** (`permission.rs:145`), so
  "Always Allow `roam__delegate`" would cover peers added later. Until the
  principal can be `roam__delegate:<endpoint-id>`, keep delegate **Ask-only**
  outside global Auto mode.
- Callee: add a distinct **`Delegate` scope** (not ordinary `Control`); stamp
  the session as driven by an authenticated *remote agent*, not a human; never
  let the calling model approve the callee's permission prompts; host-owned
  delegated-session policy (read-only or explicit allowlist as safe default).
- **v1 loop prevention (cleanest): do NOT inject the `roam` extension into
  remotely-delegated sessions** — recursion becomes impossible. Plus: fresh
  session per delegation, hard deadline (~5 min), hard turn cap (~8), one
  active delegation per peer + small global concurrency cap, propagate
  cancellation, attach a trace/delegation id. Hop-counts only needed if
  recursive delegation is later added intentionally.

### Prerequisite this surfaced

`roam connect` currently generates a **fresh ephemeral identity per
connection** (`roam.rs`). That makes durable key-bound grants and pairing
impossible across reconnects. Before Thread A/B land, outbound connections need
a **stable client identity** (use the persisted roaming key, or a single
identity-owning broker/daemon does all dials). Pairing should then be:
redeem single-use invite → pin client key → host returns a fresh reusable
client-bound credential → save it in `PeerBook`.

### Smallest genuinely useful v1 (combined roadmap)

1. **Stable outbound identity** + reusable client-bound credentials (prereq).
2. `PeerBook` + `peers save/remove/rename`; `connect <name>`.
3. Rename `list` → `connections` (alias kept).
4. `serve_with_policy` + scope enforcement (§9) — prereq for safe delegation.
5. Runtime-injected `roam__list_peers` + `roam__delegate` (one-shot ACP).
6. Dedicated `Delegate` scope; no recursive roaming; bounded execution.

Defer: durable conversations, steering, presence probing, dynamic tools,
remote MCP federation, multi-hop delegation, `grants` UI polish.

### Future idea — remote subagent spawning ("run a subagent somewhere else")

goose already has local **subagents** (spawn a child agent loop for a task).
A natural roaming extension: let an agent spawn a subagent *on a remote node*.
This is really just `roam__delegate` viewed through the subagent lens — the
remote agent IS the subagent — but it raises one important anchoring question:

**Where does the process that holds the connection open and waits for work
live?** Three models, in increasing power/complexity:

1. **Caller-anchored (delegate model, what we're building).** The *caller*
   holds the iroh connection open, opens an ACP session on the remote, sends a
   task, and awaits the result. The remote spawns the work inside that session;
   when the caller disconnects, the work ends. Simple, bounded, no host daemon
   required. This is the v1 `roam__delegate`.

2. **Host-anchored / detached.** The remote host runs a persistent listener
   (a `goose roam serve` daemon) that OWNS the spawned subagent's lifecycle.
   The caller fires a task and may disconnect; the subagent keeps running on
   the host and the caller reconnects later to collect results (by session id).
   Needs: durable sessions + the session coordinator + a resume/collect path
   (roadmap steps 2–3 of §9). This is where "spawn a subagent somewhere else
   and walk away" becomes real.

3. **Broker/relay-anchored fan-out.** A caller spawns subagents across *several*
   remote nodes and aggregates — the roaming analogue of goose's parallel
   subagents. Depends on host-anchored spawning + concurrency limits + the
   loop/cost guardrails already specified (turn caps, deadlines, no recursive
   `roam` injection).

Anchoring decides the failure semantics: caller-anchored work dies with the
caller (safe default); host-anchored work outlives the caller (powerful, needs
durable session ownership + resource accounting on the host so a disconnected
caller can't leave unbounded work running). Recommendation: ship caller-anchored
(1) as `roam__delegate`, then unlock (2) only once durable sessions + the
session coordinator land, reusing the exact same `Delegate` scope, bounded
execution, and no-recursive-roaming guardrails.

---

## Appendix: key file references (continued)

**mesh-llm (../deez, reference implementation)**
- Endpoint bind: `crates/mesh-llm-host-runtime/src/mesh/mod.rs:2110`
- QUIC tuning: `...mesh/mod.rs:2060`
- Relay URLs / map / auth: `...mesh/mod.rs:596`, `:654`
- Accept + ALPN dispatch: `...mesh/mod.rs:6310`, `:6386`
- Dial: `...src/protocol/mod.rs:602`
- Framing: `...src/protocol/mod.rs:607`
- Node key: `crates/mesh-llm-identity/src/node_key.rs`
- Owner keys: `.../keys.rs`; ownership cert: `.../ownership.rs`
- Signed bootstrap token: `crates/mesh-llm-host-runtime/src/mesh/requirements.rs:44`
- Sign-then-encrypt envelope: `crates/mesh-llm-identity/src/envelope.rs`
