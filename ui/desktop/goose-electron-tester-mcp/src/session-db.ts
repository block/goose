/**
 * session-db.ts
 *
 * Reads the goose sessions.db (SQLite) to map session names/IDs to metadata.
 * Uses the sqlite3 CLI to avoid native module dependencies.
 *
 * DB location: ~/.local/share/goose/sessions/sessions.db
 */

import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const SESSIONS_DB = join(
  homedir(),
  ".local",
  "share",
  "goose",
  "sessions",
  "sessions.db"
);

export interface ChatSession {
  id: string;
  name: string;
  description: string;
  workingDir: string;
  createdAt: string;
  updatedAt: string;
  totalTokens: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
  inUse: boolean;
  sessionType: string;
  providerName: string | null;
}

async function queryDb(sql: string): Promise<string> {
  if (!existsSync(SESSIONS_DB)) {
    throw new Error(`Sessions database not found at ${SESSIONS_DB}`);
  }

  const { stdout } = await execFileAsync("sqlite3", [
    "-json",
    SESSIONS_DB,
    sql,
  ], { timeout: 5000 });

  return stdout;
}

function parseRows(stdout: string): Record<string, unknown>[] {
  const trimmed = stdout.trim();
  if (!trimmed || trimmed === "[]") return [];
  return JSON.parse(trimmed);
}

function rowToSession(row: Record<string, unknown>): ChatSession {
  return {
    id: row.id as string,
    name: row.name as string ?? "",
    description: row.description as string ?? "",
    workingDir: row.working_dir as string ?? "",
    createdAt: row.created_at as string ?? "",
    updatedAt: row.updated_at as string ?? "",
    totalTokens: row.total_tokens as number | null,
    inputTokens: row.input_tokens as number | null,
    outputTokens: row.output_tokens as number | null,
    inUse: (row.in_use as number) === 1,
    sessionType: row.session_type as string ?? "user",
    providerName: row.provider_name as string | null,
  };
}

/**
 * List chat sessions from the DB, ordered by most recent first.
 */
export async function listChatSessions(opts?: {
  limit?: number;
  search?: string;
  activeOnly?: boolean;
}): Promise<ChatSession[]> {
  const limit = opts?.limit ?? 30;

  let where = "WHERE session_type = 'user'";
  if (opts?.activeOnly) {
    where += " AND in_use = 1";
  }
  if (opts?.search) {
    const escaped = opts.search.replace(/'/g, "''");
    where += ` AND (name LIKE '%${escaped}%' OR id LIKE '%${escaped}%')`;
  }

  const sql = `SELECT id, name, description, working_dir, created_at, updated_at, total_tokens, input_tokens, output_tokens, in_use, session_type, provider_name FROM sessions ${where} ORDER BY updated_at DESC LIMIT ${limit};`;

  const stdout = await queryDb(sql);
  return parseRows(stdout).map(rowToSession);
}

/**
 * Find a session by name (fuzzy, case-insensitive) or exact ID.
 */
export async function findSession(nameOrId: string): Promise<ChatSession | null> {
  // Try exact ID match first
  const byId = await queryDb(
    `SELECT id, name, description, working_dir, created_at, updated_at, total_tokens, input_tokens, output_tokens, in_use, session_type, provider_name FROM sessions WHERE id = '${nameOrId.replace(/'/g, "''")}' LIMIT 1;`
  );
  const idRows = parseRows(byId);
  if (idRows.length > 0) return rowToSession(idRows[0]);

  // Fuzzy name match
  const escaped = nameOrId.replace(/'/g, "''");
  const byName = await queryDb(
    `SELECT id, name, description, working_dir, created_at, updated_at, total_tokens, input_tokens, output_tokens, in_use, session_type, provider_name FROM sessions WHERE LOWER(name) LIKE '%${escaped.toLowerCase()}%' ORDER BY updated_at DESC LIMIT 1;`
  );
  const nameRows = parseRows(byName);
  if (nameRows.length > 0) return rowToSession(nameRows[0]);

  return null;
}

/**
 * Get the currently active (in_use) session, if any.
 */
export async function getCurrentSession(): Promise<ChatSession | null> {
  const sessions = await listChatSessions({ activeOnly: true, limit: 1 });
  return sessions.length > 0 ? sessions[0] : null;
}

/**
 * Get the first user message for a session (useful for context).
 */
export async function getSessionFirstMessage(sessionId: string): Promise<string | null> {
  const escaped = sessionId.replace(/'/g, "''");
  const stdout = await queryDb(
    `SELECT content_json FROM messages WHERE session_id = '${escaped}' AND role = 'user' ORDER BY created_timestamp ASC LIMIT 1;`
  );
  const rows = parseRows(stdout);
  if (rows.length === 0) return null;
  
  try {
    const content = JSON.parse(rows[0].content_json as string);
    if (Array.isArray(content)) {
      const textPart = content.find((p: { type: string }) => p.type === "text");
      return textPart?.text ?? null;
    }
    return typeof content === "string" ? content : null;
  } catch {
    return rows[0].content_json as string;
  }
}
