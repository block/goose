import fs from 'fs';
import path from 'path';

const SESSIONS_PATH = path.join(process.env.HOME || process.env.USERPROFILE || '.', '.config', 'goose', 'sessions');
if (!fs.existsSync(SESSIONS_PATH)) {
  fs.mkdirSync(SESSIONS_PATH);
}

let cache: { [key: string]: Session } = {};

interface Session {
  name: string; // Derived from a synopsis of the conversation
  messages: Array<{
    id: number;
    role: 'function' | 'system' | 'user' | 'assistant' | 'data' | 'tool';
    content: string;
  }>;
  directory: string;
}

function generateSessionName(messages: {id: number, role: string, content: string}[]): string {
  // Create a session name based on the first message or a combination of initial messages
  if (messages === undefined || messages.length === 0) return 'empty_session';
  return messages[0].content.split(' ').slice(0, 5).join(' ');
}

function createSafeFilename(name: string): string {
  // Replace unsafe characters with underscores and limit length
  return name
    .replace(/[^a-zA-Z0-9-_]/g, '_') // Replace unsafe chars with underscore
    .replace(/_{2,}/g, '_')      // Replace multiple underscores with single
    .replace(/^_|_$/g, '')       // Remove leading/trailing underscores
    .substring(0, 100);        // Limit length to 100 chars
}

export function saveSession(session: Session): string {
  try {
    const sessionData = {
      ...session,
      name: generateSessionName(session.messages)
    };
    const safeFileName = createSafeFilename(sessionData.name);
    const filePath = path.join(SESSIONS_PATH, `${safeFileName}.json`);
    fs.writeFileSync(filePath, JSON.stringify(sessionData, null, 2));
    console.log('Session saved:', sessionData);
    return sessionData.name;
  } catch (error) {
    console.error('Error saving session:', error);
  }
}

export function loadSession(sessionId: string): Session | undefined {
  try {
    if (cache[sessionId]) {
      console.log('Session loaded from cache:', sessionId);
      return cache[sessionId];
    }
    const safeFileName = createSafeFilename(sessionId);
    const filePath = path.join(SESSIONS_PATH, `${safeFileName}.json`);
    if (!fs.existsSync(filePath)) {
      console.warn('Session file not found:', sessionId);
      return undefined;
    }
    const data = fs.readFileSync(filePath, 'utf8');
    const session = JSON.parse(data) as Session;
    console.log('Session loaded:', session);
    return session;
  } catch (error) {
    console.error('Error loading session:', error);
  }
}

// load sessions that are relevant to the directory supplied (not where they are stored, but where user is operating)
export function loadSessions(): Session[] {
  try {
    if (cache && Object.keys(cache).length > 0) {
      console.log('Sessions loaded from cache');
      return Object.values(cache).splice(0, 20);
    }
    console.log('Attempting to load sessions from:', SESSIONS_PATH);
    const MAX_AGE_DAYS = 10;
    // Get the current date
    const now = Date.now();
    const maxAgeMs = MAX_AGE_DAYS * 24 * 60 * 60 * 1000;

    // Get all files in the directory
    const files = fs.readdirSync(SESSIONS_PATH);

    if (files.length === 0) {
      console.warn('No session files found in directory');
      return [];
    }

    // Filter files based on their age and limit to max 100 files
    const filteredFiles = files
      .map(file => {
        const filePath = path.join(SESSIONS_PATH, file);
        const stats = fs.statSync(filePath);
        const age = now - stats.mtimeMs;
        return { file, age };
      })
      .filter(({ file, age }) => file.endsWith('.json') && age <= maxAgeMs);

    if (filteredFiles.length === 0) {
      console.warn('No session files meet the age criteria');
      return [];
    }

    // Load the filtered files and parse them into sessions
    const sessions = filteredFiles.map(({ file }) => {
      const data = fs.readFileSync(path.join(SESSIONS_PATH, file), 'utf8');
      return JSON.parse(data) as Session;
    });
    //save sessions in cache
    sessions.forEach(session => {
      cache[session.name] = session;
    });
    return sessions.splice(0, 20);
  } catch (error) {
    console.error('Error loading sessions:', error);
    return [];
  }
}

export function clearAllSessions(): void {
  try {
    const files = fs.readdirSync(SESSIONS_PATH);
    files.forEach(file => fs.unlinkSync(path.join(SESSIONS_PATH, file)));
    console.log('All sessions cleared');
  } catch (error) {
    console.error('Error clearing sessions:', error);
  }
}
