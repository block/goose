import { getApiUrl, getSecretKey } from './config';
import { Message } from './types/message';

export interface SessionMetadata {
  description: string;
  message_count: number;
  total_tokens: number | null;
  working_dir: string; // Required in type, but may be missing in old sessions
}

// Helper function to ensure working directory is set
export function ensureWorkingDir(metadata: Partial<SessionMetadata>): SessionMetadata {
  return {
    description: metadata.description || '',
    message_count: metadata.message_count || 0,
    total_tokens: metadata.total_tokens || null,
    working_dir: metadata.working_dir || process.env.HOME || '',
  };
}

export interface Session {
  id: string;
  path: string;
  modified: string;
  metadata: SessionMetadata;
}

export interface SessionsResponse {
  sessions: Session[];
}

export interface SessionDetails {
  session_id: string;
  metadata: SessionMetadata;
  messages: Message[];
}

/**
 * Generate a session ID in the format yyyymmdd_hhmmss
 */
export function generateSessionId(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  const hours = String(now.getHours()).padStart(2, '0');
  const minutes = String(now.getMinutes()).padStart(2, '0');
  const seconds = String(now.getSeconds()).padStart(2, '0');

  return `${year}${month}${day}_${hours}${minutes}${seconds}`;
}

/**
 * Fetches all available sessions from the API
 * @returns Promise with sessions data
 */
export async function fetchSessions(): Promise<SessionsResponse> {
  try {
    const response = await fetch(getApiUrl('/sessions'), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch sessions: ${response.status} ${response.statusText}`);
    }

    // TODO: remove this logic once everyone migrates to the new sessions format
    // for now, filter out sessions whose description is empty (old CLI sessions)
    const rawSessions = await response.json();
    const sessions = rawSessions.sessions
      .filter((session: Session) => session.metadata.description !== '')
      .map((session: Session) => ({
        ...session,
        metadata: ensureWorkingDir(session.metadata),
      }));

    // order sessions by 'modified' date descending
    sessions.sort(
      (a: Session, b: Session) => new Date(b.modified).getTime() - new Date(a.modified).getTime()
    );

    return { sessions };
  } catch (error) {
    console.error('Error fetching sessions:', error);
    throw error;
  }
}

/**
 * Fetches details for a specific session
 * @param sessionId The ID of the session to fetch
 * @returns Promise with session details
 */
export async function fetchSessionDetails(sessionId: string): Promise<SessionDetails> {
  try {
    const response = await fetch(getApiUrl(`/sessions/${sessionId}`), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch session details: ${response.status} ${response.statusText}`);
    }

    const details = await response.json();
    return {
      ...details,
      metadata: ensureWorkingDir(details.metadata),
    };
  } catch (error) {
    console.error(`Error fetching session details for ${sessionId}:`, error);
    throw error;
  }
}
