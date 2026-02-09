/**
 * Pi Agent integration for Electron.
 *
 * This module provides a bridge between the Electron app and Pi,
 * running Pi directly in the Node.js main process.
 *
 * Architecture:
 * - Pi runs in the Electron main process
 * - Sessions are managed and persisted by this module
 * - IPC handlers expose Pi to the renderer
 * - Events are translated to Goose-compatible format
 */

import { ipcMain, IpcMainInvokeEvent, app } from 'electron';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import log from '../utils/logger';
import { PiEventAccumulator, type PiAgentEvent, type GooseMessage, type ToolNotification, translateGooseMessageToPi, type PiMessage } from './eventTranslator';

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

// Session data structure (matches Goose Session type for UI compatibility)
export interface PiSession {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  working_dir: string;
  message_count: number;
  conversation: GooseMessage[];
  // Token tracking
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  accumulated_input_tokens: number;
  accumulated_output_tokens: number;
  accumulated_total_tokens: number;
  // Goosed integration
  goosedUrl?: string;  // URL of goosed for this session
}

// Module state
let piModule: PiModule | null = null;
let currentAgentSession: AgentSession | null = null;
let currentSessionId: string | null = null;
let unsubscribeFn: (() => void) | null = null;
let eventAccumulator = new PiEventAccumulator();

// Session storage
function getSessionsDir(): string {
  const userDataPath = app.getPath('userData');
  const sessionsDir = join(userDataPath, 'pi-sessions');
  if (!existsSync(sessionsDir)) {
    mkdirSync(sessionsDir, { recursive: true });
  }
  return sessionsDir;
}

function getSessionPath(sessionId: string): string {
  return join(getSessionsDir(), `${sessionId}.json`);
}

function generateSessionId(): string {
  const now = new Date();
  const date = now.toISOString().slice(0, 10).replace(/-/g, '');
  const time = now.toISOString().slice(11, 19).replace(/:/g, '');
  return `${date}_${time}`;
}

function generateSessionName(messages: GooseMessage[]): string {
  // Find the first user message to generate a name from
  const firstUserMsg = messages.find(m => m.role === 'user');
  if (firstUserMsg) {
    const textContent = firstUserMsg.content.find(c => c.type === 'text');
    if (textContent && 'text' in textContent) {
      const text = textContent.text.slice(0, 50);
      return text.length < textContent.text.length ? `${text}...` : text;
    }
  }
  return 'New Chat';
}

/**
 * Save session to disk.
 */
function saveSession(session: PiSession): void {
  const path = getSessionPath(session.id);
  writeFileSync(path, JSON.stringify(session, null, 2));
  piLog('Session saved:', session.id);
}

/**
 * Load session from disk.
 */
function loadSession(sessionId: string): PiSession | null {
  const path = getSessionPath(sessionId);
  if (!existsSync(path)) {
    return null;
  }
  try {
    const data = readFileSync(path, 'utf-8');
    return JSON.parse(data) as PiSession;
  } catch (error) {
    piLog('Failed to load session:', sessionId, error);
    return null;
  }
}

/**
 * List all sessions.
 */
function listAllSessions(): PiSession[] {
  const sessionsDir = getSessionsDir();
  const files = readdirSync(sessionsDir).filter(f => f.endsWith('.json'));
  const sessions: PiSession[] = [];
  
  for (const file of files) {
    try {
      const data = readFileSync(join(sessionsDir, file), 'utf-8');
      sessions.push(JSON.parse(data) as PiSession);
    } catch {
      // Skip invalid files
    }
  }
  
  // Sort by updated_at descending (most recent first)
  sessions.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
  return sessions;
}

/**
 * Delete a session.
 */
function deleteSessionFile(sessionId: string): boolean {
  const path = getSessionPath(sessionId);
  if (existsSync(path)) {
    unlinkSync(path);
    return true;
  }
  return false;
}

// In-memory session state for current session
let currentSession: PiSession | null = null;

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

export interface CreateSessionOptions {
  workingDir?: string;
  goosedUrl?: string;  // URL of goosed server for session storage
}

/**
 * Create session in goosed's SQLite database.
 */
async function createGoosedSession(goosedUrl: string, workingDir: string): Promise<{ id: string; created_at: string }> {
  const response = await fetch(`${goosedUrl}/sessions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ working_dir: workingDir }),
  });
  
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Failed to create session in goosed: ${response.status} ${text}`);
  }
  
  const session = await response.json();
  return { id: session.id, created_at: session.created_at };
}

/**
 * Save conversation to goosed's SQLite database.
 */
async function saveConversationToGoosed(goosedUrl: string, sessionId: string, messages: GooseMessage[]): Promise<void> {
  const response = await fetch(`${goosedUrl}/sessions/${sessionId}/conversation`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ messages }),
  });
  
  if (!response.ok) {
    const text = await response.text();
    piLog('Failed to save conversation to goosed:', response.status, text);
    // Don't throw - we still have local state
  }
}

/**
 * Load session from goosed's SQLite database.
 */
async function loadGoosedSession(goosedUrl: string, sessionId: string): Promise<PiSession | null> {
  try {
    const response = await fetch(`${goosedUrl}/sessions/${sessionId}`);
    if (!response.ok) {
      return null;
    }
    
    const session = await response.json();
    return {
      id: session.id,
      name: session.name,
      created_at: session.created_at,
      updated_at: session.updated_at,
      working_dir: session.working_dir,
      message_count: session.message_count || 0,
      conversation: session.conversation?.messages || [],
      input_tokens: session.input_tokens || 0,
      output_tokens: session.output_tokens || 0,
      total_tokens: session.total_tokens || 0,
      accumulated_input_tokens: session.accumulated_input_tokens || 0,
      accumulated_output_tokens: session.accumulated_output_tokens || 0,
      accumulated_total_tokens: session.accumulated_total_tokens || 0,
      goosedUrl,
    };
  } catch (error) {
    piLog('Failed to load session from goosed:', error);
    return null;
  }
}

/**
 * Create a new Pi session.
 */
export async function createPiSession(options: CreateSessionOptions = {}): Promise<PiSession> {
  piLog('createPiSession called with options:', options);
  if (!piModule) {
    piLog('ERROR: Pi module not loaded');
    throw new Error('Pi module not loaded. Call initializePi() first.');
  }

  // Stop existing agent session if any
  if (currentAgentSession) {
    await stopCurrentAgentSession();
  }

  const workingDir = options.workingDir || process.cwd();
  const goosedUrl = options.goosedUrl;
  
  // Set working directory
  process.chdir(workingDir);

  const sessionConfig: CreateAgentSessionOptions = {
    cwd: workingDir,
  };

  piLog('Creating agent session with config:', sessionConfig);
  const result = await piModule.createAgentSession(sessionConfig);
  piLog('Agent session created, result:', { hasSession: !!result.session });
  
  currentAgentSession = result.session;
  eventAccumulator.reset();

  // Create session - use goosed if URL provided, otherwise fall back to local storage
  let sessionId: string;
  let createdAt: string;
  
  if (goosedUrl) {
    piLog('Creating session in goosed at:', goosedUrl);
    const goosedSession = await createGoosedSession(goosedUrl, workingDir);
    sessionId = goosedSession.id;
    createdAt = goosedSession.created_at;
    piLog('Session created in goosed:', sessionId);
  } else {
    // Fallback to local ID generation (for testing or when goosed unavailable)
    sessionId = generateSessionId();
    createdAt = new Date().toISOString();
    piLog('Created local session (no goosedUrl provided):', sessionId);
  }
  
  currentSession = {
    id: sessionId,
    name: 'New Chat',
    created_at: createdAt,
    updated_at: createdAt,
    working_dir: workingDir,
    message_count: 0,
    conversation: [],
    input_tokens: 0,
    output_tokens: 0,
    total_tokens: 0,
    accumulated_input_tokens: 0,
    accumulated_output_tokens: 0,
    accumulated_total_tokens: 0,
    goosedUrl,
  };
  
  currentSessionId = sessionId;
  
  // Only save to local JSON if not using goosed
  if (!goosedUrl) {
    saveSession(currentSession);
  }

  piLog('Session created successfully:', sessionId);
  return currentSession;
}

/**
 * Resume an existing session.
 */
export async function resumePiSession(sessionId: string, goosedUrl?: string): Promise<PiSession> {
  piLog('resumePiSession called for:', sessionId, 'goosedUrl:', goosedUrl);
  if (!piModule) {
    throw new Error('Pi module not loaded. Call initializePi() first.');
  }

  // Try to load from goosed first, then fall back to local storage
  let session: PiSession | null = null;
  
  if (goosedUrl) {
    piLog('Loading session from goosed:', goosedUrl);
    session = await loadGoosedSession(goosedUrl, sessionId);
    if (session) {
      piLog('Session loaded from goosed, message count:', session.conversation?.length || 0);
    }
  }
  
  if (!session) {
    piLog('Loading session from local storage');
    session = loadSession(sessionId);
  }
  
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }

  // Stop existing agent session if any
  if (currentAgentSession) {
    await stopCurrentAgentSession();
  }

  // Set working directory
  process.chdir(session.working_dir);

  const sessionConfig: CreateAgentSessionOptions = {
    cwd: session.working_dir,
  };

  piLog('Creating agent session for resume with config:', sessionConfig);
  const result = await piModule.createAgentSession(sessionConfig);
  currentAgentSession = result.session;
  currentSession = session;
  currentSession.goosedUrl = goosedUrl;  // Remember goosed URL for saving
  currentSessionId = sessionId;
  eventAccumulator.reset();

  // THE KEY FIX: Inject conversation history into Pi so it knows the context
  if (session.conversation && session.conversation.length > 0) {
    piLog('Injecting conversation history into Pi, message count:', session.conversation.length);
    
    // Translate Goose messages to Pi format
    const piMessages: PiMessage[] = [];
    for (const gooseMsg of session.conversation) {
      const piMsg = translateGooseMessageToPi(gooseMsg);
      if (piMsg) {
        piMessages.push(piMsg);
      }
    }
    
    piLog('Translated messages for Pi:', piMessages.length);
    
    // Inject into Pi's agent state
    // Pi's agent.replaceMessages() expects AgentMessage[] which is compatible with PiMessage
    if (piMessages.length > 0 && currentAgentSession.agent) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      currentAgentSession.agent.replaceMessages(piMessages as any);
      piLog('Conversation history injected into Pi agent');
    }
  }

  piLog('Session resumed successfully:', sessionId);
  return session;
}

/**
 * Stop the current agent session (but keep session data).
 */
async function stopCurrentAgentSession(): Promise<void> {
  if (unsubscribeFn) {
    unsubscribeFn();
    unsubscribeFn = null;
  }
  if (currentAgentSession) {
    try {
      currentAgentSession.abort();
    } catch {
      // Ignore abort errors
    }
    currentAgentSession = null;
    log.info('[Pi] Agent session stopped');
  }
}

/**
 * Stop and clear the current session.
 */
export async function stopPiSession(): Promise<void> {
  await stopCurrentAgentSession();
  currentSession = null;
  currentSessionId = null;
  eventAccumulator.reset();
  log.info('[Pi] Session cleared');
}

/**
 * Get the current session.
 */
export function getCurrentSession(): PiSession | null {
  return currentSession;
}

/**
 * Send a prompt to Pi and stream events via callback.
 */
export async function promptPi(
  message: string,
  onEvent: (event: PiAgentEvent, gooseMessage?: GooseMessage) => void,
  onNotification: (notification: ToolNotification) => void,
  onComplete: (finalMessages: GooseMessage[]) => void,
  onError: (error: Error) => void
): Promise<void> {
  piLog('promptPi called with message:', message.substring(0, 100));
  if (!currentAgentSession || !currentSession) {
    piLog('ERROR: No Pi session');
    onError(new Error('No Pi session. Call createPiSession() first.'));
    return;
  }

  // Add user message to session for persistence (UI already shows it via usePiChat)
  const userMessage: GooseMessage = {
    id: `msg_${Date.now()}_user`,
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [{ type: 'text', text: message }],
    metadata: { userVisible: true, agentVisible: true },
  };
  currentSession.conversation.push(userMessage);
  currentSession.message_count++;
  currentSession.updated_at = new Date().toISOString();
  
  // Update session name from first user message
  if (currentSession.conversation.filter(m => m.role === 'user').length === 1) {
    currentSession.name = generateSessionName(currentSession.conversation);
  }
  
  // Save session - use goosed if available, otherwise local
  if (currentSession.goosedUrl) {
    saveConversationToGoosed(currentSession.goosedUrl, currentSession.id, currentSession.conversation);
  } else {
    saveSession(currentSession);
  }

  eventAccumulator.reset();

  // Subscribe to events
  if (unsubscribeFn) {
    unsubscribeFn();
  }
  
  piLog('Subscribing to session events...');
  unsubscribeFn = currentAgentSession.subscribe((event: AgentSessionEvent) => {
    const piEvent = event as PiAgentEvent;
    piLog('Received event:', piEvent.type, JSON.stringify(piEvent).substring(0, 500));
    const result = eventAccumulator.processEvent(piEvent);
    if (result.message) {
      piLog('Translated message:', JSON.stringify(result.message).substring(0, 500));
    }
    
    // Send tool notification if present
    if (result.notification) {
      onNotification(result.notification);
    }
    
    // Send translated message to renderer (skip user messages - UI already has them)
    if (result.message && result.message.role !== 'user') {
      onEvent(piEvent, result.message);
      
      // Update conversation with assistant messages for persistence
      if (result.message.role === 'assistant') {
        // Find or add this message in conversation
        const existingIdx = currentSession!.conversation.findIndex(m => m.id === result.message!.id);
        if (existingIdx >= 0) {
          currentSession!.conversation[existingIdx] = result.message;
        } else {
          currentSession!.conversation.push(result.message);
          currentSession!.message_count++;
        }
        currentSession!.updated_at = new Date().toISOString();
      }
    }

    if (result.isComplete) {
      piLog('Event stream complete');
      if (unsubscribeFn) {
        unsubscribeFn();
        unsubscribeFn = null;
      }
      
      // Save final conversation state
      if (currentSession!.goosedUrl) {
        saveConversationToGoosed(currentSession!.goosedUrl, currentSession!.id, currentSession!.conversation);
      } else {
        saveSession(currentSession!);
      }
      onComplete(currentSession!.conversation);
    }
  });

  try {
    piLog('Calling currentAgentSession.prompt()...');
    await currentAgentSession.prompt(message);
    piLog('prompt() returned');
  } catch (error) {
    piLog('prompt() threw error:', error);
    if (unsubscribeFn) {
      unsubscribeFn();
      unsubscribeFn = null;
    }
    // Still save what we have
    if (currentSession!.goosedUrl) {
      saveConversationToGoosed(currentSession!.goosedUrl, currentSession!.id, currentSession!.conversation);
    } else {
      saveSession(currentSession!);
    }
    onError(error instanceof Error ? error : new Error(String(error)));
  }
}

/**
 * Abort current Pi operation.
 */
export function abortPi(): void {
  if (currentAgentSession) {
    currentAgentSession.abort();
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
  currentSessionId: string | null;
} {
  return {
    available: isPiAvailable(),
    version: getPiVersion(),
    hasSession: currentSession !== null,
    isStreaming: currentAgentSession?.state.isStreaming || false,
    currentSessionId,
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

  // Create new session
  ipcMain.handle('pi:createSession', async (_event: IpcMainInvokeEvent, options: CreateSessionOptions) => {
    piLog('IPC pi:createSession called with options:', options);
    try {
      const session = await createPiSession(options);
      return { success: true, session };
    } catch (error) {
      piLog('IPC pi:createSession error:', error);
      return { success: false, error: String(error) };
    }
  });

  // Resume existing session
  ipcMain.handle('pi:resumeSession', async (_event: IpcMainInvokeEvent, sessionId: string, goosedUrl?: string) => {
    piLog('IPC pi:resumeSession called for:', sessionId, 'goosedUrl:', goosedUrl);
    try {
      const session = await resumePiSession(sessionId, goosedUrl);
      return { success: true, session };
    } catch (error) {
      piLog('IPC pi:resumeSession error:', error);
      return { success: false, error: String(error) };
    }
  });

  // Get current session
  ipcMain.handle('pi:getCurrentSession', () => {
    return getCurrentSession();
  });

  // List all sessions - for now, returns local sessions only
  // When using goosed, the UI calls goosed API directly for session list
  ipcMain.handle('pi:listSessions', () => {
    return listAllSessions();
  });

  // Get a specific session
  ipcMain.handle('pi:getSession', async (_event: IpcMainInvokeEvent, sessionId: string, goosedUrl?: string) => {
    if (goosedUrl) {
      return loadGoosedSession(goosedUrl, sessionId);
    }
    return loadSession(sessionId);
  });

  // Delete a session - for now, only deletes local sessions
  // goosed session deletion goes through goosed API
  ipcMain.handle('pi:deleteSession', (_event: IpcMainInvokeEvent, sessionId: string) => {
    return deleteSessionFile(sessionId);
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
        // onNotification - tool execution events
        (notification) => {
          webContents.send('pi:notification', notification);
        },
        // onComplete
        (finalMessages) => {
          webContents.send('pi:complete', { messages: finalMessages });
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
