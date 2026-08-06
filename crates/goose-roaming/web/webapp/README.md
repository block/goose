# goose roam — web client

A browser chat client that connects to a `goose roam share` agent **over iroh,
entirely in the browser** (iroh compiled to wasm, relay-only via
WebSocket-to-relay). No Tauri, no Electron, no local bridge process — the
browser tab itself is the roam peer.

The UI is a **React app that reuses goose's reference clients directly**:

- **`@aaif/goose-sdk`** (`ui/sdk`, via vite alias to its built dist) —
  `GooseClient` is the protocol layer; our roam byte-duplex is exactly the
  `Stream` it expects.
- **`@desktop/*`** (`ui/desktop/src`, imported as source) — the desktop's real
  `MarkdownContent` (react-markdown + remark-gfm + katex + syntax highlighting)
  renders agent messages; `ToolCallStatusIndicator` provides tool status dots;
  the desktop's full Tailwind v4 theme (`styles/main.css`) is imported so the
  components carry their real styles. A ~10-line `window.electron` shim
  (`shim.ts`) covers the Electron-only APIs `MarkdownContent` calls
  (`openExternal` → `window.open`), and `IntlProvider` feeds it the desktop's
  compiled English message catalog.

Plus app-local widgets: tool-call cards that update in place, collapsible
thinking blocks, a plan checklist, inline non-blocking permission cards (never
`window.confirm`, which would freeze the ACP message pump), and a session
sidebar (list / load-with-history-replay / new).

Still stateless + CDN-hostable: static files only, no backend, all traffic
browser ⇄ relay ⇄ roam host. Still deliberately lean: main thread only, text
prompts, no reconnect. Hardening is tracked in `../README.md`.

## The stack (all in the tab)

```
iroh (wasm, relay-only) ── roam handshake ──► authorized ACP byte duplex
     │  goose_roaming_web.wasm (RoamClient / RoamConnection)
     ▼  roamByteStreams()
Web Streams <Uint8Array>
     ▼  ndJsonStream()                (@agentclientprotocol/sdk)
Stream<AnyMessage>
     ▼  new GooseClient(client, stream)        (@aaif/goose-sdk — ui/sdk)
typed ACP: initialize / listSessions / newSession / loadSession / prompt
           / sessionUpdate / requestPermission
     ▼
React chat UI reusing ui/desktop components
(MarkdownContent · ToolCallStatusIndicator · desktop Tailwind theme)
```

The wasm module (`../goose-roaming-web`) does **only** the transport: hold a
roam identity keypair, decode a `goose+roam://` card, dial relay-only, run the
roam handshake, and expose a byte duplex. Everything ACP-shaped is the existing
TypeScript SDK. Nothing about the protocol is hand-rolled.

## Build & run

```bash
# 1. build the wasm transport module + generate JS bindings
cd ../..                       # repo root not required; script is self-locating
./crates/goose-roaming/web/build-web.sh

# 2. run the app
cd crates/goose-roaming/web/webapp
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

Proven end to end. Build-time green (`tsc` clean vs ACP SDK 0.19.0,
Vite bundles), in-browser wasm runtime green (`tests/smoke.mjs`), and a **live
round trip green** (`tests/e2e.mjs`: real Chrome → managed relay → running
`goose roam share` → agent response). `tests/visual.mjs` captures the rendered
GUI (markdown + tool widget + session sidebar) as a screenshot.

Hardening tracked in `../README.md`: Web Worker, reconnect, backpressure
tuning, key security, revocation-closes-connections.
