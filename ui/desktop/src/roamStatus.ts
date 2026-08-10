import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * Status file written by `goose serve --roam` (see crates/goose-cli/src/cli.rs,
 * start_roam_share). The file is removed on startup and rewritten atomically once
 * the roaming endpoint is online.
 */
export interface RoamServeStatus {
  card: string;
  endpointId: string;
  fingerprint: string;
  startedAt: number;
}

/**
 * Mirrors Paths::data_dir() in crates/goose/src/config/paths.rs:
 * - GOOSE_PATH_ROOT override -> <root>/data
 * - macOS/Linux (etcetera app strategy = XDG on both) -> $XDG_DATA_HOME or ~/.local/share, + /goose
 * - Windows -> %APPDATA%\Block\goose\data
 */
export const getRoamServeStatusPath = (goosePathRoot?: string): string => {
  if (goosePathRoot) {
    return path.join(goosePathRoot, 'data', 'roam', 'serve.json');
  }
  if (process.platform === 'win32') {
    const appData = process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming');
    return path.join(appData, 'Block', 'goose', 'data', 'roam', 'serve.json');
  }
  const xdgData = process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share');
  return path.join(xdgData, 'goose', 'roam', 'serve.json');
};

export const readRoamServeStatus = (goosePathRoot?: string): RoamServeStatus | null => {
  try {
    const raw = fs.readFileSync(getRoamServeStatusPath(goosePathRoot), 'utf8');
    const parsed = JSON.parse(raw);
    if (
      typeof parsed?.card === 'string' &&
      typeof parsed?.endpointId === 'string' &&
      typeof parsed?.fingerprint === 'string' &&
      typeof parsed?.startedAt === 'number'
    ) {
      return parsed as RoamServeStatus;
    }
    return null;
  } catch {
    return null;
  }
};
