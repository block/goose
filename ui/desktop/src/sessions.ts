import { Message } from './types/message';
import { getSessionHistory, listSessions, SessionInfo } from './api';
import { convertApiMessageToFrontendMessage } from './components/context_management';
import { getApiUrl } from './config';

export interface SessionMetadata {
  description: string;
  message_count: number;
  total_tokens: number | null;
  working_dir: string; // Required in type, but may be missing in old sessions
  // Add the accumulated token fields from the API
  accumulated_input_tokens?: number | null;
  accumulated_output_tokens?: number | null;
  accumulated_total_tokens?: number | null;
}

// Helper function to ensure working directory is set
export function ensureWorkingDir(metadata: Partial<SessionMetadata>): SessionMetadata {
  return {
    description: metadata.description || '',
    message_count: metadata.message_count || 0,
    total_tokens: metadata.total_tokens || null,
    working_dir: metadata.working_dir || process.env.HOME || '',
    accumulated_input_tokens: metadata.accumulated_input_tokens || null,
    accumulated_output_tokens: metadata.accumulated_output_tokens || null,
    accumulated_total_tokens: metadata.accumulated_total_tokens || null,
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
/**
 * Fetches all available sessions from the API
 * @returns Promise with an array of Session objects
 */
export async function fetchSessions(): Promise<Session[]> {
  try {
    const response = await listSessions<true>();

    // Check if the response has the expected structure
    if (response && response.data && response.data.sessions) {
      // Since the API returns SessionInfo, we need to convert to Session
      const sessions = response.data.sessions
        .filter(
          (sessionInfo: SessionInfo) =>
            sessionInfo.metadata && sessionInfo.metadata.message_count > 0
        )
        .map(
          (sessionInfo: SessionInfo): Session => ({
            id: sessionInfo.id,
            path: sessionInfo.path,
            modified: sessionInfo.modified,
            metadata: ensureWorkingDir(sessionInfo.metadata),
          })
        );

      // order sessions by 'modified' date descending
      sessions.sort(
        (a: Session, b: Session) => new Date(b.modified).getTime() - new Date(a.modified).getTime()
      );

      return sessions;
    } else {
      throw new Error('Unexpected response format from listSessions');
    }
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
    const response = await getSessionHistory<true>({
      path: { session_id: sessionId },
    });

    // Convert the SessionHistoryResponse to a SessionDetails object
    return {
      session_id: response.data.sessionId,
      metadata: ensureWorkingDir(response.data.metadata),
      messages: response.data.messages.map((message) =>
        convertApiMessageToFrontendMessage(message, true, true)
      ), // slight diffs between backend and frontend Message obj
    };
  } catch (error) {
    console.error(`Error fetching session details for ${sessionId}:`, error);
    throw error;
  }
}

/**
 * Updates the metadata for a specific session
 * @param sessionId The ID of the session to update
 * @param description The new description (name) for the session
 * @returns Promise that resolves when the update is complete
 */
export async function updateSessionMetadata(sessionId: string, description: string): Promise<void> {
  try {
    const url = getApiUrl(`/sessions/${sessionId}/metadata`);
    const secretKey = await window.electron.getSecretKey();

    const response = await fetch(url, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': secretKey,
      },
      body: JSON.stringify({ description }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to update session metadata: ${response.statusText} - ${errorText}`);
    }
  } catch (error) {
    console.error(`Error updating session metadata for ${sessionId}:`, error);
    throw error;
  }
}
