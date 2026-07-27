import os from "node:os";
import path from "node:path";

const SHORTEN_PATH_MAX_LEN = 60;
const SHORTEN_PATH_MAX_PARTS = 3;

export function isErrorStatus(status: string): boolean {
  return status.startsWith("error") || status.startsWith("failed");
}

// Separators come from node:path so this also works on Windows, where the
// launcher (scripts/dev-start.mjs) supports goose.exe and paths use backslashes.
export function shortenPath(target: string): string {
  const home = os.homedir();
  const sep = path.sep;
  const normalized = path.normalize(target);
  const withTilde =
    normalized === home
      ? "~"
      : normalized.startsWith(`${home}${sep}`)
        ? `~${normalized.slice(home.length)}`
        : normalized;

  if (withTilde.length <= SHORTEN_PATH_MAX_LEN) return withTilde;

  const parts = withTilde.split(sep);
  if (parts.length <= SHORTEN_PATH_MAX_PARTS) return withTilde;

  const shortened = [parts[0]];
  for (const part of parts.slice(1, -2)) {
    if (part) shortened.push(part[0]!);
  }
  shortened.push(...parts.slice(-2));

  return shortened.join(sep);
}

export function formatError(e: unknown): string {
  if (e instanceof Error) {
    return e.message || e.toString();
  }
  if (typeof e === "string") {
    return e;
  }
  if (e && typeof e === "object") {
    try {
      return JSON.stringify(e, null, 2);
    } catch {
      return String(e);
    }
  }
  return String(e);
}
