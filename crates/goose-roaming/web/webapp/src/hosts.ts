// Saved-host bookkeeping: localStorage keys, card parsing helpers, and the
// shapes for saved hosts and live connections.
import type { GooseClient } from "@aaif/goose-sdk";
import type { RoamConnection } from "./wasm/goose_roaming_web.js";

export const HOST_CARD_KEY = "goose-roam-last-host-card";
export const HOSTS_KEY = "goose-roam-hosts";

// Kept most-recently-used-first by the connect path (unshift on reconnect).
export type SavedHost = { name: string; card: string; endpointId: string };

// One live roam connection. The tab holds several at once — each saved host
// gets its own iroh duplex + GooseClient, and the session list is the merge.
export type HostConn = {
  endpointId: string;
  name: string;
  agent: GooseClient;
  /** Held to keep the QUIC connection alive for the life of the entry. */
  conn: RoamConnection;
  relay: string | null;
};

export function relayRegion(cardText: string): string | null {
  try {
    const b64 = cardText.trim().replace(/^goose\+roam:\/\//, "");
    const json = JSON.parse(atob(b64.replace(/-/g, "+").replace(/_/g, "/")));
    const url: string | undefined = json.relay_urls?.[0];
    const m = url?.match(/^https?:\/\/([a-z0-9-]+)\./i);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}

export function loadHosts(): SavedHost[] {
  try {
    return JSON.parse(localStorage.getItem(HOSTS_KEY) ?? "[]");
  } catch {
    return [];
  }
}

// Endpoint id straight out of the card JSON — for keying reconnect attempts
// when the dial itself failed (no RoamConnection to ask).
export function cardEndpointHint(cardText: string): string | null {
  try {
    const b64 = cardText.trim().replace(/^goose\+roam:\/\//, "");
    const json = JSON.parse(atob(b64.replace(/-/g, "+").replace(/_/g, "/")));
    return typeof json.endpoint_id === "string" ? json.endpoint_id : null;
  } catch {
    return null;
  }
}

// Last-open session is remembered per host: "<endpointId>|<sessionId>".
export const SESSION_KEY = "goose-roam-last-session";

// Collapsed project groups persist per device.
export const COLLAPSED_KEY = "goose-roam-collapsed-projects";

export function loadCollapsed(): Set<string> {
  try {
    const raw = localStorage.getItem(COLLAPSED_KEY);
    if (raw) return new Set(JSON.parse(raw) as string[]);
  } catch {
    // ignore malformed state
  }
  return new Set();
}