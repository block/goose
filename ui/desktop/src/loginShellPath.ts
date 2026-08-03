import { spawn } from 'child_process';

import type { Logger } from './gooseServe';

const RESOLVE_TIMEOUT_MS = 5000;

/**
 * Resolve the user's full PATH by running their login shell (bash/zsh).
 *
 * The desktop app launched from Finder/Dock inherits a minimal PATH from
 * launchd, so goosed can't find CLI-backed providers (claude, etc.). Sourcing
 * the user's profile via a login+interactive shell recovers the real PATH.
 * Doing this here rather than in goosed keeps the plain `goose` CLI on the
 * ambient PATH. Returns null on Windows, timeout, or any failure.
 */
const resolveLoginShellPath = (logger?: Logger): Promise<string | null> => {
  if (process.platform === 'win32') {
    return Promise.resolve(null);
  }

  const shell = process.env.GOOSE_SHELL || process.env.SHELL || 'bash';

  return new Promise((resolve) => {
    // detached: a new session keeps the interactive shell's job-control setup
    // from stealing the terminal foreground and suspending the app.
    const child = spawn(shell, ['-l', '-i', '-c', 'echo $PATH'], {
      stdio: ['ignore', 'pipe', 'ignore'],
      detached: true,
      windowsHide: true,
    });

    const timer = setTimeout(() => {
      child.kill();
      resolve(null);
    }, RESOLVE_TIMEOUT_MS);
    timer.unref?.();

    let stdout = '';
    child.stdout?.on('data', (chunk: Buffer) => {
      stdout += chunk.toString('utf8');
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      logger?.error('Failed to resolve login shell PATH', error);
      resolve(null);
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      const path = stdout.trim().split('\n').pop()?.trim();
      resolve(code === 0 && path ? path : null);
    });
  });
};

let cached: Promise<string | null> | undefined;

/** Resolve the login-shell PATH once per app run, caching the result. */
export const getLoginShellPath = (logger?: Logger): Promise<string | null> => {
  cached ??= resolveLoginShellPath(logger);
  return cached;
};
