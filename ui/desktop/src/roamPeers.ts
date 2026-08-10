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

/**
 * Read a JSON state file. A missing file is `null` (empty state); a file that
 * exists but does not parse is a hard error — mirroring TrustBook::load's
 * fail-closed contract so a corrupt trust file is never silently replaced.
 */
const readJson = (file: string): unknown => {
  let raw: string;
  try {
    raw = fs.readFileSync(file, 'utf8');
  } catch (err) {
    if ((err as { code?: string }).code === 'ENOENT') return null;
    throw err;
  }
  return JSON.parse(raw);
};

const loadTrust = (goosePathRoot?: string): TrustFile => {
  const parsed = readJson(trustPath(goosePathRoot)) as Partial<TrustFile> | null;
  if (parsed !== null && (!Array.isArray(parsed.allowed) || !parsed.allowed.every(isString))) {
    throw new Error('trust file is malformed; refusing to modify it');
  }
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

/** Must match CARD_VERSION in crates/goose-roaming/src/card.rs. */
const CARD_VERSION = 1;

/** Mirror of ConnectionCard::decode — goose+roam://<base64url(JSON)>. */
/** Decode bounds — must match card.rs (the card-decoding contract). */
const MAX_CARD_TEXT_BYTES = 8 * 1024;
const MAX_RELAY_URLS = 16;
const MAX_RELAY_URL_BYTES = 512;

export const decodeRoamCard = (
  text: string
): { endpointId: string; relayUrls: string[]; fingerprint: string } | { error: string } => {
  const trimmed = text.trim();
  if (trimmed.length > MAX_CARD_TEXT_BYTES) {
    return { error: 'card too large' };
  }
  if (!trimmed.startsWith('goose+roam://')) {
    return { error: 'not a goose+roam:// card' };
  }
  try {
    const b64 = trimmed.slice('goose+roam://'.length);
    // Node's base64url decoding is permissive (ignores junk); reject anything
    // outside the alphabet up front so decode matches card.rs strictness.
    if (!/^[A-Za-z0-9_-]+$/.test(b64)) {
      return { error: 'malformed card' };
    }
    const parsed = JSON.parse(Buffer.from(b64, 'base64url').toString('utf8')) as {
      version?: number;
      endpoint_id?: string;
      relay_urls?: unknown;
    };
    if (parsed.version !== CARD_VERSION) {
      return { error: `unsupported card version ${parsed.version}` };
    }
    if (!isString(parsed.endpoint_id) || !/^[0-9a-f]{64}$/.test(parsed.endpoint_id)) {
      return { error: 'card has no valid endpoint id' };
    }
    if (!Array.isArray(parsed.relay_urls) || !parsed.relay_urls.every(isString)) {
      return { error: 'card has no valid relay url list' };
    }
    const relayUrls = parsed.relay_urls;
    if (relayUrls.length > MAX_RELAY_URLS) {
      return { error: 'too many relay urls' };
    }
    for (const url of relayUrls) {
      if (url.length > MAX_RELAY_URL_BYTES) {
        return { error: 'relay url too long' };
      }
      if (!url.startsWith('https://') && !url.startsWith('http://')) {
        return { error: `relay url must be http(s): ${url}` };
      }
    }
    return {
      endpointId: parsed.endpoint_id,
      relayUrls,
      fingerprint: roamFingerprint(parsed.endpoint_id),
    };
  } catch {
    return { error: 'malformed card' };
  }
};

/**
 * Mirror of `goose roam peers accept '<card>' <name>`: save the card to the
 * address book (roaming_peers.json, PeerBook shape) and put its key on the
 * allowlist. The running share picks it up on the peer's next connection —
 * same file seam as revoke, no IPC to the backend.
 */
export const acceptRoamPeer = (
  cardText: string,
  name: string | undefined,
  goosePathRoot?: string
): { name: string; endpointId: string; fingerprint: string } | { error: string } => {
  const card = decodeRoamCard(cardText);
  if ('error' in card) return card;

  const peerName = (name ?? '').trim() || `device-${card.endpointId.slice(0, 12)}`;

  // Fail closed: a peers/trust file that exists but does not parse aborts the
  // accept (readJson/loadTrust throw) rather than being overwritten. Both are
  // loaded before either write so a corrupt trust file can't strand a
  // half-done accept.
  const file = peersPath(goosePathRoot);
  let book: { peers?: Record<string, unknown> };
  let trust: TrustFile;
  try {
    book = (readJson(file) as { peers?: Record<string, unknown> } | null) ?? {};
    trust = loadTrust(goosePathRoot);
  } catch {
    return { error: 'trust or address book file is unreadable or corrupt; refusing to modify it' };
  }
  const peers = (book.peers ?? {}) as Record<string, unknown>;
  peers[peerName] = {
    name: peerName,
    card: {
      version: CARD_VERSION,
      endpoint_id: card.endpointId,
      relay_urls: card.relayUrls,
    },
    endpoint_id: card.endpointId,
    fingerprint: card.fingerprint,
    added_ms: Date.now(),
  };
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const tmp = `${file}.tmp-${process.pid}`;
  fs.writeFileSync(tmp, JSON.stringify({ peers }, null, 2));
  fs.renameSync(tmp, file);

  // TrustBook::accept semantics: clear any prior revocation, then allow.
  trust.revoked_keys = trust.revoked_keys.filter((k) => k !== card.endpointId);
  trust.allowed.push(card.endpointId);
  saveTrust(trust, goosePathRoot);

  return { name: peerName, endpointId: card.endpointId, fingerprint: card.fingerprint };
};
