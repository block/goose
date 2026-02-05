/**
 * Pi Agent integration for Electron.
 *
 * This module provides a bridge between the Electron app and Pi,
 * running Pi directly in the Node.js main process.
 *
 * Architecture:
 * - Pi runs in the Electron main process
 * - IPC handlers expose Pi to the renderer
 * - Events are translated to Goose-compatible format
 * - MCP extensions configured via goosed are passed to Pi
 */

import { ipcMain, IpcMainInvokeEvent } from 'electron';
import { appendFileSync } from 'node:fs';
import log from '../utils/logger';
import { PiEventAccumulator, type PiAgentEvent, type GooseMessage } from './eventTranslator';

const PI_LOG = '/tmp/pi-debug.log';
const piLog = (msg: string, ...args: unknown[]) => {
  const line = `[${new Date().toISOString()}] ${msg} ${args.map(a => JSON.stringify(a)).join(' ')}\n`;
  console.log('[Pi]', msg, ...args);
  try { appendFileSync(PI_LOG, line); } catch {}
};

// Pi types - these match the actual @mariozechner/pi-coding-agent exports
import type {
  AgentSession,
  CreateAgentSessionOptions,
  CreateAgentSessionResult,
  AgentSessionEvent,
} from '@mariozechner/pi-coding-agent';

interface PiModule {
  createAgentSession: (options?: CreateAgentSessionOptions) => Promise<CreateAgentSessionResult>;
  VERSION: string;
}

// Module state
let piModule: PiModule | null = null;
let currentSession: AgentSession | null = null;
let unsubscribeFn: (() => void) | null = null;
let eventAccumulator = new PiEventAccumulator();

/**
 * Initialize Pi module.
 * Called once at app startup.
 */
export async function initializePi(): Promise<boolean> {
  try {
    piLog('Attempting to load Pi module...');
    piModule = (await import('@mariozechner/pi-coding-agent')) as PiModule;
    piLog('Module loaded successfully, version:', piModule.VERSION);
    return true;
  } catch (error) {
    piLog('Failed to load module:', error);
    return false;
  }
}

/**
 * Check if Pi is available.
 */
export function isPiAvailable(): boolean {
  return piModule !== null;
}

/**
 * Get Pi version.
 */
export function getPiVersion(): string | null {
  return piModule?.VERSION || null;
}

export interface StartSessionOptions {
  workingDir?: string;
}

/**
 * Start a new Pi session.
 */
export async function startPiSession(options: StartSessionOptions = {}): Promise<void> {
  piLog('startPiSession called with options:', options);
  if (!piModule) {
    piLog('ERROR: Pi module not loaded');
    throw new Error('Pi module not loaded. Call initializePi() first.');
  }

  // Stop existing session if any
  if (currentSession) {
    await stopPiSession();
  }

  // Set working directory
  if (options.workingDir) {
    process.chdir(options.workingDir);
  }

  const sessionConfig: CreateAgentSessionOptions = {
    cwd: options.workingDir || process.cwd(),
  };

  piLog('Creating agent session with config:', sessionConfig);
  const result = await piModule.createAgentSession(sessionConfig);
  piLog('Session created, result:', { hasSession: !!result.session });
  currentSession = result.session;
  eventAccumulator.reset();

  piLog('Session started successfully');
}

/**
 * Stop the current Pi session.
 */
export async function stopPiSession(): Promise<void> {
  if (unsubscribeFn) {
    unsubscribeFn();
    unsubscribeFn = null;
  }
  if (currentSession) {
    try {
      currentSession.abort();
    } catch {
      // Ignore abort errors
    }
    currentSession = null;
    eventAccumulator.reset();
    log.info('[Pi] Session stopped');
  }
}

/**
 * Send a prompt to Pi and stream events via callback.
 */
export async function promptPi(
  message: string,
  onEvent: (event: PiAgentEvent, gooseMessage?: GooseMessage) => void,
  onComplete: () => void,
  onError: (error: Error) => void
): Promise<void> {
  piLog('promptPi called with message:', message.substring(0, 100));
  if (!currentSession) {
    piLog('ERROR: No Pi session');
    onError(new Error('No Pi session. Call startPiSession() first.'));
    return;
  }

  eventAccumulator.reset();

  // Subscribe to events
  if (unsubscribeFn) {
    unsubscribeFn();
  }
  piLog('Subscribing to session events...');
  unsubscribeFn = currentSession.subscribe((event: AgentSessionEvent) => {
    const piEvent = event as PiAgentEvent;
    piLog('Received event:', piEvent.type);
    const result = eventAccumulator.processEvent(piEvent);
    onEvent(piEvent, result.message);

    if (result.isComplete) {
      piLog('Event stream complete');
      if (unsubscribeFn) {
        unsubscribeFn();
        unsubscribeFn = null;
      }
      onComplete();
    }
  });

  try {
    piLog('Calling currentSession.prompt()...');
    await currentSession.prompt(message);
    piLog('prompt() returned');
  } catch (error) {
    piLog('prompt() threw error:', error);
    if (unsubscribeFn) {
      unsubscribeFn();
      unsubscribeFn = null;
    }
    onError(error instanceof Error ? error : new Error(String(error)));
  }
}

/**
 * Abort current Pi operation.
 */
export function abortPi(): void {
  if (currentSession) {
    currentSession.abort();
  }
}

/**
 * Get Pi state.
 */
export function getPiState(): {
  available: boolean;
  version: string | null;
  hasSession: boolean;
  isStreaming: boolean;
} {
  return {
    available: isPiAvailable(),
    version: getPiVersion(),
    hasSession: currentSession !== null,
    isStreaming: currentSession?.state.isStreaming || false,
  };
}

/**
 * Register IPC handlers for Pi.
 * Call this from main.ts after app is ready.
 */
export function registerPiIpcHandlers(): void {
  // Check if Pi is available
  ipcMain.handle('pi:isAvailable', () => {
    piLog('IPC pi:isAvailable called, returning:', isPiAvailable());
    return isPiAvailable();
  });

  // Get Pi version
  ipcMain.handle('pi:getVersion', () => getPiVersion());

  // Get Pi state
  ipcMain.handle('pi:getState', () => getPiState());

  // Start Pi session
  ipcMain.handle('pi:startSession', async (_event: IpcMainInvokeEvent, options: StartSessionOptions) => {
    piLog('IPC pi:startSession called with options:', options);
    try {
      await startPiSession(options);
      return { success: true };
    } catch (error) {
      piLog('IPC pi:startSession error:', error);
      return { success: false, error: String(error) };
    }
  });

  // Stop Pi session
  ipcMain.handle('pi:stopSession', async () => {
    await stopPiSession();
    return { success: true };
  });

  // Abort current operation
  ipcMain.handle('pi:abort', () => {
    abortPi();
    return { success: true };
  });

  // Send prompt - streams events back via IPC
  ipcMain.handle('pi:prompt', async (event: IpcMainInvokeEvent, message: string) => {
    const webContents = event.sender;

    return new Promise<{ success: boolean; error?: string }>((resolve) => {
      promptPi(
        message,
        // onEvent
        (piEvent, gooseMessage) => {
          // Send raw Pi event
          webContents.send('pi:event', piEvent);

          // Send translated Goose message if available
          if (gooseMessage) {
            webContents.send('pi:message', gooseMessage);
          }
        },
        // onComplete
        () => {
          webContents.send('pi:complete');
          resolve({ success: true });
        },
        // onError
        (error) => {
          webContents.send('pi:error', { error: error.message });
          resolve({ success: false, error: error.message });
        }
      );
    });
  });

  log.info('[Pi] IPC handlers registered');
}

// Re-export types and translator
export * from './eventTranslator';
