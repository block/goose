import { useSyncExternalStore } from 'react';

// Local backends are per-window, so neither the server nor this renderer's ACP
// snapshot can see a prompt run owned by another desktop window. The main
// process relays each window's per-session stream status ('broadcast-session-
// status' -> 'remote-session-status-update') and tells us to drop a window's
// statuses when it closes or reloads ('remote-session-status-cleared'). Archive
// guards consult this registry so a session busy in another window still counts
// as busy here.

const BUSY_STREAM_STATES = new Set(['loading', 'streaming', 'waiting']);

const busyByWindow = new Map<number, Map<string, string>>();
const listeners = new Set<() => void>();
let subscribedToIpc = false;

function notify() {
  listeners.forEach((listener) => listener());
}

function handleRemoteStatus(_event: unknown, payload: unknown) {
  const { windowId, sessionId, streamState } = (payload ?? {}) as {
    windowId?: number;
    sessionId?: string;
    streamState?: string;
  };
  if (typeof windowId !== 'number' || typeof sessionId !== 'string') return;

  let byWindow = busyByWindow.get(windowId);
  if (streamState !== undefined && BUSY_STREAM_STATES.has(streamState)) {
    if (!byWindow) {
      byWindow = new Map();
      busyByWindow.set(windowId, byWindow);
    }
    byWindow.set(sessionId, streamState);
  } else {
    if (!byWindow?.delete(sessionId)) return;
  }
  notify();
}

function handleRemoteCleared(_event: unknown, payload: unknown) {
  const { windowId } = (payload ?? {}) as { windowId?: number };
  if (typeof windowId !== 'number') return;
  if (busyByWindow.delete(windowId)) notify();
}

function ensureIpcSubscription() {
  if (subscribedToIpc || !window.electron?.on) return;
  subscribedToIpc = true;
  window.electron.on('remote-session-status-update', handleRemoteStatus);
  window.electron.on('remote-session-status-cleared', handleRemoteCleared);
  // The main process only forwards live transitions; seed from its cached busy
  // map so runs that started before this window subscribed still count.
  window.electron
    .getRemoteSessionStatuses?.()
    .then((statuses) => {
      statuses.forEach((status) => handleRemoteStatus(undefined, status));
    })
    .catch((error) => console.error('Failed to seed remote session statuses:', error));
}

function subscribe(listener: () => void): () => void {
  ensureIpcSubscription();
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function isSessionBusyElsewhere(sessionId: string): boolean {
  for (const byWindow of busyByWindow.values()) {
    if (byWindow.has(sessionId)) return true;
  }
  return false;
}

export function useSessionBusyElsewhere(sessionId: string): boolean {
  return useSyncExternalStore(subscribe, () => isSessionBusyElsewhere(sessionId));
}
