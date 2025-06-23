import { Message } from './types/message';
import { getSessionHistory, listSessions, SessionInfo } from './api';
import { convertApiMessageToFrontendMessage } from './components/context_management';
import { client } from './api/client.gen';

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
            sessionInfo.metadata && sessionInfo.metadata.description !== ''
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
 * Creates a branch from an existing session up to a specified message index
 *
 * @param sessionId - ID of the source session to branch from
 * @param messageIndex - Index of the last message to include in the branch
 * @param description - Optional description for the new branch
 * @returns Promise with the ID of the newly created branch session
 */
export async function createSessionBranch(
  sessionId: string,
  messageIndex: number,
  description?: string
): Promise<string> {
  try {
    const requestBody = {
      messageIndex: messageIndex,
      description,
    };

    const response = await client.post<{ branchSessionId: string }, unknown, false>({
      url: `/sessions/${sessionId}/branch`,
      headers: {
        'Content-Type': 'application/json',
      },
      body: requestBody,
      throwOnError: false,
    });

    if (response.error) {
      throw new Error(`Server error: ${JSON.stringify(response.error)}`);
    }

    if (!response.data || !response.data.branchSessionId) {
      throw new Error(`Invalid response format: ${JSON.stringify(response.data)}`);
    }

    return response.data.branchSessionId;
  } catch (error) {
    console.error(`Error creating session branch for ${sessionId}:`, error);
    throw error;
  }
}
