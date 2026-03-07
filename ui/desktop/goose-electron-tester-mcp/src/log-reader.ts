/**
 * log-reader.ts
 *
 * Reads goosed server logs, MCP extension logs, and the Electron main.log.
 *
 * Log locations:
 *   - Goosed server:  ~/.local/state/goose/logs/server/YYYY-MM-DD/YYYYMMDD_HHMMSS-goosed.log
 *   - MCP extensions: ~/.local/state/goose/logs/mcps/mcp_<name>.log
 *   - Electron main:  ~/Library/Application Support/Goose/logs/main.log
 *
 * The server logs use tracing format:
 *   2026-02-12T23:34:02.656146Z  INFO goosed::commands::agent: crates/goose-server/src/commands/agent.rs: listening on 127.0.0.1:61546
 */

import { readdir, readFile, stat } from "node:fs/promises";
import { join } from "node:path";
import { homedir } from "node:os";
import { existsSync } from "node:fs";

// ── Path helpers ────────────────────────────────────────────────────

const HOME = homedir();
const GOOSE_STATE_LOGS = join(HOME, ".local", "state", "goose", "logs");
export const SERVER_LOGS_DIR = join(GOOSE_STATE_LOGS, "server");
const MCP_LOGS_DIR = join(GOOSE_STATE_LOGS, "mcps");

// Electron main.log can be in different locations depending on the app
const ELECTRON_LOG_CANDIDATES = [
  join(HOME, "Library", "Application Support", "Goose", "logs", "main.log"),
  join(HOME, "Library", "Application Support", "Block.goose", "logs", "main.log"),
  join(HOME, "Library", "Logs", "Goose", "main.log"),
  // Linux
  join(HOME, ".config", "Goose", "logs", "main.log"),
];

function findElectronMainLog(): string | null {
  for (const candidate of ELECTRON_LOG_CANDIDATES) {
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

// ── Types ───────────────────────────────────────────────────────────

export interface ServerSession {
  filename: string;
  filepath: string;
  date: string;         // YYYY-MM-DD
  startTime: string;    // YYYYMMDD_HHMMSS
  sizeBytes: number;
  sizeHuman: string;
  modifiedAt: string;
}

export interface LogLine {
  lineNumber: number;
  raw: string;
  timestamp?: string;
  level?: string;
  module?: string;
  message?: string;
}

// ── Size formatting ─────────────────────────────────────────────────

function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ── Parse a tracing log line ────────────────────────────────────────

const LOG_LINE_RE = /^(\d{4}-\d{2}-\d{2}T[\d:.]+Z)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(\S+):\s+\S+:\s+(.*)$/;

function parseLogLine(raw: string, lineNumber: number): LogLine {
  const m = raw.match(LOG_LINE_RE);
  if (m) {
    return {
      lineNumber,
      raw,
      timestamp: m[1],
      level: m[2],
      module: m[3],
      message: m[4],
    };
  }
  return { lineNumber, raw };
}

// ── Server sessions ─────────────────────────────────────────────────

export async function listServerSessions(opts?: {
  date?: string;       // YYYY-MM-DD, default: today
  limit?: number;      // default: 20
}): Promise<ServerSession[]> {
  const limit = opts?.limit ?? 20;
  const sessions: ServerSession[] = [];

  let dateDirs: string[];
  if (opts?.date) {
    dateDirs = [opts.date];
  } else {
    // List all date dirs, sorted descending
    try {
      const entries = await readdir(SERVER_LOGS_DIR);
      dateDirs = entries
        .filter((e) => /^\d{4}-\d{2}-\d{2}$/.test(e))
        .sort()
        .reverse();
    } catch {
      return [];
    }
  }

  for (const dateDir of dateDirs) {
    if (sessions.length >= limit) break;

    const dirPath = join(SERVER_LOGS_DIR, dateDir);
    try {
      const files = await readdir(dirPath);
      const logFiles = files
        .filter((f) => f.endsWith("-goosed.log"))
        .sort()
        .reverse();

      for (const file of logFiles) {
        if (sessions.length >= limit) break;

        const filepath = join(dirPath, file);
        const st = await stat(filepath);
        const startTime = file.replace("-goosed.log", "");

        sessions.push({
          filename: file,
          filepath,
          date: dateDir,
          startTime,
          sizeBytes: st.size,
          sizeHuman: humanSize(st.size),
          modifiedAt: st.mtime.toISOString(),
        });
      }
    } catch {
      continue;
    }
  }

  return sessions;
}

// ── Read server log ─────────────────────────────────────────────────

export async function readServerLog(opts: {
  filepath?: string;
  date?: string;
  session?: string;     // filename or startTime prefix
  tail?: number;        // read last N lines (default: 200)
  head?: number;        // read first N lines
  level?: string;       // filter by level(s), comma-separated
  search?: string;      // text search (case-insensitive)
  module?: string;      // filter by module prefix
}): Promise<{ lines: LogLine[]; filepath: string; totalLines: number }> {
  let filepath = opts.filepath;

  // Resolve filepath from date + session
  if (!filepath) {
    const sessions = await listServerSessions({ date: opts.date, limit: 100 });
    if (sessions.length === 0) {
      throw new Error(`No server log sessions found${opts.date ? ` for date ${opts.date}` : ""}`);
    }

    if (opts.session) {
      const match = sessions.find(
        (s) => s.filename === opts.session || s.startTime.startsWith(opts.session!)
      );
      if (!match) {
        const available = sessions.map((s) => s.startTime).join(", ");
        throw new Error(`Session "${opts.session}" not found. Available: ${available}`);
      }
      filepath = match.filepath;
    } else {
      // Default to most recent
      filepath = sessions[0].filepath;
    }
  }

  const content = await readFile(filepath, "utf-8");
  const allRawLines = content.split("\n");
  const totalLines = allRawLines.length;

  // Apply head/tail
  let rawLines: string[];
  let startLineNumber: number;
  if (opts.head) {
    rawLines = allRawLines.slice(0, opts.head);
    startLineNumber = 1;
  } else {
    const tail = opts.tail ?? 200;
    const start = Math.max(0, allRawLines.length - tail);
    rawLines = allRawLines.slice(start);
    startLineNumber = start + 1;
  }

  // Parse
  let lines = rawLines.map((raw, i) => parseLogLine(raw, startLineNumber + i));

  // Filter by level
  if (opts.level) {
    const levels = new Set(opts.level.toUpperCase().split(",").map((l) => l.trim()));
    lines = lines.filter((l) => l.level && levels.has(l.level));
  }

  // Filter by module
  if (opts.module) {
    const prefix = opts.module.toLowerCase();
    lines = lines.filter((l) => l.module && l.module.toLowerCase().startsWith(prefix));
  }

  // Filter by search
  if (opts.search) {
    const term = opts.search.toLowerCase();
    lines = lines.filter((l) => l.raw.toLowerCase().includes(term));
  }

  return { lines, filepath, totalLines };
}

// ── MCP extension logs ──────────────────────────────────────────────

export interface McpLogFile {
  name: string;
  filepath: string;
  sizeBytes: number;
  sizeHuman: string;
  modifiedAt: string;
}

export async function listMcpLogs(): Promise<McpLogFile[]> {
  try {
    const entries = await readdir(MCP_LOGS_DIR);
    const results: McpLogFile[] = [];

    for (const entry of entries.sort()) {
      const filepath = join(MCP_LOGS_DIR, entry);
      const st = await stat(filepath);
      if (st.isFile()) {
        results.push({
          name: entry,
          filepath,
          sizeBytes: st.size,
          sizeHuman: humanSize(st.size),
          modifiedAt: st.mtime.toISOString(),
        });
      }
    }
    return results;
  } catch {
    return [];
  }
}

export async function readMcpLog(opts: {
  name?: string;        // filename or extension name (e.g., "developer" or "mcp_developer.log")
  filepath?: string;
  tail?: number;
  search?: string;
}): Promise<{ lines: string[]; filepath: string; totalLines: number }> {
  let filepath = opts.filepath;

  if (!filepath) {
    const logs = await listMcpLogs();
    if (!opts.name) {
      throw new Error("Provide either 'name' or 'filepath'");
    }

    const normalizedName = opts.name.toLowerCase();
    const match = logs.find(
      (l) =>
        l.name === opts.name ||
        l.name.toLowerCase().includes(normalizedName) ||
        l.name === `mcp_${normalizedName}.log`
    );

    if (!match) {
      const available = logs.map((l) => l.name).join(", ");
      throw new Error(`MCP log "${opts.name}" not found. Available: ${available}`);
    }
    filepath = match.filepath;
  }

  const content = await readFile(filepath, "utf-8");
  const allLines = content.split("\n");
  const totalLines = allLines.length;

  const tail = opts.tail ?? 200;
  let lines = allLines.slice(Math.max(0, allLines.length - tail));

  if (opts.search) {
    const term = opts.search.toLowerCase();
    lines = lines.filter((l) => l.toLowerCase().includes(term));
  }

  return { lines, filepath, totalLines };
}

// ── Electron main.log ───────────────────────────────────────────────

export async function readElectronMainLog(opts?: {
  tail?: number;
  search?: string;
  level?: string;       // filter: error, warn, info
}): Promise<{ lines: string[]; filepath: string; totalLines: number }> {
  const filepath = findElectronMainLog();
  if (!filepath) {
    throw new Error(
      `Electron main.log not found. Searched:\n${ELECTRON_LOG_CANDIDATES.map((c) => `  ${c}`).join("\n")}`
    );
  }

  const content = await readFile(filepath, "utf-8");
  const allLines = content.split("\n");
  const totalLines = allLines.length;

  const tail = opts?.tail ?? 100;
  let lines = allLines.slice(Math.max(0, allLines.length - tail));

  if (opts?.level) {
    const levels = new Set(opts.level.toLowerCase().split(",").map((l) => l.trim()));
    lines = lines.filter((l) => {
      const m = l.match(/\[(error|warn|info|debug|verbose|silly)\]/);
      return m && levels.has(m[1]);
    });
  }

  if (opts?.search) {
    const term = opts.search.toLowerCase();
    lines = lines.filter((l) => l.toLowerCase().includes(term));
  }

  return { lines, filepath, totalLines };
}
