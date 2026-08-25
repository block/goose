import { EventEmitter } from 'node:events';
import path from 'node:path';
import { BrowserWindow, ipcMain } from 'electron';
import type { DiscoveredClientExtension } from '../client-extensions/types';
import {
  PLUGIN_SESSION_EVENT_CHANNEL,
  type PluginSessionEvent,
} from '../client-extensions/plugin-events';

export type { PluginSessionEvent };

const sessionEmitter = new EventEmitter();
sessionEmitter.setMaxListeners(0);

ipcMain.on(PLUGIN_SESSION_EVENT_CHANNEL, (_event, data: PluginSessionEvent) => {
  sessionEmitter.emit(data.type, data);
});

// Deliberately narrow — plugins declare what they need; surface grows over time.
export interface PluginContext {
  extensionId: string;
  onDispose: (fn: () => void) => void;
  ipc: {
    handle: (channel: string, handler: (...args: unknown[]) => unknown) => void;
    on: (channel: string, handler: (...args: unknown[]) => void) => void;
    broadcast: (channel: string, ...args: unknown[]) => void;
    send: (windowId: number, channel: string, ...args: unknown[]) => void;
  };
  session: {
    on: (
      type: PluginSessionEvent['type'],
      handler: (event: PluginSessionEvent) => void
    ) => () => void;
  };
}

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
      console.error(`[plugin-host] "${extension.id}" host module has no apply() export`);
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
        // Bust require cache to allow hot-reload on next load
        delete require.cache[hostPath];
      },
    });
  } catch (err) {
    console.error(`[plugin-host] Failed to load "${extension.id}":`, err);
    for (const fn of [...disposables].reverse()) {
      try { fn(); } catch {}
    }
    delete require.cache[hostPath];
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
}

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
