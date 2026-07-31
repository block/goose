# goose roam — web client (spike)

A lean browser chat client that connects to a `goose roam share` agent **over
iroh, entirely in the browser** (iroh compiled to wasm, relay-only via
WebSocket-to-relay). No Tauri, no Electron, no local bridge process — the
browser tab itself is the roam peer.

## The stack (all in the tab)

```
iroh (wasm, relay-only) ── roam handshake ──► authorized ACP byte duplex
     │  goose_roaming_web.wasm (RoamClient / RoamConnection)
     ▼  roamByteStreams()
Web Streams <Uint8Array>
     ▼  ndJsonStream()                (@agentclientprotocol/sdk)
Stream<AnyMessage>
     ▼  new ClientSideConnection(client, stream)
typed ACP: initialize / newSession / prompt / sessionUpdate / requestPermission
     ▼
lean chat UI
```

The wasm module (`../goose-roaming-web`) does **only** the transport: hold a
roam identity keypair, decode a `goose+roam://` card, dial relay-only, run the
roam handshake, and expose a byte duplex. Everything ACP-shaped is the existing
TypeScript SDK. Nothing about the protocol is hand-rolled.

## Build & run

```bash
# 1. build the wasm transport module + generate JS bindings
cd ../..                       # repo root not required; script is self-locating
./crates/goose-roaming/web-interface-spike/build-web.sh

# 2. run the app
cd crates/goose-roaming/web-interface-spike/webapp
pnpm install
pnpm dev                       # http://localhost:5178
```

`build-web.sh` compiles `goose-roaming-web` to wasm (via `build-wasm.sh`) and
runs `wasm-bindgen --target web` into `webapp/src/wasm/`.

## Pairing (two-way, like the CLI)

1. Open the app. It generates a per-browser roam identity (persisted in
   `localStorage`) and shows **this browser's key**.
2. On the host: `goose roam peers accept <that key>` (one time).
3. On the host: `goose roam id` → copy the `goose+roam://…` card.
4. Paste the card into the app → **connect**.

Both sides have chosen to trust the other's key — the same mutual card-swap two
CLIs do. The host runs the real agent (its tools, shell, cwd); the browser is a
pure ACP client.

## Smoke test (proves the wasm runs in a browser)

```bash
pnpm dev                       # in one shell (serves on :5178)
node tests/smoke.mjs           # in another
```

`tests/smoke.mjs` drives headless Google Chrome via Playwright (uses the
system Chrome via `channel: "chrome"`, so no `playwright install` needed) and
asserts the wasm instantiates, generates an ed25519 keypair, round-trips a
`goose+roam://` card through the decoder, and persists identity — with no
console errors. This isolates "does the browser wasm run" from the live
relay/handshake path.

## Status

Spike. Build-time green (compiles, `tsc` clean vs ACP SDK 0.19.0, Vite bundles)
and **in-browser wasm runtime green** (smoke test). Still unproven: a live
round trip against a running `goose roam share`. Runtime hardening (Web Worker,
reconnect, backpressure tuning, key security, revocation-closes-connections) is
tracked in `../README.md`.
