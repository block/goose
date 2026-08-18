import { EventEmitter } from 'node:events';
import path from 'node:path';
import { BrowserWindow, ipcMain } from 'electron';
import type { DiscoveredClientExtension } from '../client-extensions/types';
import {
  PLUGIN_SESSION_EVENT_CHANNEL,
  type PluginSessionEvent,
} from '../client-extensions/plugin-events';

export type { PluginSessionEvent };

// Module-level emitter: all loaded plugins share one source of truth.
// Keyed by PluginSessionEvent['type'] (plus the '*' catch-all).
const sessionEmitter = new EventEmitter();
sessionEmitter.setMaxListeners(0);

// Register once — renderer sends events here whenever ACP notifications arrive.
ipcMain.on(PLUGIN_SESSION_EVENT_CHANNEL, (_event, data: PluginSessionEvent) => {
  sessionEmitter.emit(data.type, data);
});

// The context handed to every host plugin's apply() function.
// Deliberately narrow: plugins ask for what they need, we extend this over time.
export interface PluginContext {
  extensionId: string;

  // Register cleanup logic — called when the plugin is unloaded.
  onDispose: (fn: () => void) => void;

  // Namespaced IPC — all channels are automatically prefixed with
  // `plugin:<extensionId>:` so plugins can't collide with each other or core.
  ipc: {
    // Register a request/response handler callable from the renderer.
    handle: (channel: string, handler: (...args: unknown[]) => unknown) => void;
    // Register a fire-and-forget listener from the renderer.
    on: (channel: string, handler: (...args: unknown[]) => void) => void;
    // Push an event to all renderer windows.
    broadcast: (channel: string, ...args: unknown[]) => void;
    // Push an event to a specific window by id.
    send: (windowId: number, channel: string, ...args: unknown[]) => void;
  };

  // Subscribe to live ACP session events forwarded from the renderer.
  // The returned function removes the listener — or call onDispose to clean up automatically.
  session: {
    on: (
      type: PluginSessionEvent['type'],
      handler: (event: PluginSessionEvent) => void
    ) => () => void;
  };
}

// Shape of a host plugin module. CommonJS: module.exports = { apply }
interface PluginModule {
  apply: (ctx: PluginContext) => (() => void) | void;
}

interface LoadedPlugin {
  extensionId: string;
  hostPath: string;
  dispose: () => void;
}

const loaded = new Map<string, LoadedPlugin>();

function buildContext(extensionId: string, disposables: (() => void)[]): PluginContext {
  const prefix = `plugin:${extensionId}:`;

  return {
    extensionId,

    onDispose(fn) {
      disposables.push(fn);
    },

    ipc: {
      handle(channel, handler) {
        const ch = prefix + channel;
        ipcMain.handle(ch, (_event, ...args) => handler(...args));
        disposables.push(() => ipcMain.removeHandler(ch));
      },

      on(channel, handler) {
        const ch = prefix + channel;
        const wrapped = (_event: Electron.IpcMainEvent, ...args: unknown[]) => handler(...args);
        ipcMain.on(ch, wrapped);
        disposables.push(() => ipcMain.removeListener(ch, wrapped));
      },

      broadcast(channel, ...args) {
        const ch = prefix + channel;
        for (const win of BrowserWindow.getAllWindows()) {
          if (!win.isDestroyed()) {
            win.webContents.send(ch, ...args);
          }
        }
      },

      send(windowId, channel, ...args) {
        const ch = prefix + channel;
        const win = BrowserWindow.fromId(windowId);
        if (win && !win.isDestroyed()) {
          win.webContents.send(ch, ...args);
        }
      },
    },

    session: {
      on(type, handler) {
        sessionEmitter.on(type, handler);
        const remove = () => sessionEmitter.off(type, handler);
        disposables.push(remove);
        return remove;
      },
    },
  };
}

export function loadPlugin(extension: DiscoveredClientExtension): void {
  if (!extension.manifest.host || loaded.has(extension.id)) return;

  const hostPath = path.resolve(extension.rootPath, extension.manifest.host);
  const disposables: (() => void)[] = [];

  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const mod = require(hostPath) as PluginModule;
    if (typeof mod.apply !== 'function') {
      console.warn(`[plugin-host] "${extension.id}" host module has no apply() export`);
      return;
    }

    const ctx = buildContext(extension.id, disposables);
    const returned = mod.apply(ctx);
    if (typeof returned === 'function') {
      disposables.push(returned);
    }

    loaded.set(extension.id, {
      extensionId: extension.id,
      hostPath,
      dispose() {
        for (const fn of [...disposables].reverse()) {
          try {
            fn();
          } catch (err) {
            console.error(`[plugin-host] Dispose error in "${extension.id}":`, err);
          }
        }
        disposables.length = 0;
        // Allow hot-reload by busting the require cache
        delete require.cache[hostPath];
      },
    });

    console.log(`[plugin-host] Loaded "${extension.id}"`);
  } catch (err) {
    console.error(`[plugin-host] Failed to load "${extension.id}":`, err);
  }
}

export function unloadPlugin(extensionId: string): void {
  const plugin = loaded.get(extensionId);
  if (!plugin) return;
  try {
    plugin.dispose();
  } catch (err) {
    console.error(`[plugin-host] Error unloading "${extensionId}":`, err);
  }
  loaded.delete(extensionId);
  console.log(`[plugin-host] Unloaded "${extensionId}"`);
}

// Called after any extension state change to reconcile loaded plugins with
// the current enabled set.
export function syncPlugins(extensions: DiscoveredClientExtension[]): void {
  const enabledWithHost = new Set(
    extensions.filter((e) => e.enabled && e.manifest.host).map((e) => e.id)
  );

  for (const id of loaded.keys()) {
    if (!enabledWithHost.has(id)) unloadPlugin(id);
  }

  for (const ext of extensions) {
    if (ext.enabled && ext.manifest.host && !loaded.has(ext.id)) {
      loadPlugin(ext);
    }
  }
}

export function unloadAllPlugins(): void {
  for (const id of [...loaded.keys()]) {
    unloadPlugin(id);
  }
}
