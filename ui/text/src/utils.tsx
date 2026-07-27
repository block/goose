import os from "node:os";

const SHORTEN_PATH_MAX_LEN = 60;
const SHORTEN_PATH_MAX_PARTS = 3;

export function isErrorStatus(status: string): boolean {
  return status.startsWith("error") || status.startsWith("failed");
}

export function shortenPath(path: string): string {
  const home = os.homedir();
  const withTilde =
    path === home
      ? "~"
      : path.startsWith(`${home}/`)
        ? `~${path.slice(home.length)}`
        : path;

  if (withTilde.length <= SHORTEN_PATH_MAX_LEN) return withTilde;

  const parts = withTilde.split("/");
  if (parts.length <= SHORTEN_PATH_MAX_PARTS) return withTilde;

  const shortened = [parts[0]];
  for (const part of parts.slice(1, -2)) {
    if (part) shortened.push(part[0]!);
  }
  shortened.push(...parts.slice(-2));

  return shortened.join("/");
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
