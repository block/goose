#!/usr/bin/env node

/**
 * goose-electron-tester-mcp
 *
 * An MCP server for debugging the Goose Electron app and its Rust backend.
 *
 * Provides two sets of tools:
 *   1. Electron CDP tools — connect to Electron's Chrome DevTools Protocol
 *      endpoint for live console logs, JS evaluation, and target inspection.
 *   2. Server log tools — read goosed server logs, MCP extension logs,
 *      and the Electron main process log from disk.
 *
 * Usage:
 *   1. Start the Goose Electron app with remote debugging enabled:
 *        ENABLE_PLAYWRIGHT=1 npm start
 *        ENABLE_PLAYWRIGHT=1 PLAYWRIGHT_DEBUG_PORT=9333 npm start
 *
 *   2. Add this MCP server to your goose config as a stdio extension:
 *        Command: node
 *        Args:    <path-to-this-repo>/dist/index.js
 *        Env:     ELECTRON_DEBUG_PORT=9222
 *
 *   Environment variables:
 *     ELECTRON_DEBUG_PORT  – default CDP port (default: 9222)
 *     ELECTRON_DEBUG_HOST  – default CDP host (default: 127.0.0.1)
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { CDPClient } from "./cdp-client.js";
import type { ConsoleEntry } from "./cdp-client.js";
import { execSync } from "child_process";
import { existsSync, writeFileSync, chmodSync } from "fs";
import {
  listServerSessions,
  readServerLog,
  listMcpLogs,
  readMcpLog,
  readElectronMainLog,
  SERVER_LOGS_DIR,
} from "./log-reader.js";
import {
  listChatSessions,
  findSession,
  getCurrentSession,
  type ChatSession,
} from "./session-db.js";

const DEFAULT_PORT = parseInt(process.env.ELECTRON_DEBUG_PORT ?? "9222", 10);
const DEFAULT_HOST = process.env.ELECTRON_DEBUG_HOST ?? "127.0.0.1";

let cdp = new CDPClient(DEFAULT_PORT, DEFAULT_HOST);

// ── App launcher helpers ────────────────────────────────────────────

function isPortListening(port: number): boolean {
  try {
    const result = execSync(`lsof -i :${port} -sTCP:LISTEN -t 2>/dev/null`, { encoding: "utf-8" });
    return result.trim().length > 0;
  } catch {
    return false;
  }
}

async function waitForPort(port: number, timeoutMs: number = 30000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (isPortListening(port)) return true;
    await new Promise((r) => setTimeout(r, 500));
  }
  return false;
}

function launchDevServer(projectDir: string, port: number): string {
  if (!existsSync(`${projectDir}/package.json`)) {
    throw new Error(`No package.json found in ${projectDir}`);
  }

  if (isPortListening(port)) {
    return `Port ${port} is already in use. An app may already be running. Use electron_connect to attach.`;
  }

  // Write a .command file and open it — macOS opens .command files in Terminal.app
  // which gives the process full WindowServer access for GUI windows.
  // This is the only reliable way to launch Electron with visible windows
  // from a background process on macOS.
  const cmdFile = `/tmp/goose-tester-dev-${port}.command`;
  writeFileSync(cmdFile, `#!/bin/bash
cd "${projectDir}"
export ENABLE_PLAYWRIGHT=1
export PLAYWRIGHT_DEBUG_PORT=${port}
exec npm run start-gui
`);
  chmodSync(cmdFile, 0o755);
  execSync(`open "${cmdFile}"`);

  return cmdFile;
}

function launchBundledApp(appPath: string, port: number): void {
  if (!existsSync(appPath)) {
    throw new Error(`App not found: ${appPath}`);
  }

  if (isPortListening(port)) {
    throw new Error(`Port ${port} is already in use. An app may already be running. Use electron_connect to attach.`);
  }

  execSync(`open -a "${appPath}" --args --remote-debugging-port=${port}`);
}
let activePort = DEFAULT_PORT;
let activeHost = DEFAULT_HOST;

// ── Formatting helpers ──────────────────────────────────────────────

function formatEntry(e: ConsoleEntry): string {
  const ts = new Date(e.timestamp).toISOString().slice(11, 23);
  const loc = e.url ? ` (${e.url}${e.lineNumber !== undefined ? `:${e.lineNumber}` : ""})` : "";
  const levelTag = e.level.toUpperCase().padEnd(7);
  const src = e.source === "exception" ? " ⚠ EXCEPTION" : "";
  return `[${ts}] ${levelTag} [${e.targetTitle}]${src} ${e.text}${loc}`;
}

function formatEntries(entries: ConsoleEntry[]): string {
  if (entries.length === 0) return "No console entries found.";
  return entries.map(formatEntry).join("\n");
}

async function resolveTargetId(targetId?: string): Promise<string> {
  if (targetId) return targetId;
  const attachedIds = cdp.getAttachedTargetIds();
  if (attachedIds.length === 0) {
    throw new Error("Not attached to any targets. Call electron_connect first.");
  }
  try {
    const targets = await cdp.listTargets();
    const pageTarget = targets.find((t) => t.type === "page" && attachedIds.includes(t.id));
    return pageTarget?.id ?? attachedIds[0];
  } catch {
    return attachedIds[0];
  }
}

// ── MCP Server ──────────────────────────────────────────────────────

const server = new Server(
  { name: "goose-electron-tester-mcp", version: "1.3.0" },
  { capabilities: { tools: {} } }
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [
    // ── App launcher tools ─────────────────────────────────────
    {
      name: "electron_launch_dev",
      description:
        "Launch the Goose Electron dev server from a project directory with remote debugging enabled, then auto-connect. Waits for the app to start before connecting.",
      inputSchema: {
        type: "object" as const,
        properties: {
          project_dir: {
            type: "string",
            description: "Path to the ui/desktop directory (must contain package.json). Example: /Users/zane/Development/goose/ui/desktop",
          },
          port: {
            type: "number",
            description: `CDP debug port (default: ${DEFAULT_PORT})`,
          },
        },
        required: ["project_dir"],
      },
    },
    {
      name: "electron_launch_app",
      description:
        "Launch a bundled/packaged Goose .app with remote debugging enabled, then auto-connect. Works with any .app bundle. Waits for the app to start before connecting.",
      inputSchema: {
        type: "object" as const,
        properties: {
          app_path: {
            type: "string",
            description: 'Path to the .app bundle. Example: /Users/zane/Downloads/Goose.app or "/Applications/Goose.app"',
          },
          port: {
            type: "number",
            description: `CDP debug port (default: ${DEFAULT_PORT})`,
          },
        },
        required: ["app_path"],
      },
    },

    {
      name: "electron_stop",
      description:
        "Stop a running Goose Electron app by killing the process on the debug port. Cleans up screen sessions for dev servers. Use before re-launching.",

      inputSchema: {
        type: "object" as const,
        properties: {
          port: {
            type: "number",
            description: `CDP debug port to stop (default: ${DEFAULT_PORT})`,
          },
        },
      },
    },

    // ── Electron CDP tools ──────────────────────────────────────
    {
      name: "electron_connect",
      description:
        "Connect to the Electron app's CDP endpoint and start collecting console logs from all targets. Call this first. If already connected, disconnects and reconnects (useful to pick up new targets or switch to a different instance).",
      inputSchema: {
        type: "object" as const,
        properties: {
          port: {
            type: "number",
            description: `CDP port (default: ${DEFAULT_PORT}). Use different ports for different Electron instances.`,
          },
          host: {
            type: "string",
            description: `CDP host (default: ${DEFAULT_HOST})`,
          },
        },
      },
    },
    {
      name: "electron_list_targets",
      description:
        "List all debuggable targets in the Electron app (renderer windows, background pages, service workers, etc).",
      inputSchema: { type: "object" as const, properties: {} },
    },
    {
      name: "electron_get_logs",
      description:
        "Get collected console logs from the Electron app. Supports filtering by level, target, text search, and pagination via 'since' cursor.",
      inputSchema: {
        type: "object" as const,
        properties: {
          level: {
            type: "string",
            description: "Filter by level(s), comma-separated: log, warn, error, info, debug, verbose",
          },
          target_id: {
            type: "string",
            description: "Filter to a specific target ID (from electron_list_targets)",
          },
          search: {
            type: "string",
            description: "Filter entries containing this text (case-insensitive)",
          },
          since: {
            type: "number",
            description: "Only entries with id > this value. Use last entry's id as cursor for polling.",
          },
          limit: {
            type: "number",
            description: "Max entries to return (default: 100, from the end)",
          },
        },
      },
    },
    {
      name: "electron_clear_logs",
      description: "Clear all collected console log entries.",
      inputSchema: { type: "object" as const, properties: {} },
    },
    {
      name: "electron_evaluate",
      description:
        "Evaluate a JavaScript expression in a specific Electron renderer window. Returns the result.",
      inputSchema: {
        type: "object" as const,
        properties: {
          target_id: {
            type: "string",
            description: "Target ID (from electron_list_targets). Defaults to the first attached page.",
          },
          expression: {
            type: "string",
            description: "JavaScript expression to evaluate",
          },
        },
        required: ["expression"],
      },
    },
    {
      name: "electron_version",
      description: "Get Electron/Chromium version info from the running app.",
      inputSchema: { type: "object" as const, properties: {} },
    },

    // ── Screenshot & DOM inspection tools ───────────────────────
    {
      name: "electron_screenshot",
      description:
        "Capture a screenshot of an Electron renderer window. Returns a base64-encoded PNG (or JPEG/WebP). " +
        "Use this to visually inspect the current state of the UI, verify layout, or document bugs.",
      inputSchema: {
        type: "object" as const,
        properties: {
          target_id: {
            type: "string",
            description: "Target ID (from electron_list_targets). Defaults to the first attached page.",
          },
          format: {
            type: "string",
            enum: ["png", "jpeg", "webp"],
            description: "Image format (default: png)",
          },
          quality: {
            type: "number",
            description: "Compression quality 0-100 (only for jpeg/webp, default: 80)",
          },
          full_page: {
            type: "boolean",
            description: "Capture the full scrollable page, not just the viewport (default: false)",
          },
          save_path: {
            type: "string",
            description: "Optional: save the screenshot to this file path instead of returning inline. Parent directories are created automatically.",
          },
        },
      },
    },
    {
      name: "electron_screenshot_element",
      description:
        "Capture a screenshot of a specific DOM element by CSS selector. " +
        "Useful for focusing on a particular component (e.g., '.chat-message:last-child', '#settings-panel').",
      inputSchema: {
        type: "object" as const,
        properties: {
          selector: {
            type: "string",
            description: "CSS selector for the element to capture",
          },
          target_id: {
            type: "string",
            description: "Target ID (from electron_list_targets). Defaults to the first attached page.",
          },
          format: {
            type: "string",
            enum: ["png", "jpeg", "webp"],
            description: "Image format (default: png)",
          },
          quality: {
            type: "number",
            description: "Compression quality 0-100 (only for jpeg/webp)",
          },
          padding: {
            type: "number",
            description: "Extra pixels around the element (default: 0)",
          },
          save_path: {
            type: "string",
            description: "Optional: save to file instead of returning inline",
          },
        },
        required: ["selector"],
      },
    },
    {
      name: "electron_dom_snapshot",
      description:
        "Get a structured DOM snapshot with computed styles. Returns a compact representation of the page's DOM tree " +
        "with layout info — useful for understanding UI structure without needing to see pixels. " +
        "Much smaller than full outerHTML.",
      inputSchema: {
        type: "object" as const,
        properties: {
          target_id: {
            type: "string",
            description: "Target ID (from electron_list_targets). Defaults to the first attached page.",
          },
          computed_styles: {
            type: "array",
            items: { type: "string" },
            description: "CSS properties to include (default: display, visibility, opacity, color, background-color, font-size, font-weight, width, height, overflow)",
          },
        },
      },
    },
    {
      name: "electron_get_html",
      description:
        "Get the outerHTML of the document or a specific element. " +
        "Use a selector to narrow down to a specific component (e.g., 'main', '.sidebar', '#chat-container').",
      inputSchema: {
        type: "object" as const,
        properties: {
          selector: {
            type: "string",
            description: "CSS selector (default: returns full document HTML). Use specific selectors to keep output manageable.",
          },
          target_id: {
            type: "string",
            description: "Target ID (from electron_list_targets). Defaults to the first attached page.",
          },
        },
      },
    },

    // ── Navigation & interaction tools ──────────────────────────
    {
      name: "electron_click",
      description:
        "Click on an element by CSS selector, or at specific x,y coordinates. " +
        "When using a selector, resolves the element's bounding box and clicks its center. " +
        "Returns the click coordinates for verification.",
      inputSchema: {
        type: "object" as const,
        properties: {
          selector: {
            type: "string",
            description: "CSS selector of element to click (e.g., '.settings-btn', '#submit', 'button[data-testid=\"send\"]')",
          },
          x: {
            type: "number",
            description: "X coordinate (used when no selector is provided)",
          },
          y: {
            type: "number",
            description: "Y coordinate (used when no selector is provided)",
          },
          button: {
            type: "string",
            enum: ["left", "right", "middle"],
            description: "Mouse button (default: left)",
          },
          click_count: {
            type: "number",
            description: "Number of clicks (2 for double-click, default: 1)",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
      },
    },
    {
      name: "electron_type",
      description:
        "Type text into the currently focused element, or focus a selector first then type. " +
        "Each character is dispatched as individual keyDown/keyUp events, simulating real keyboard input.",
      inputSchema: {
        type: "object" as const,
        properties: {
          text: {
            type: "string",
            description: "Text to type",
          },
          selector: {
            type: "string",
            description: "CSS selector to focus before typing (optional — types into currently focused element if omitted)",
          },
          clear: {
            type: "boolean",
            description: "Select all and delete existing content before typing (default: false)",
          },
          press_enter: {
            type: "boolean",
            description: "Press Enter after typing (default: false)",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
        required: ["text"],
      },
    },
    {
      name: "electron_press_key",
      description:
        "Press a keyboard key (Enter, Tab, Escape, Backspace, arrow keys, etc). " +
        "Supports modifier keys via the modifiers bitmask: 1=Alt, 2=Ctrl, 4=Meta/Cmd, 8=Shift. " +
        "Example: Cmd+A = key:'a', modifiers:4",
      inputSchema: {
        type: "object" as const,
        properties: {
          key: {
            type: "string",
            description: "Key name: Enter, Tab, Escape, Backspace, Delete, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Home, End, Space, a-z, F1-F12",
          },
          modifiers: {
            type: "number",
            description: "Modifier bitmask: 1=Alt, 2=Ctrl, 4=Meta/Cmd, 8=Shift. Combine with addition (e.g., Ctrl+Shift = 10)",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
        required: ["key"],
      },
    },
    {
      name: "electron_navigate",
      description:
        "Navigate the renderer to a URL. For in-app navigation in a single-page app, " +
        "prefer electron_click on nav elements or electron_evaluate with router APIs.",
      inputSchema: {
        type: "object" as const,
        properties: {
          url: {
            type: "string",
            description: "URL to navigate to",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
        required: ["url"],
      },
    },
    {
      name: "electron_wait_for",
      description:
        "Wait for a CSS selector to appear (or become visible) in the DOM. " +
        "Polls at 200ms intervals. Returns true if found within timeout, false otherwise. " +
        "Use this after clicks/navigation to wait for UI transitions to complete before screenshotting.",
      inputSchema: {
        type: "object" as const,
        properties: {
          selector: {
            type: "string",
            description: "CSS selector to wait for",
          },
          timeout: {
            type: "number",
            description: "Max wait time in milliseconds (default: 10000)",
          },
          visible: {
            type: "boolean",
            description: "Wait for the element to be visible (not just in DOM). Default: false",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
        required: ["selector"],
      },
    },
    {
      name: "electron_scroll",
      description:
        "Scroll the page to specific coordinates or scroll an element into view by selector.",
      inputSchema: {
        type: "object" as const,
        properties: {
          selector: {
            type: "string",
            description: "CSS selector of element to scroll into view",
          },
          x: {
            type: "number",
            description: "Scroll to X position (used when no selector)",
          },
          y: {
            type: "number",
            description: "Scroll to Y position (used when no selector)",
          },
          target_id: {
            type: "string",
            description: "Target ID. Defaults to the first attached page.",
          },
        },
      },
    },

    // ── Server log tools ────────────────────────────────────────
    {
      name: "server_list_sessions",
      description:
        "List goosed server log sessions. Each session is a separate goosed process that was started (one per Electron window). Shows date, start time, file size, and path. Use this to find the session you want to inspect.",
      inputSchema: {
        type: "object" as const,
        properties: {
          date: {
            type: "string",
            description: "Filter to a specific date (YYYY-MM-DD). Default: show all recent dates.",
          },
          limit: {
            type: "number",
            description: "Max sessions to return (default: 20)",
          },
        },
      },
    },
    {
      name: "server_get_logs",
      description:
        "Read goosed server logs for a specific session. Shows tracing output with timestamps, levels, modules, and messages. By default reads the most recent session's last 200 lines.",
      inputSchema: {
        type: "object" as const,
        properties: {
          session: {
            type: "string",
            description:
              "Session identifier — the start time prefix (e.g., '20260212_153402') or full filename. From server_list_sessions. Defaults to most recent.",
          },
          date: {
            type: "string",
            description: "Date to look in (YYYY-MM-DD). Helps narrow down which session to find.",
          },
          tail: {
            type: "number",
            description: "Read last N lines (default: 200)",
          },
          head: {
            type: "number",
            description: "Read first N lines instead of tail",
          },
          level: {
            type: "string",
            description: "Filter by level(s), comma-separated: TRACE, DEBUG, INFO, WARN, ERROR",
          },
          search: {
            type: "string",
            description: "Text search (case-insensitive)",
          },
          module: {
            type: "string",
            description: "Filter by module prefix (e.g., 'goose::agent', 'goosed::routes')",
          },
        },
      },
    },
    {
      name: "server_list_mcp_logs",
      description:
        "List available MCP extension log files. Each extension (developer, memory, etc.) writes to its own log file.",
      inputSchema: { type: "object" as const, properties: {} },
    },
    {
      name: "server_get_mcp_log",
      description:
        "Read an MCP extension's log file. Provide the extension name (e.g., 'developer') or full filename.",
      inputSchema: {
        type: "object" as const,
        properties: {
          name: {
            type: "string",
            description: "Extension name (e.g., 'developer', 'codesearch') or full filename (e.g., 'mcp_developer.log')",
          },
          tail: {
            type: "number",
            description: "Read last N lines (default: 200)",
          },
          search: {
            type: "string",
            description: "Text search (case-insensitive)",
          },
        },
        required: ["name"],
      },
    },
    {
      name: "electron_get_main_log",
      description:
        "Read the Electron main process log (main.log). Contains stdout/stderr from the Electron main process, including goosed spawn messages and general app lifecycle events.",
      inputSchema: {
        type: "object" as const,
        properties: {
          tail: {
            type: "number",
            description: "Read last N lines (default: 100)",
          },
          search: {
            type: "string",
            description: "Text search (case-insensitive)",
          },
          level: {
            type: "string",
            description: "Filter by level: error, warn, info",
          },
        },
      },
    },

    // ── Chat session tools (from sessions.db) ───────────────────
    {
      name: "server_list_chat_sessions",
      description:
        "List chat sessions from the sessions database. Shows session names, IDs, token counts, and status. Use this to find a session by name, then use server_get_chat_session_logs to read its server logs.",
      inputSchema: {
        type: "object" as const,
        properties: {
          search: {
            type: "string",
            description: "Search sessions by name (case-insensitive, fuzzy match)",
          },
          limit: {
            type: "number",
            description: "Max sessions to return (default: 30)",
          },
          active_only: {
            type: "boolean",
            description: "Only show currently active (in_use) sessions",
          },
        },
      },
    },
    {
      name: "server_get_chat_session_logs",
      description:
        "Read goosed server logs for a chat session, looked up by session name or ID from the sessions database. " +
        "Finds the session in the DB, locates the server log file that contains it, and filters to only show log lines for that session. " +
        "If no session_name or session_id is provided, uses the current active session.",
      inputSchema: {
        type: "object" as const,
        properties: {
          session_name: {
            type: "string",
            description: "Session name to search for (case-insensitive, fuzzy match). E.g. 'missing windows release'",
          },
          session_id: {
            type: "string",
            description: "Exact session ID from the database (e.g. '20260213_4'). Alternative to session_name.",
          },
          tail: {
            type: "number",
            description: "Read last N lines (default: 500, reads more since we filter by session)",
          },
          level: {
            type: "string",
            description: "Filter by level(s), comma-separated: TRACE, DEBUG, INFO, WARN, ERROR",
          },
          search: {
            type: "string",
            description: "Additional text search within the session's logs (case-insensitive)",
          },
        },
      },
    },
  ],
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  switch (name) {
    // ═══════════════════════════════════════════════════════════════
    // App launcher tools
    // ═══════════════════════════════════════════════════════════════

    case "electron_launch_dev": {
      const projectDir = args?.project_dir as string;
      const port = (args?.port as number | undefined) ?? DEFAULT_PORT;

      try {
        const result = launchDevServer(projectDir, port);

        if (result.startsWith("Port")) {
          // Already running — just connect
          cdp.disconnectAll();
          cdp = new CDPClient(port, DEFAULT_HOST);
          activePort = port;
          activeHost = DEFAULT_HOST;
          const targets = await cdp.attachAll();
          const summary = targets.map((t) => `  • [${t.type}] ${t.title} (${t.id})\n    ${t.url}`).join("\n");
          return {
            content: [{ type: "text", text: `${result}\n\nConnected to existing app on port ${port}.\nAttached to ${targets.length} target(s):\n\n${summary}` }],
          };
        }

        const ready = await waitForPort(port, 30000);
        if (!ready) {
          return {
            content: [{ type: "text", text: `Dev server launched but port ${port} didn't start listening within 30s.\n\nCheck logs: cat /tmp/goose-tester-dev-${port}.log` }],
            isError: true,
          };
        }

        // Auto-connect
        cdp.disconnectAll();
        cdp = new CDPClient(port, DEFAULT_HOST);
        activePort = port;
        activeHost = DEFAULT_HOST;

        // Give the renderer a moment to initialize
        await new Promise((r) => setTimeout(r, 2000));

        const targets = await cdp.attachAll();
        const summary = targets.map((t) => `  • [${t.type}] ${t.title} (${t.id})\n    ${t.url}`).join("\n");

        return {
          content: [{
            type: "text",
            text: `Dev server launched and connected!\n\n  Project: ${projectDir}\n  Port: ${port}\n\nAttached to ${targets.length} target(s):\n\n${summary}\n\nConsole logs are now being collected.`,
          }],
        };
      } catch (e) {
        return {
          content: [{ type: "text", text: `Failed to launch dev server: ${e instanceof Error ? e.message : String(e)}` }],
          isError: true,
        };
      }
    }

    case "electron_launch_app": {
      const appPath = args?.app_path as string;
      const port = (args?.port as number | undefined) ?? DEFAULT_PORT;

      try {
        if (isPortListening(port)) {
          // Already running — just connect
          cdp.disconnectAll();
          cdp = new CDPClient(port, DEFAULT_HOST);
          activePort = port;
          activeHost = DEFAULT_HOST;
          const targets = await cdp.attachAll();
          const summary = targets.map((t) => `  • [${t.type}] ${t.title} (${t.id})\n    ${t.url}`).join("\n");
          return {
            content: [{ type: "text", text: `Port ${port} already in use. Connected to existing app.\nAttached to ${targets.length} target(s):\n\n${summary}` }],
          };
        }

        launchBundledApp(appPath, port);

        const ready = await waitForPort(port, 15000);
        if (!ready) {
          return {
            content: [{ type: "text", text: `App launched but port ${port} didn't start listening within 15s.\n\nThe app may not support remote debugging, or it may have started on a different port.` }],
            isError: true,
          };
        }

        // Auto-connect
        cdp.disconnectAll();
        cdp = new CDPClient(port, DEFAULT_HOST);
        activePort = port;
        activeHost = DEFAULT_HOST;

        // Give the renderer a moment to initialize
        await new Promise((r) => setTimeout(r, 2000));

        const targets = await cdp.attachAll();
        const summary = targets.map((t) => `  • [${t.type}] ${t.title} (${t.id})\n    ${t.url}`).join("\n");

        return {
          content: [{
            type: "text",
            text: `Bundled app launched and connected!\n\n  App: ${appPath}\n  Port: ${port}\n\nAttached to ${targets.length} target(s):\n\n${summary}\n\nConsole logs are now being collected.`,
          }],
        };
      } catch (e) {
        return {
          content: [{ type: "text", text: `Failed to launch app: ${e instanceof Error ? e.message : String(e)}` }],
          isError: true,
        };
      }
    }

    case "electron_stop": {
      const port = (args?.port as number | undefined) ?? DEFAULT_PORT;
      const messages: string[] = [];

      // Disconnect CDP
      cdp.disconnectAll();
      messages.push("Disconnected CDP client.");

      // Kill process on port
      try {
        const pid = execSync(`lsof -ti :${port} -sTCP:LISTEN 2>/dev/null`, { encoding: "utf-8" }).trim();
        if (pid) {
          // Kill the process group (negative PID) to get all children
          try { execSync(`kill -- -${pid} 2>/dev/null`); } catch { /* ignore */ }
          execSync(`kill ${pid} 2>/dev/null`);
          messages.push(`Killed process ${pid} on port ${port}.`);
          // Wait a moment, force kill if needed
          await new Promise((r) => setTimeout(r, 2000));
          try {
            execSync(`kill -0 ${pid} 2>/dev/null`);
            execSync(`kill -9 ${pid} 2>/dev/null`);
            messages.push(`Force killed process ${pid}.`);
          } catch { /* already dead */ }
        } else {
          messages.push(`No process listening on port ${port}.`);
        }
      } catch {
        messages.push(`No process listening on port ${port}.`);
      }

      // Clean up screen session if any
      const sessionName = `goose-tester-dev-${port}`;
      try {
        execSync(`screen -S ${sessionName} -X quit 2>/dev/null`);
        messages.push(`Cleaned up screen session '${sessionName}'.`);
      } catch { /* no session */ }

      return {
        content: [{ type: "text", text: messages.join("\n") + `\n\n✅ Port ${port} is free.` }],
      };
    }

    // ═══════════════════════════════════════════════════════════════
    // Electron CDP tools
    // ═══════════════════════════════════════════════════════════════

    case "electron_connect": {
      const port = (args?.port as number | undefined) ?? DEFAULT_PORT;
      const host = (args?.host as string | undefined) ?? DEFAULT_HOST;

      if (port !== activePort || host !== activeHost) {
        cdp.disconnectAll();
        cdp = new CDPClient(port, host);
        activePort = port;
        activeHost = host;
      } else {
        cdp.disconnectAll();
      }

      try {
        const targets = await cdp.attachAll();
        if (targets.length === 0) {
          return {
            content: [
              {
                type: "text",
                text: `Connected to CDP on ${activeHost}:${activePort} but no debuggable targets found.\n\nMake sure the Electron app is running with:\n  ENABLE_PLAYWRIGHT=1 npm start`,
              },
            ],
          };
        }

        const summary = targets
          .map((t) => `  • [${t.type}] ${t.title} (${t.id})\n    ${t.url}`)
          .join("\n");

        return {
          content: [
            {
              type: "text",
              text: `Connected to Electron on ${activeHost}:${activePort}\nAttached to ${targets.length} target(s):\n\n${summary}\n\nConsole logs are now being collected. Use electron_get_logs to view them.`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to connect to Electron CDP on ${activeHost}:${activePort}.\n\nError: ${e instanceof Error ? e.message : String(e)}\n\nMake sure the Goose Electron app is running with:\n  ENABLE_PLAYWRIGHT=1 npm start`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_list_targets": {
      try {
        const targets = await cdp.listTargets();
        const attached = new Set(cdp.getAttachedTargetIds());

        if (targets.length === 0) {
          return {
            content: [{ type: "text", text: "No targets found. Is the Electron app running?" }],
          };
        }

        const lines = targets.map((t) => {
          const status = attached.has(t.id) ? "✓ attached" : "  detached";
          return `[${status}] [${t.type}] ${t.title}\n  id: ${t.id}\n  url: ${t.url}`;
        });

        return {
          content: [
            {
              type: "text",
              text: `Connected to ${activeHost}:${activePort}\n${targets.length} target(s):\n\n${lines.join("\n\n")}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to list targets: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_get_logs": {
      const entries = cdp.getEntries({
        targetId: args?.target_id as string | undefined,
        level: args?.level as string | undefined,
        since: args?.since as number | undefined,
        limit: (args?.limit as number | undefined) ?? 100,
        search: args?.search as string | undefined,
      });

      const lastId = entries.length > 0 ? entries[entries.length - 1].id : 0;

      return {
        content: [
          {
            type: "text",
            text:
              formatEntries(entries) +
              (entries.length > 0
                ? `\n\n--- ${entries.length} entries (last_id: ${lastId}, use since:${lastId} to poll) ---`
                : "\n\nNo logs yet. Interact with the app to generate output, or call electron_connect first."),
          },
        ],
      };
    }

    case "electron_clear_logs": {
      cdp.clearEntries();
      return {
        content: [{ type: "text", text: "Console log buffer cleared." }],
      };
    }

    case "electron_evaluate": {
      const expression = args?.expression as string;
      if (!expression) {
        return {
          content: [{ type: "text", text: "Missing required parameter: expression" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const result = await cdp.evaluate(targetId, expression);
        if (result.exceptionDetails) {
          return {
            content: [
              {
                type: "text",
                text: `Exception:\n${JSON.stringify(result.exceptionDetails, null, 2)}`,
              },
            ],
            isError: true,
          };
        }
        return {
          content: [{ type: "text", text: JSON.stringify(result.result, null, 2) }],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Evaluation failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_version": {
      try {
        const info = await cdp.getVersion();
        const lines = Object.entries(info)
          .map(([k, v]) => `${k}: ${v}`)
          .join("\n");
        return { content: [{ type: "text", text: lines }] };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to get version: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    // ═══════════════════════════════════════════════════════════════
    // Screenshot & DOM inspection tools
    // ═══════════════════════════════════════════════════════════════

    case "electron_screenshot": {
      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const format = (args?.format as "png" | "jpeg" | "webp" | undefined) ?? "png";
        const quality = args?.quality as number | undefined;
        const fullPage = args?.full_page as boolean | undefined;
        const savePath = args?.save_path as string | undefined;

        const data = await cdp.captureScreenshot(targetId, { format, quality, fullPage });

        if (savePath) {
          const { mkdir, writeFile } = await import("node:fs/promises");
          const { dirname } = await import("node:path");
          await mkdir(dirname(savePath), { recursive: true });
          await writeFile(savePath, Buffer.from(data, "base64"));
          const sizeKB = Math.round(Buffer.from(data, "base64").length / 1024);
          return {
            content: [
              {
                type: "text",
                text: `Screenshot saved to ${savePath} (${sizeKB} KB, ${format})`,
              },
            ],
          };
        }

        const mimeType = format === "jpeg" ? "image/jpeg" : format === "webp" ? "image/webp" : "image/png";
        return {
          content: [
            {
              type: "image",
              data,
              mimeType,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Screenshot failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_screenshot_element": {
      const selector = args?.selector as string;
      if (!selector) {
        return {
          content: [{ type: "text", text: "Missing required parameter: selector" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const format = (args?.format as "png" | "jpeg" | "webp" | undefined) ?? "png";
        const quality = args?.quality as number | undefined;
        const padding = args?.padding as number | undefined;
        const savePath = args?.save_path as string | undefined;

        const result = await cdp.captureElementScreenshot(targetId, selector, { format, quality, padding });

        if (savePath) {
          const { mkdir, writeFile } = await import("node:fs/promises");
          const { dirname } = await import("node:path");
          await mkdir(dirname(savePath), { recursive: true });
          await writeFile(savePath, Buffer.from(result.data, "base64"));
          const sizeKB = Math.round(Buffer.from(result.data, "base64").length / 1024);
          return {
            content: [
              {
                type: "text",
                text: `Element screenshot saved to ${savePath} (${sizeKB} KB, ${format})\nBounding box: ${JSON.stringify(result.box)}`,
              },
            ],
          };
        }

        const mimeType = format === "jpeg" ? "image/jpeg" : format === "webp" ? "image/webp" : "image/png";
        return {
          content: [
            {
              type: "image",
              data: result.data,
              mimeType,
            },
            {
              type: "text",
              text: `Element "${selector}" — bounding box: ${JSON.stringify(result.box)}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Element screenshot failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_dom_snapshot": {
      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const computedStyles = args?.computed_styles as string[] | undefined;

        const snapshot = await cdp.getDomSnapshot(targetId, { computedStyles });

        // The snapshot can be large — summarize it
        const snapshotStr = JSON.stringify(snapshot, null, 2);
        const truncated = snapshotStr.length > 50000
          ? snapshotStr.slice(0, 50000) + `\n\n... [truncated, ${snapshotStr.length} total chars]`
          : snapshotStr;

        return {
          content: [
            {
              type: "text",
              text: truncated,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `DOM snapshot failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_get_html": {
      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const selector = args?.selector as string | undefined;

        const html = await cdp.getDocumentOuterHTML(targetId, selector);

        const truncated = html.length > 100000
          ? html.slice(0, 100000) + `\n\n... [truncated, ${html.length} total chars]`
          : html;

        return {
          content: [
            {
              type: "text",
              text: selector
                ? `outerHTML for "${selector}" (${html.length} chars):\n\n${truncated}`
                : `Full document HTML (${html.length} chars):\n\n${truncated}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Get HTML failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    // ═══════════════════════════════════════════════════════════════
    // Navigation & interaction tools
    // ═══════════════════════════════════════════════════════════════

    case "electron_click": {
      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const selector = args?.selector as string | undefined;
        const x = args?.x as number | undefined;
        const y = args?.y as number | undefined;
        const button = (args?.button as "left" | "right" | "middle" | undefined) ?? "left";
        const clickCount = (args?.click_count as number | undefined) ?? 1;

        if (selector) {
          const point = await cdp.clickSelector(targetId, selector, { button, clickCount });
          return {
            content: [
              {
                type: "text",
                text: `Clicked "${selector}" at (${Math.round(point.x)}, ${Math.round(point.y)})`,
              },
            ],
          };
        } else if (x !== undefined && y !== undefined) {
          await cdp.clickAtPoint(targetId, x, y, { button, clickCount });
          return {
            content: [
              {
                type: "text",
                text: `Clicked at (${x}, ${y})`,
              },
            ],
          };
        } else {
          return {
            content: [{ type: "text", text: "Provide either a 'selector' or both 'x' and 'y' coordinates." }],
            isError: true,
          };
        }
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Click failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_type": {
      const text = args?.text as string;
      if (!text) {
        return {
          content: [{ type: "text", text: "Missing required parameter: text" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const selector = args?.selector as string | undefined;
        const clear = args?.clear as boolean | undefined;
        const pressEnter = args?.press_enter as boolean | undefined;

        if (selector) {
          await cdp.focus(targetId, selector);
        }

        if (clear) {
          // Select all then delete
          await cdp.pressKey(targetId, "a", { modifiers: 4 }); // Cmd+A
          await cdp.pressKey(targetId, "Backspace");
        }

        await cdp.typeText(targetId, text);

        if (pressEnter) {
          await cdp.pressKey(targetId, "Enter");
        }

        const details = [
          `Typed ${text.length} character(s)`,
          selector ? `into "${selector}"` : "into focused element",
          clear ? "(cleared first)" : "",
          pressEnter ? "(pressed Enter)" : "",
        ].filter(Boolean).join(" ");

        return {
          content: [{ type: "text", text: details }],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Type failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_press_key": {
      const key = args?.key as string;
      if (!key) {
        return {
          content: [{ type: "text", text: "Missing required parameter: key" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const modifiers = args?.modifiers as number | undefined;

        await cdp.pressKey(targetId, key, { modifiers });

        const modNames: string[] = [];
        if (modifiers) {
          if (modifiers & 1) modNames.push("Alt");
          if (modifiers & 2) modNames.push("Ctrl");
          if (modifiers & 4) modNames.push("Cmd");
          if (modifiers & 8) modNames.push("Shift");
        }
        const combo = modNames.length > 0 ? `${modNames.join("+")}+${key}` : key;

        return {
          content: [{ type: "text", text: `Pressed ${combo}` }],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Key press failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_navigate": {
      const url = args?.url as string;
      if (!url) {
        return {
          content: [{ type: "text", text: "Missing required parameter: url" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const result = await cdp.navigate(targetId, url);

        if (result.errorText) {
          return {
            content: [
              {
                type: "text",
                text: `Navigation error: ${result.errorText}`,
              },
            ],
            isError: true,
          };
        }

        return {
          content: [{ type: "text", text: `Navigated to ${url}` }],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Navigation failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_wait_for": {
      const selector = args?.selector as string;
      if (!selector) {
        return {
          content: [{ type: "text", text: "Missing required parameter: selector" }],
          isError: true,
        };
      }

      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const timeout = args?.timeout as number | undefined;
        const visible = args?.visible as boolean | undefined;

        const found = await cdp.waitForSelector(targetId, selector, {
          timeoutMs: timeout,
          visible,
        });

        return {
          content: [
            {
              type: "text",
              text: found
                ? `✓ Found "${selector}"${visible ? " (visible)" : ""}`
                : `✗ Timed out waiting for "${selector}" after ${timeout ?? 10000}ms`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Wait failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_scroll": {
      try {
        const targetId = await resolveTargetId(args?.target_id as string | undefined);
        const selector = args?.selector as string | undefined;
        const x = args?.x as number | undefined;
        const y = args?.y as number | undefined;

        await cdp.scrollTo(targetId, { selector, x, y });

        return {
          content: [
            {
              type: "text",
              text: selector
                ? `Scrolled "${selector}" into view`
                : `Scrolled to (${x ?? 0}, ${y ?? 0})`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Scroll failed: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    // ═══════════════════════════════════════════════════════════════
    // Server log tools
    // ═══════════════════════════════════════════════════════════════

    case "server_list_sessions": {
      try {
        const sessions = await listServerSessions({
          date: args?.date as string | undefined,
          limit: (args?.limit as number | undefined) ?? 20,
        });

        if (sessions.length === 0) {
          return {
            content: [{ type: "text", text: "No server log sessions found." }],
          };
        }

        const lines = sessions.map((s, i) => {
          const timeFormatted = s.startTime.replace(
            /^(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})$/,
            "$1-$2-$3 $4:$5:$6"
          );
          return `${i + 1}. [${s.date}] ${timeFormatted}  ${s.sizeHuman.padStart(10)}  ${s.startTime}`;
        });

        return {
          content: [
            {
              type: "text",
              text: `Goosed server sessions (most recent first):\n\n   Date        Started           Size  Session ID\n${lines.join("\n")}\n\nUse server_get_logs with session: "<Session ID>" to read a specific session's logs.`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to list sessions: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "server_get_logs": {
      try {
        const result = await readServerLog({
          session: args?.session as string | undefined,
          date: args?.date as string | undefined,
          tail: args?.tail as number | undefined,
          head: args?.head as number | undefined,
          level: args?.level as string | undefined,
          search: args?.search as string | undefined,
          module: args?.module as string | undefined,
        });

        const formatted = result.lines
          .map((l) => {
            if (l.level) {
              const ts = l.timestamp?.slice(11, 23) ?? "";
              return `[${ts}] ${l.level.padEnd(5)} ${l.module ?? ""}: ${l.message ?? l.raw}`;
            }
            return l.raw;
          })
          .join("\n");

        return {
          content: [
            {
              type: "text",
              text: `${result.filepath}\n(${result.totalLines} total lines, showing ${result.lines.length})\n\n${formatted}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to read server logs: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "server_list_mcp_logs": {
      try {
        const logs = await listMcpLogs();

        if (logs.length === 0) {
          return {
            content: [{ type: "text", text: "No MCP extension log files found." }],
          };
        }

        const lines = logs.map(
          (l) => `  ${l.name.padEnd(35)} ${l.sizeHuman.padStart(10)}  modified: ${l.modifiedAt.slice(0, 19)}`
        );

        return {
          content: [
            {
              type: "text",
              text: `MCP extension logs:\n\n${lines.join("\n")}\n\nUse server_get_mcp_log with name: "<name>" to read.`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to list MCP logs: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "server_get_mcp_log": {
      try {
        const result = await readMcpLog({
          name: args?.name as string | undefined,
          tail: args?.tail as number | undefined,
          search: args?.search as string | undefined,
        });

        return {
          content: [
            {
              type: "text",
              text: `${result.filepath}\n(${result.totalLines} total lines, showing ${result.lines.length})\n\n${result.lines.join("\n")}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to read MCP log: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "electron_get_main_log": {
      try {
        const result = await readElectronMainLog({
          tail: args?.tail as number | undefined,
          search: args?.search as string | undefined,
          level: args?.level as string | undefined,
        });

        return {
          content: [
            {
              type: "text",
              text: `${result.filepath}\n(${result.totalLines} total lines, showing ${result.lines.length})\n\n${result.lines.join("\n")}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to read Electron main log: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    // ═══════════════════════════════════════════════════════════════
    // Chat session tools (from sessions.db)
    // ═══════════════════════════════════════════════════════════════

    case "server_list_chat_sessions": {
      try {
        const sessions = await listChatSessions({
          search: args?.search as string | undefined,
          limit: (args?.limit as number | undefined) ?? 30,
          activeOnly: args?.active_only as boolean | undefined,
        });

        if (sessions.length === 0) {
          return {
            content: [{ type: "text", text: "No chat sessions found." }],
          };
        }

        const lines = sessions.map((s) => {
          const active = s.inUse ? " ◉ ACTIVE" : "";
          const tokens = s.totalTokens ? `${s.totalTokens.toLocaleString()} tokens` : "no tokens";
          return `  ${s.id.padEnd(15)} ${(s.name || "(unnamed)").padEnd(40)} ${tokens.padStart(14)}  ${s.createdAt}${active}`;
        });

        return {
          content: [
            {
              type: "text",
              text: `Chat sessions (from sessions.db):\n\n  ${"ID".padEnd(15)} ${"Name".padEnd(40)} ${"Tokens".padStart(14)}  Created\n${lines.join("\n")}\n\nUse server_get_chat_session_logs with session_name or session_id to read server logs for a session.`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to list chat sessions: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    case "server_get_chat_session_logs": {
      try {
        const sessionName = args?.session_name as string | undefined;
        const sessionId = args?.session_id as string | undefined;

        // Resolve the chat session from the DB
        let chatSession: ChatSession | null = null;

        if (sessionId) {
          chatSession = await findSession(sessionId);
        } else if (sessionName) {
          chatSession = await findSession(sessionName);
        } else {
          // Default: current active session
          chatSession = await getCurrentSession();
        }

        if (!chatSession) {
          const hint = sessionName
            ? `No session matching "${sessionName}" found.`
            : sessionId
              ? `No session with ID "${sessionId}" found.`
              : "No active session found.";
          return {
            content: [
              {
                type: "text",
                text: `${hint}\n\nUse server_list_chat_sessions to see available sessions.`,
              },
            ],
            isError: true,
          };
        }

        // Find which server log file contains this session ID.
        // Log files can be huge (hundreds of MB), so we search smartly:
        // 1. Extract the date from the session ID (YYYYMMDD_N format)
        // 2. Search that date's directory first, then adjacent dates
        // 3. Use grep -l (not -r) on specific files to avoid scanning everything
        const { execSync } = await import("node:child_process");
        const { readdirSync } = await import("node:fs");
        const { join } = await import("node:path");
        let matchedLogPath: string | undefined;

        // Extract date from session ID (e.g., "20260213_4" -> "2026-02-13")
        const dateMatch = chatSession.id.match(/^(\d{4})(\d{2})(\d{2})_/);
        const sessionDateStr = dateMatch
          ? `${dateMatch[1]}-${dateMatch[2]}-${dateMatch[3]}`
          : undefined;

        // Build list of date directories to search, starting with the session date and nearby dates
        let dateDirs: string[] = [];
        try {
          dateDirs = readdirSync(SERVER_LOGS_DIR)
            .filter((d) => /^\d{4}-\d{2}-\d{2}$/.test(d))
            .sort()
            .reverse(); // newest first
        } catch { /* dir doesn't exist */ }

        // Prioritize: session date first, then the day before, then others
        if (sessionDateStr) {
          const prevDate = new Date(sessionDateStr + "T00:00:00Z");
          prevDate.setUTCDate(prevDate.getUTCDate() - 1);
          const prevDateStr = prevDate.toISOString().slice(0, 10);

          const priority = [sessionDateStr, prevDateStr];
          const prioritized = priority.filter((d) => dateDirs.includes(d));
          const rest = dateDirs.filter((d) => !priority.includes(d));
          dateDirs = [...prioritized, ...rest];
        }

        // Search each date directory's log files for the session ID
        // Use grep -l on individual files with a short timeout per file
        for (const dateDir of dateDirs) {
          if (matchedLogPath) break;
          const dirPath = join(SERVER_LOGS_DIR, dateDir);
          let logFiles: string[] = [];
          try {
            logFiles = readdirSync(dirPath)
              .filter((f) => f.endsWith("-goosed.log"))
              .sort()
              .reverse(); // newest first
          } catch { continue; }

          for (const logFile of logFiles) {
            const filePath = join(dirPath, logFile);
            try {
              // Use grep -c to just count matches (much faster than -l on a single file)
              // and set a generous timeout since files can be hundreds of MB
              const countStr = execSync(
                `grep -c "${chatSession.id}" "${filePath}" 2>/dev/null || true`,
                { encoding: "utf-8", timeout: 120000 }
              ).trim();
              if (parseInt(countStr, 10) > 0) {
                matchedLogPath = filePath;
                break;
              }
            } catch {
              // timeout or other error, skip this file
            }
          }
        }

        if (!matchedLogPath) {
          // Debug: try to list what's in the logs dir
          let debugInfo = "";
          try {
            const lsResult = execSync(`ls -la "${SERVER_LOGS_DIR}/" 2>&1 | head -10`, { encoding: "utf-8" });
            debugInfo = `\n\nDebug - ls ${SERVER_LOGS_DIR}/:\n${lsResult}`;
          } catch { /* ignore */ }
          try {
            const grepDebug = execSync(`grep -rl "${chatSession.id}" "${SERVER_LOGS_DIR}/" 2>&1 | head -3`, { encoding: "utf-8" });
            debugInfo += `\nDebug - grep result: ${grepDebug}`;
          } catch (e) {
            debugInfo += `\nDebug - grep error: ${e instanceof Error ? e.message : String(e)}`;
          }

          return {
            content: [
              {
                type: "text",
                text: `Found session "${chatSession.name}" (${chatSession.id}) in DB, but could not find its server log file.\n\nSession created: ${chatSession.createdAt}\nSearched: ${SERVER_LOGS_DIR}${debugInfo}`,
              },
            ],
          };
        }

        // Use grep to extract only lines mentioning this session ID.
        // TRACE lines can contain full session state dumps (hundreds of MB total),
        // so we exclude them by default and truncate lines to 1000 chars.
        // Users can opt into TRACE with the level filter.
        const includeTrace = args?.level && (args.level as string).toUpperCase().includes("TRACE");
        const grepFilter = includeTrace
          ? `grep "${chatSession.id}" "${matchedLogPath}"`
          : `grep "${chatSession.id}" "${matchedLogPath}" | grep -Ev "TRACE"`;
        let matchedLines: string[] = [];
        let grepError = "";
        try {
          const grepCmd = `${grepFilter} 2>&1 | cut -c1-1000`;
          const grepLines = execSync(
            grepCmd,
            { encoding: "utf-8", timeout: 120000, maxBuffer: 50 * 1024 * 1024 }
          );
          matchedLines = grepLines.split("\n").filter((l) => l.length > 0);
        } catch (e: unknown) {
          const err = e as { status?: number; stderr?: string; stdout?: string; message?: string };
          grepError = `status=${err.status}, stderr=${(err.stderr ?? "").slice(0, 300)}, stdout_len=${(err.stdout ?? "").length}, msg=${(err.message ?? "").slice(0, 300)}`;
        }

        // Get total line count for context
        let totalLines = 0;
        try {
          const wcResult = execSync(`wc -l < "${matchedLogPath}"`, { encoding: "utf-8" }).trim();
          totalLines = parseInt(wcResult, 10) || 0;
        } catch { /* ignore */ }

        // Parse matched lines
        interface ParsedLogLine {
          lineNumber: number;
          raw: string;
          timestamp: string | undefined;
          level: string | undefined;
          module: string | undefined;
          message: string | undefined;
        }

        const LOG_LINE_RE = /^(\d{4}-\d{2}-\d{2}T[\d:.]+Z)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(\S+):\s+\S+:\s+(.*)$/;
        let parsedLines: ParsedLogLine[] = matchedLines.map((raw, i) => {
          const m = raw.match(LOG_LINE_RE);
          if (m) {
            return { lineNumber: i + 1, raw, timestamp: m[1], level: m[2], module: m[3], message: m[4] };
          }
          return { lineNumber: i + 1, raw, timestamp: undefined, level: undefined, module: undefined, message: undefined };
        });

        // Apply level filter
        if (args?.level) {
          const levels = new Set((args.level as string).toUpperCase().split(",").map((s) => s.trim()));
          parsedLines = parsedLines.filter((l) => l.level && levels.has(l.level));
        }

        // Apply search filter
        if (args?.search) {
          const term = (args.search as string).toLowerCase();
          parsedLines = parsedLines.filter((l) => l.raw.toLowerCase().includes(term));
        }

        // Apply tail
        const tail = (args?.tail as number | undefined) ?? 500;
        if (parsedLines.length > tail) {
          parsedLines = parsedLines.slice(-tail);
        }

        // Truncate very long lines (session dumps can be huge)
        const formatted = parsedLines
          .map((l) => {
            if (l.level) {
              const ts = l.timestamp?.slice(11, 23) ?? "";
              const msg = l.message ?? l.raw;
              const truncated = msg.length > 500 ? msg.slice(0, 500) + "... [truncated]" : msg;
              return `[${ts}] ${l.level.padEnd(5)} ${l.module ?? ""}: ${truncated}`;
            }
            const truncated = l.raw.length > 500 ? l.raw.slice(0, 500) + "... [truncated]" : l.raw;
            return truncated;
          })
          .join("\n");

        const header = [
          `Session: "${chatSession.name}" (${chatSession.id})`,
          `Created: ${chatSession.createdAt}`,
          `Tokens: ${chatSession.totalTokens?.toLocaleString() ?? "unknown"}`,
          `Active: ${chatSession.inUse ? "yes" : "no"}`,
          `Log file: ${matchedLogPath}`,
          `(${totalLines} total lines in log, ${parsedLines.length} lines for this session)`,
        ].join("\n");

        return {
          content: [
            {
              type: "text",
              text: parsedLines.length > 0
                ? `${header}\n\n${formatted}`
                : `${header}\n\nNo log lines found for this session after filtering.${grepError ? `\nGrep error: ${grepError}` : ""}`,
            },
          ],
        };
      } catch (e) {
        return {
          content: [
            {
              type: "text",
              text: `Failed to get chat session logs: ${e instanceof Error ? e.message : String(e)}`,
            },
          ],
          isError: true,
        };
      }
    }

    default:
      return {
        content: [{ type: "text", text: `Unknown tool: ${name}` }],
        isError: true,
      };
  }
});

// ── Start ─────────────────────────────────────────────────────────

const transport = new StdioServerTransport();
await server.connect(transport);
