# Roaming web client

The **official browser client for goose roam**: a pure-browser web app that
connects to a roaming goose agent — iroh compiled to wasm and run *inside the
browser tab*, relay-only over WebSocket-to-relay. No Tauri, no local bridge
process: the browser tab itself is the roam peer.

The UI is a **React app that reuses goose's reference clients**: `GooseClient`
from `ui/sdk` as the protocol layer, and the desktop app's real components
(`MarkdownContent`, `ToolCallStatusIndicator`, the desktop Tailwind theme) for
rendering — see `webapp/README.md` for the exact reuse surface.

**This feature is optional.** None of it is part of the goose build — the wasm
crate here (`goose-roaming-web`) has its own `[workspace]` table, so
`cargo build` / `test` / `clippy` in the goose workspace never touch it, and
consumers using goose as an SDK/library pay zero cost for it. Building the wasm
needs an extra toolchain (below); nothing else in goose depends on it.

The build output is a **static site**: build it once, host it on any static
host/CDN over HTTPS, and any browser can reach any `goose roam share` host
whose card it holds — all traffic goes browser ⇄ iroh relay ⇄ host,
end-to-end encrypted, never through the site's origin.

## Status: proven end to end ✅

A real headless Chrome, running iroh in wasm, connected to a real
`goose roam share` agent **over the managed iroh relays**, completed the roam
handshake with an accepted key, opened an ACP session, sent a prompt, and got a
live agent response. The whole thesis works. See `webapp/tests/e2e.mjs`.

What's proven, in layers:
- **compiles** — the full iroh 1.0.2 browser transport stack goes to
  `wasm32-unknown-unknown` (iroh + noq/noq-proto/noq-udp QUIC + tokio-websockets
  + ws_stream/wasm-streams + web-sys + wasm-bindgen + ed25519-dalek + ring +
  rustls).
- **bundles** — `tsc` clean against the real ACP SDK 0.19.0; Vite builds a
  static site (1.28 MB gzipped wasm, 27 kB gzipped app+SDK).
- **runs in-browser** — `webapp/tests/smoke.mjs`: wasm instantiates, generates
  an ed25519 keypair live, round-trips a `goose+roam://` card.
- **live round trip** — `webapp/tests/e2e.mjs`: browser ⇄ relay ⇄ `goose roam
  share`, prompt in, response out.

## What's here

```
web/
├── goose-roaming-web/   Rust cdylib: the wasm transport shim
│                        (RoamClient / RoamConnection — identity, card decode,
│                         relay-only dial, roam handshake, byte duplex)
├── webapp/              Vite + TS chat app (see webapp/README.md, webapp/TRYME.md)
├── build-wasm.sh        compile a wasm crate with the right toolchain
└── build-web.sh         build-wasm.sh + wasm-bindgen → webapp/src/wasm/
```

Product code is ~850 hand-written lines (≈400 Rust shim + ≈450 web app); the
heavy lifting is iroh + the ACP SDK.

## How to build the wasm

One-time prereqs:

```bash
rustup toolchain install 1.96.1
rustup target add wasm32-unknown-unknown --toolchain 1.96.1
brew install llvm            # provides a wasm-capable clang
```

Then:

```bash
./build-web.sh               # compiles goose-roaming-web to wasm AND runs
                             # wasm-bindgen → webapp/src/wasm/
```

Override the toolchain / LLVM path via `ROAM_WASM_TOOLCHAIN` / `ROAM_WASM_LLVM`.

### Why it needs a special toolchain (3 non-obvious requirements)

`build-wasm.sh` encodes these; each fails confusingly on its own.

1. **wasm std must belong to the same rustc cargo shells out to.** If PATH puts
   a Homebrew rustc (no wasm std) ahead of the rustup shim, a plain
   `cargo build --target wasm32-...` fails with *"can't find crate for core"*
   even after `rustup target add`. Fix: pin a rustup toolchain that has the
   target and prepend its `bin` to PATH.
2. **`ring` compiles C → needs a wasm-capable clang.** Apple clang can't emit
   wasm32. Point `CC_wasm32_unknown_unknown` / `AR_wasm32_unknown_unknown` at
   Homebrew LLVM. (A pure-Rust crypto backend would drop this dependency —
   worth evaluating.)
3. **getrandom needs the browser backend:** `--cfg getrandom_backend="wasm_js"`
   in RUSTFLAGS + the `wasm_js` feature.

Browser posture also uses `iroh` with `default-features = false` + `tls-ring`
(not the crate's default `tls-aws-lc-rs`, which doesn't build for wasm here) and
drops native-only `portmapper` / `metrics` / `fast-apple-datapath`.

## How to try it

See **`webapp/TRYME.md`** for the full runbook (headless auto-demo *and*
drive-it-yourself-in-a-browser). Short version:

```bash
./build-web.sh                                   # if webapp/src/wasm is missing
cd webapp && ./serve.sh                          # serves on http://localhost:5178
# then: goose roam peers accept '<browser card>' ; goose roam share
# paste the host card into the page, connect, chat.
```

## Architecture

Built here — everything on the main thread for simplicity:

```
Browser tab
├─ goose-roaming-web (wasm): RoamClient / RoamConnection
│     iroh endpoint (relay-only) + roam handshake
│     → RoamConnection.send(bytes) / .recv() → Uint8Array   (bounded channels)
├─ roamByteStreams(): wraps that duplex as Web Streams<Uint8Array>
├─ ndJsonStream(writable, readable) + ClientSideConnection  (ACP SDK 0.19.0)
└─ lean chat UI
```

The key leverage: `@agentclientprotocol/sdk`'s
`ClientSideConnection(toClient, stream)` where
`stream = ndJsonStream(WritableStream<Uint8Array>, ReadableStream<Uint8Array>)`.
That byte-duplex is the *entire* contract the wasm module has to satisfy — the
ACP client (request/response correlation, permission requests, notifications) is
not hand-rolled. `tsc` is clean against the SDK's real `Stream` / `Client`
types.

**Recommended for production:** move the transport + ACP wiring into a **Web
Worker** so QUIC/crypto/JSON don't compete with the UI thread. The seam is
unchanged; only where it runs moves. This first version keeps it on the main thread to
make the first end-to-end path small.

## Deployment shape (why this is interesting)

The built `webapp/dist/` is 4 static files and a wasm blob — **no backend**.
After load, the app never calls its origin again; all real traffic is browser ⇄
relay ⇄ goose host. So it can live on any static host / CDN (must be HTTPS for
`wss://`; no `SharedArrayBuffer`/workers → no COOP/COEP headers needed). The CDN
is dumb code-delivery and never sees session traffic; the relays only ever see
QUIC-TLS ciphertext. The relay addresses are **not** baked into the bundle —
they arrive at runtime in the pasted host card.

## Findings worth carrying upstream

### Trailing-dot relay host (fixed here, but the native card is the root)
The native card emits relay URLs in FQDN form (`https://…iroh.link./`, trailing
dot). The native iroh client tolerates it, but **browsers reject a trailing-dot
host in the TLS SNI**, so `wss://host./relay` fails with `ERR_CONNECTION_CLOSED`
even though the relay is fine (`101 Switching Protocols`, identical to n0's).
Proven with a raw browser `WebSocket`: dot → ERROR, no-dot → OPEN. The wasm
strips the dot at card decode (`strip_trailing_dot_host`). **Better fix:** have
the native card emit the non-FQDN host so no client has to normalize.

### Managed relays register open — which is what makes the browser work
`goose roam` defaults to the four managed relays mesh-llm uses (per-region on
`iroh.link`, provisioned via `services.iroh.computer`), never iroh's shared
public n0 relays. They register **open** (`AccessConfig::Everyone`, no token),
so an HTTPS page can dial them over `wss://` with no auth header — browsers
can't set WebSocket `Authorization` headers, so open relays are exactly what
makes the pure-browser client viable. `GOOSE_ROAM_RELAYS` /
`GOOSE_ROAM_RELAY_TOKEN` override. (These URLs are a personal n0 namespace today;
a real deployment wants a goose/Block-owned relay namespace with an SLA.)

## Open follow-ups (hardening)

- **The browser key is a remote-admin credential.** An accepted key grants the
  host's full ACP surface (tools/shell). It lives in `localStorage`, so a public
  CDN URL wants a dedicated origin + strict CSP (no third-party scripts) and
  short host-side leases. Design before any public deploy.
- **Revocation doesn't close live connections** (`node.rs`): revoke only blocks
  the *next* inbound auth check; an already-open ACP session stays up. Fix
  before shipping remote access.
- **Card fingerprint is 48-bit** (`card.rs`): bump toward 128-bit before relying
  on it for out-of-band QR verification.
- **Version skew**: a CDN can serve an old client against an updated host — the
  roam handshake + ACP need a version-negotiation story.
- **Reconnect semantics**: a dropped connection after `session/prompt` is
  ambiguous about whether tools already ran — design before polishing UI.
- **Backpressure**: `ws_stream_wasm` has an unbounded recv queue; the bounded
  channels here help, but a Web Worker + bounded Web Streams is the real answer.
