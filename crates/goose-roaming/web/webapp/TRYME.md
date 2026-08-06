# Try the roaming web client

Two ways to see it work. Both are real (real browser, real `goose roam share`,
real managed relays). No Tauri.

Prereqs: a built goose binary (`cargo build -p goose-cli` →
`target/debug/goose`) and a provider configured (you have Anthropic). The wasm
bindings are already built into `src/wasm/`; rebuild with `../build-web.sh` if
needed.

---

## A) Fastest: watch the automated proof (headless, ~30s)

Drives the whole flow in headless Chrome and prints each step.

```bash
cd crates/goose-roaming/web/webapp
ln -sfn ../../../../ui/node_modules node_modules        # offline deps (registry down)
./serve.sh &                                            # serves on :5178
GOOSE_BIN="$PWD/../../../../target/debug/goose" node tests/e2e.mjs
```

You'll see: share goes live → browser identity → accept → **CONNECTED THROUGH
RELAY** → **AGENT RESPONDED**. A screenshot lands at `/tmp/roam-e2e.png`.

---

## B) Drive it yourself in a real browser (see it with your eyes)

Two terminals.

### Terminal 1 — serve the web app
```bash
cd crates/goose-roaming/web/webapp
./serve.sh
```
Open <http://localhost:5178>. The page shows **this browser's card**
(`goose+roam://…`) and its key. Copy the card (there's a copy button).

### Terminal 2 — accept your browser, then share an agent
```bash
cd /Users/micn/Development/goose
G=./target/debug/goose

# 1. let this browser connect (paste the card you copied, keep the quotes)
$G roam peers accept 'goose+roam://…PASTE_BROWSER_CARD…'

# 2. start sharing an agent (runs in the dir you start it in). This blocks and
#    prints the HOST card:
$G roam share
```
Copy the `goose+roam://…` **host** card it prints.

### Back in the browser
Paste the **host** card into the box, hit **connect**. You should see
"connected to …", then type a message and watch the agent stream back.

Stop the host with Ctrl-C in Terminal 2.

---

### Notes / gotchas
- **Order matters**: accept the browser key *before* (or during) `roam share` —
  the live share re-reads the allowlist per connection, so accepting after it's
  running also works.
- The browser generates a stable identity (persisted in `localStorage`), so you
  only `accept` it once per browser profile.
- If connect hangs: the managed relays must be reachable (they're
  `*.relay.michaelneale.mesh-llm.iroh.link`). The web client strips the card's
  trailing-dot relay host so the browser TLS/SNI is happy.
- Current limitations: main-thread (no worker), no reconnect, key in localStorage.
