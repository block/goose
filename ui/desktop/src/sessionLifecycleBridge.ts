import { AppEvents } from './constants/events';

// Archive/delete mutate the shared session store, but window.dispatchEvent only
// reaches the current renderer — another desktop window with the same chat open
// would keep talking to a session that is now archived or gone. Relay these
// lifecycle events through the main process so every window runs its normal
// App/sidebar cleanup. Remote re-dispatches go through window.dispatchEvent
// directly (not dispatchSessionLifecycleEvent), so they are not re-broadcast.

const RELAYED_EVENTS: ReadonlySet<string> = new Set([
  AppEvents.SESSION_ARCHIVED,
  AppEvents.SESSION_DELETED,
]);

let subscribed = false;

export function dispatchSessionLifecycleEvent(name: string, detail: unknown): void {
  window.dispatchEvent(new CustomEvent(name, { detail }));
  if (RELAYED_EVENTS.has(name)) {
    window.electron?.broadcastSessionLifecycle?.({ name, detail });
  }
}

export function initSessionLifecycleBridge(): void {
  if (subscribed || !window.electron?.on) return;
  subscribed = true;
  window.electron.on('remote-session-lifecycle', (_event, payload) => {
    const { name, detail } = (payload ?? {}) as { name?: string; detail?: unknown };
    if (typeof name !== 'string' || !RELAYED_EVENTS.has(name)) return;
    window.dispatchEvent(new CustomEvent(name, { detail }));
  });
}
