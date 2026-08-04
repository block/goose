import { BrowserWindow, ipcMain, type IpcMainInvokeEvent } from 'electron';
import * as os from 'node:os';
import * as path from 'node:path';
import type { IPty } from 'node-pty';

export type TerminalKey = {
  sessionId: string;
  tabId: string;
};

type PtyRecord = {
  pty: IPty;
  webContentsId: number;
  sessionId: string;
  tabId: string;
};

const ptys = new Map<string, PtyRecord>();

function keyOf(sessionId: string, tabId: string): string {
  return `${sessionId}::${tabId}`;
}

function resolveShell(): { file: string; args: string[] } {
  const gooseShell = process.env.GOOSE_SHELL?.trim();
  if (gooseShell) {
    return { file: gooseShell, args: [] };
  }

  if (process.platform === 'win32') {
    const comspec = process.env.ComSpec || 'powershell.exe';
    if (comspec.toLowerCase().includes('powershell')) {
      return { file: comspec, args: ['-NoLogo'] };
    }
    return { file: comspec, args: [] };
  }

  const shell = process.env.SHELL || '/bin/zsh';
  // Login shell so PATH / profile match Terminal.app expectations on macOS.
  return { file: shell, args: ['-l'] };
}

function getNodePty(): typeof import('node-pty') {
  // Native module — must stay external to the Vite bundle.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  return require('node-pty') as typeof import('node-pty');
}

export function createPty(options: {
  sessionId: string;
  tabId: string;
  cwd: string;
  cols: number;
  rows: number;
  webContentsId: number;
}): { ok: true } | { ok: false; error: string } {
  const mapKey = keyOf(options.sessionId, options.tabId);
  const existing = ptys.get(mapKey);
  if (existing) {
    existing.webContentsId = options.webContentsId;
    try {
      existing.pty.resize(
        Math.max(2, Math.floor(options.cols)),
        Math.max(1, Math.floor(options.rows))
      );
    } catch {
      // ignore resize failures on attach
    }
    return { ok: true };
  }

  const cwd =
    options.cwd && options.cwd.trim().length > 0 ? path.resolve(options.cwd) : os.homedir();
  const { file, args } = resolveShell();

  try {
    const nodePty = getNodePty();
    const pty = nodePty.spawn(file, args, {
      name: 'xterm-256color',
      cols: Math.max(2, Math.floor(options.cols)),
      rows: Math.max(1, Math.floor(options.rows)),
      cwd,
      env: {
        ...process.env,
        TERM: 'xterm-256color',
        COLORTERM: 'truecolor',
      } as Record<string, string>,
    });

    const record: PtyRecord = {
      pty,
      webContentsId: options.webContentsId,
      sessionId: options.sessionId,
      tabId: options.tabId,
    };
    ptys.set(mapKey, record);

    pty.onData((data) => {
      const win = BrowserWindow.getAllWindows().find(
        (w) => w.webContents.id === record.webContentsId
      );
      if (!win || win.isDestroyed()) return;
      win.webContents.send('terminal-data', {
        sessionId: record.sessionId,
        tabId: record.tabId,
        data,
      });
    });

    pty.onExit(({ exitCode, signal }) => {
      ptys.delete(mapKey);
      const win = BrowserWindow.getAllWindows().find(
        (w) => w.webContents.id === record.webContentsId
      );
      if (!win || win.isDestroyed()) return;
      win.webContents.send('terminal-exit', {
        sessionId: record.sessionId,
        tabId: record.tabId,
        exitCode,
        signal,
      });
    });

    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export function writePty(sessionId: string, tabId: string, data: string): boolean {
  const record = ptys.get(keyOf(sessionId, tabId));
  if (!record) return false;
  record.pty.write(data);
  return true;
}

export function resizePty(sessionId: string, tabId: string, cols: number, rows: number): boolean {
  const record = ptys.get(keyOf(sessionId, tabId));
  if (!record) return false;
  try {
    record.pty.resize(Math.max(2, Math.floor(cols)), Math.max(1, Math.floor(rows)));
    return true;
  } catch {
    return false;
  }
}

export function killPty(sessionId: string, tabId: string): void {
  const mapKey = keyOf(sessionId, tabId);
  const record = ptys.get(mapKey);
  if (!record) return;
  ptys.delete(mapKey);
  try {
    record.pty.kill();
  } catch {
    // already dead
  }
}

export function killSessionPtys(sessionId: string): void {
  for (const [mapKey, record] of [...ptys.entries()]) {
    if (record.sessionId !== sessionId) continue;
    ptys.delete(mapKey);
    try {
      record.pty.kill();
    } catch {
      // ignore
    }
  }
}

export function killWindowPtys(webContentsId: number): void {
  for (const [mapKey, record] of [...ptys.entries()]) {
    if (record.webContentsId !== webContentsId) continue;
    ptys.delete(mapKey);
    try {
      record.pty.kill();
    } catch {
      // ignore
    }
  }
}

export function killAllPtys(): void {
  for (const [mapKey, record] of [...ptys.entries()]) {
    ptys.delete(mapKey);
    try {
      record.pty.kill();
    } catch {
      // ignore
    }
  }
}

export function registerPtyIpc(): void {
  ipcMain.handle(
    'terminal-create',
    (
      event: IpcMainInvokeEvent,
      payload: { sessionId: string; tabId: string; cwd: string; cols: number; rows: number }
    ) => {
      return createPty({
        ...payload,
        webContentsId: event.sender.id,
      });
    }
  );

  ipcMain.handle(
    'terminal-write',
    (_event, payload: { sessionId: string; tabId: string; data: string }) => {
      return writePty(payload.sessionId, payload.tabId, payload.data);
    }
  );

  ipcMain.handle(
    'terminal-resize',
    (_event, payload: { sessionId: string; tabId: string; cols: number; rows: number }) => {
      return resizePty(payload.sessionId, payload.tabId, payload.cols, payload.rows);
    }
  );

  ipcMain.handle(
    'terminal-kill',
    (_event, payload: { sessionId: string; tabId: string }) => {
      killPty(payload.sessionId, payload.tabId);
      return true;
    }
  );

  ipcMain.handle('terminal-kill-session', (_event, payload: { sessionId: string }) => {
    killSessionPtys(payload.sessionId);
    return true;
  });
}
