# goose-electron-tester-mcp

An MCP server for testing and QA of the Goose Desktop app (aka "Goose Tester" 🦢🔍). Provides four sets of tools:

1. **Electron CDP tools** — connect to the Electron app's Chrome DevTools Protocol endpoint for live console log collection, JS evaluation, and target inspection.
2. **Screenshot & DOM inspection tools** — capture screenshots of the app (full page or specific elements), get DOM snapshots, and extract HTML. Useful for visual QA, bug documentation, and verifying UI changes.
3. **Navigation & interaction tools** — click elements, type text, press keys, navigate, scroll, and wait for selectors. Enables agents to drive the UI for smoke tests, feature testing, and bug reproduction.
4. **Server log tools** — read goosed server logs, MCP extension logs, and the Electron main process log from disk.

## Quick Start

### 1. Build

```bash
cd ui/desktop/goose-electron-tester-mcp
npm install
npx tsc
```

### 2. Start the dev Electron app with remote debugging

```bash
cd ui/desktop
ENABLE_PLAYWRIGHT=1 npm start
```

Or with a custom port:

```bash
ENABLE_PLAYWRIGHT=1 PLAYWRIGHT_DEBUG_PORT=9333 npm start
```

Or use the launcher script:

```bash
# Dev server with Vite hot reload
./tests/e2e/specs/launch-app.sh dev /path/to/ui/desktop 9224

# Bundled/packaged app
./tests/e2e/specs/launch-app.sh app "/path/to/Goose.app" 9223
```

### 3. Add as an extension in Goose

In the production Goose Desktop app, go to **Extensions → Add Custom Extension**:

| Field | Value |
|-------|-------|
| Name | Goose Electron Tester |
| Type | stdio |
| Command | `node` |
| Args | `/path/to/goose-main/ui/desktop/goose-electron-tester-mcp/dist/index.js` |
| Env | `ELECTRON_DEBUG_PORT=9222` |

Or add directly to `~/.config/goose/config.yaml`:

```yaml
gooseelectrontester:
  enabled: true
  name: Goose Electron Tester
  type: stdio
  cmd: node
  args:
    - /path/to/goose-main/ui/desktop/goose-electron-tester-mcp/dist/index.js
  env:
    ELECTRON_DEBUG_PORT: "9222"
```

## Tools

### Electron CDP Tools

These require the dev Electron app to be running with `ENABLE_PLAYWRIGHT=1`.

| Tool | Description |
|------|-------------|
| `electron_connect` | Connect to the Electron app's CDP endpoint and start collecting console logs. Accepts optional `port` and `host` to target different instances. |
| `electron_list_targets` | List all debuggable targets (renderer windows, workers, etc.) |
| `electron_get_logs` | View collected console logs with filtering by `level`, `target_id`, `search`, and pagination via `since` cursor |
| `electron_clear_logs` | Clear the in-memory log buffer |
| `electron_evaluate` | Evaluate JavaScript in a renderer window |
| `electron_version` | Get Electron/Chromium version info |

### Screenshot & DOM Inspection Tools

These require a CDP connection (`electron_connect` first). No extra dependencies — uses CDP's built-in `Page.captureScreenshot`, `DOM`, and `DOMSnapshot` domains.

| Tool | Description |
|------|-------------|
| `electron_screenshot` | Capture a screenshot of a renderer window. Options: `format` (png/jpeg/webp), `quality`, `full_page` (scroll capture), `save_path` (save to file). Returns base64 image inline or saves to disk. |
| `electron_screenshot_element` | Screenshot a specific DOM element by CSS selector. Options: `selector` (required), `padding` (extra px around element), `format`, `save_path`. |
| `electron_dom_snapshot` | Get a structured DOM snapshot with computed styles (display, visibility, colors, sizes, etc). Compact representation for understanding layout without pixels. |
| `electron_get_html` | Get outerHTML of the document or a specific element by CSS selector. Use targeted selectors to keep output manageable. |

**QA workflow example:**
```
1. "Connect to the electron app"              → electron_connect
2. "Screenshot the current state"             → electron_screenshot
3. "Screenshot the chat input area"           → electron_screenshot_element { selector: ".chat-input" }
4. "Save a screenshot for the bug report"     → electron_screenshot { save_path: "/tmp/bug-123.png" }
5. "What's the DOM structure of the sidebar?"  → electron_get_html { selector: ".sidebar" }
```

### Navigation & Interaction Tools

These let agents drive the UI — click buttons, type text, navigate, and wait for transitions. Combined with screenshots, this enables full smoke tests and bug reproduction workflows.

| Tool | Description |
|------|-------------|
| `electron_click` | Click an element by CSS selector (resolves bounding box, clicks center) or at x,y coordinates. Options: `button` (left/right/middle), `click_count` (2 for double-click). |
| `electron_type` | Type text into the focused element or a selector. Options: `clear` (select-all + delete first), `press_enter` (submit after typing). |
| `electron_press_key` | Press a key: Enter, Tab, Escape, Backspace, arrows, F-keys, a-z. Supports modifier bitmask: 1=Alt, 2=Ctrl, 4=Cmd, 8=Shift (e.g., Cmd+A = `key:"a", modifiers:4`). |
| `electron_navigate` | Navigate the renderer to a URL. For SPA in-app navigation, prefer clicking nav elements. |
| `electron_wait_for` | Wait for a CSS selector to appear/become visible. Polls at 200ms. Use after clicks/navigation before screenshotting. |
| `electron_scroll` | Scroll to coordinates or scroll an element into view by selector. |

**Smoke test example:**
```
1. electron_connect
2. electron_screenshot                                    → capture initial state
3. electron_click { selector: "[data-testid='nav-settings']" } → open settings
4. electron_wait_for { selector: ".settings-panel", visible: true }
5. electron_screenshot                                    → capture settings page
6. electron_click { selector: "[data-testid='dark-mode-button']" } → toggle theme
7. electron_screenshot                                    → verify theme changed
8. electron_get_logs { level: "error" }                   → check for console errors
```

**Bug reproduction example:**
```
1. electron_connect
2. electron_screenshot { save_path: "/tmp/before.png" }   → document initial state
3. electron_type { selector: "[data-session-active='true'] textarea", text: "test message", press_enter: true }
4. electron_wait_for { selector: "[data-testid='message-container']" }
5. electron_screenshot { save_path: "/tmp/after.png" }    → document result
6. electron_get_logs { level: "error,warn" }              → check for issues
```

### Server Log Tools

These work anytime — they read log files from disk. No CDP connection required.

| Tool | Description |
|------|-------------|
| `server_list_sessions` | List goosed server log sessions with date, start time, and size. Each session = one goosed process. |
| `server_get_logs` | Read a session's server logs. Filter by `level` (TRACE/DEBUG/INFO/WARN/ERROR), `module` prefix, `search` text. Supports `head`/`tail`. |
| `server_list_mcp_logs` | List available MCP extension log files |
| `server_get_mcp_log` | Read an MCP extension's log by name (e.g., "developer") |
| `electron_get_main_log` | Read the Electron main process log (main.log) |

## Log Locations

| Log | Path |
|-----|------|
| Goosed server | `~/.local/state/goose/logs/server/YYYY-MM-DD/YYYYMMDD_HHMMSS-goosed.log` |
| MCP extensions | `~/.local/state/goose/logs/mcps/mcp_<name>.log` |
| Electron main | `~/Library/Application Support/Goose/logs/main.log` |

## Multiple Instances

Start each dev instance on a different port:

```bash
ENABLE_PLAYWRIGHT=1 PLAYWRIGHT_DEBUG_PORT=9222 npm start   # Instance 1
ENABLE_PLAYWRIGHT=1 PLAYWRIGHT_DEBUG_PORT=9223 npm start   # Instance 2
```

Then switch between them:

```
"Connect to the electron app on port 9222"   → attaches to instance 1
"Connect to the electron app on port 9223"   → disconnects from 1, attaches to 2
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ELECTRON_DEBUG_PORT` | `9222` | Default CDP port |
| `ELECTRON_DEBUG_HOST` | `127.0.0.1` | Default CDP host |

## Architecture

```
src/
├── index.ts        # MCP server — tool definitions and handlers
├── cdp-client.ts   # Raw CDP WebSocket client for Electron (console, screenshots, DOM)
├── log-reader.ts   # File-based log reader for goosed/MCP/Electron logs
└── session-db.ts   # SQLite reader for goose sessions.db
```

The CDP client connects directly to Electron's remote debugging endpoint using raw WebSocket (no Puppeteer). It discovers targets via `http://<host>:<port>/json/list`, attaches to each via WebSocket, and uses the `Runtime`, `Log`, `Page`, `DOM`, `DOMSnapshot`, and `Input` CDP domains. This provides console log collection, screenshot capture, element inspection, DOM snapshots, click/type/key input, and navigation — all through the same WebSocket connection with zero additional dependencies.

The log reader uses simple file I/O to read goosed server logs (tracing format), MCP extension logs, and the Electron main.log.
