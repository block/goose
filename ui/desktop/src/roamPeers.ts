import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * Read/modify the roam trust state that `goose roam` maintains:
 * - roaming_peers.json (address book: saved cards + nicknames)
 * - roaming_trust.json (allowlist + revocations)
 *
 * A running `goose serve --roam` / `goose roam share` watches the trust file
 * (mtime poll, ~2s) and force-closes live connections for revoked keys, so a
 * revoke written here takes effect on a live share without any IPC to it.
 * Writes are atomic (temp + rename) to match TrustBook::save semantics — the
 * watcher must never observe a half-written file.
 */

export interface RoamPeer {
  name: string | null;
  endpointId: string;
  fingerprint: string;
  accepted: boolean;
  addedMs: number | null;
}

/**
 * Mirrors Paths::config_dir() in crates/goose/src/config/paths.rs:
 * - GOOSE_PATH_ROOT override -> <root>/config
 * - macOS/Linux (etcetera app strategy = XDG on both) -> $XDG_CONFIG_HOME or ~/.config, + /goose
 * - Windows -> %APPDATA%\Block\goose\config
 */
const configDir = (goosePathRoot?: string): string => {
  if (goosePathRoot) {
    return path.join(goosePathRoot, 'config');
  }
  if (process.platform === 'win32') {
    const appData = process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming');
    return path.join(appData, 'Block', 'goose', 'config');
  }
  const xdgConfig = process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config');
  return path.join(xdgConfig, 'goose');
};

const trustPath = (goosePathRoot?: string): string =>
  path.join(configDir(goosePathRoot), 'roaming_trust.json');

const peersPath = (goosePathRoot?: string): string =>
  path.join(configDir(goosePathRoot), 'roaming_peers.json');

interface TrustFile {
  allowed: string[];
  revoked_keys: string[];
}

const readJson = (file: string): unknown => {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
};

const loadTrust = (goosePathRoot?: string): TrustFile => {
  const parsed = readJson(trustPath(goosePathRoot)) as Partial<TrustFile> | null;
  return {
    allowed: Array.isArray(parsed?.allowed) ? parsed.allowed.filter(isString) : [],
    revoked_keys: Array.isArray(parsed?.revoked_keys) ? parsed.revoked_keys.filter(isString) : [],
  };
};

const isString = (v: unknown): v is string => typeof v === 'string';

/**
 * Same derivation as ConnectionCard::fingerprint (crates/goose-roaming/src/card.rs):
 * SHA-256 of the raw 32-byte endpoint key (`endpoint_id.as_bytes()`, not its hex
 * form), first 16 bytes as eight 4-hex groups. Computed fresh so the display
 * never shows the stale short fingerprints saved by older builds.
 */
export const roamFingerprint = (endpointId: string): string => {
  const digest = crypto.createHash('sha256').update(Buffer.from(endpointId, 'hex')).digest();
  const groups: string[] = [];
  for (let i = 0; i < 16; i += 2) {
    groups.push(digest.subarray(i, i + 2).toString('hex'));
  }
  return groups.join('-');
};

export const listRoamPeers = (goosePathRoot?: string): RoamPeer[] => {
  const trust = loadTrust(goosePathRoot);
  const accepted = new Set(trust.allowed);

  const peersFile = readJson(peersPath(goosePathRoot)) as {
    peers?: Record<string, { name?: string; endpoint_id?: string; added_ms?: number }>;
  } | null;

  const out: RoamPeer[] = [];
  const seen = new Set<string>();
  for (const rec of Object.values(peersFile?.peers ?? {})) {
    const endpointId = rec.endpoint_id;
    if (!isString(endpointId)) continue;
    seen.add(endpointId);
    out.push({
      name: isString(rec.name) ? rec.name : null,
      endpointId,
      fingerprint: roamFingerprint(endpointId),
      accepted: accepted.has(endpointId),
      addedMs: typeof rec.added_ms === 'number' ? rec.added_ms : null,
    });
  }
  // Keys accepted by raw id with no saved card still grant access — show them.
  for (const id of trust.allowed) {
    if (!seen.has(id)) {
      out.push({
        name: null,
        endpointId: id,
        fingerprint: roamFingerprint(id),
        accepted: true,
        addedMs: null,
      });
    }
  }
  out.sort((a, b) => (b.addedMs ?? 0) - (a.addedMs ?? 0));
  return out;
};

const saveTrust = (trust: TrustFile, goosePathRoot?: string): void => {
  const file = trustPath(goosePathRoot);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const body: TrustFile = {
    allowed: [...new Set(trust.allowed)].sort(),
    revoked_keys: [...new Set(trust.revoked_keys)].sort(),
  };
  const tmp = `${file}.tmp-${process.pid}`;
  fs.writeFileSync(tmp, JSON.stringify(body, null, 2));
  fs.renameSync(tmp, file);
};

/** Mirror of TrustBook::revoke_key: drop from allowlist, pin in revoked set. */
export const revokeRoamPeer = (endpointId: string, goosePathRoot?: string): boolean => {
  if (!/^[0-9a-f]{64}$/.test(endpointId)) return false;
  const trust = loadTrust(goosePathRoot);
  if (!trust.allowed.includes(endpointId)) return false;
  trust.allowed = trust.allowed.filter((k) => k !== endpointId);
  trust.revoked_keys.push(endpointId);
  saveTrust(trust, goosePathRoot);
  return true;
};
