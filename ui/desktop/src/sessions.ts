import { Message } from './types/message';
import { getSessionHistory, listSessions, SessionInfo } from './api';
import { convertApiMessageToFrontendMessage } from './components/context_management';
import { wildcardMatch } from './utils/wildcardMatch';

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
  contentSearchMatch?: boolean; // Flag to indicate if the session content matches a search term
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
 * Extracts text content from a message
 * @param message The message to extract text from
 * @returns The text content of the message
 */
function extractMessageText(message: Message): string {
  let text = '';
  
  for (const content of message.content) {
    if (content.type === 'text') {
      text += content.text + ' ';
    } else if (content.type === 'toolRequest' || content.type === 'toolResponse') {
      // Try to extract text from tool calls and responses
      try {
        text += JSON.stringify(content) + ' ';
      } catch (e) {
        // Ignore errors in JSON stringification
      }
    }
  }
  
  return text;
}

/**
 * Searches for a term within a session's content
 * @param sessionId The ID of the session to search
 * @param searchTerm The term to search for
 * @param caseSensitive Whether the search should be case sensitive
 * @returns Promise<boolean> True if the search term was found in the session content
 */
export async function searchSessionContent(
  sessionId: string, 
  searchTerm: string, 
  caseSensitive: boolean = false
): Promise<boolean> {
  try {
    const sessionDetails = await fetchSessionDetails(sessionId);
    const hasWildcard = searchTerm.includes('*');
    
    for (const message of sessionDetails.messages) {
      const messageText = extractMessageText(message);
      
      if (hasWildcard) {
        if (wildcardMatch(messageText, searchTerm, caseSensitive)) {
          return true;
        }
      } else {
        if (caseSensitive) {
          if (messageText.includes(searchTerm)) {
            return true;
          }
        } else {
          if (messageText.toLowerCase().includes(searchTerm.toLowerCase())) {
            return true;
          }
        }
      }
    }
    
    return false;
  } catch (error) {
    console.error(`Error searching session content for ${sessionId}:`, error);
    return false;
  }
}

/**
 * Searches for a term within multiple sessions' content
 * @param sessions Array of sessions to search
 * @param searchTerm The term to search for
 * @param caseSensitive Whether the search should be case sensitive
 * @param onProgress Callback function that receives progress updates
 * @returns Promise<Session[]> Sessions with contentSearchMatch flag set
 */
export async function searchSessionsContent(
  sessions: Session[],
  searchTerm: string,
  caseSensitive: boolean = false,
  onProgress?: (current: number, total: number) => void
): Promise<Session[]> {
  const result = [...sessions];
  let processedCount = 0;
  
  // If no search term, return all sessions without content match flag
  if (!searchTerm) {
    return result.map(session => ({ ...session, contentSearchMatch: false }));
  }
  
  // Search each session in parallel with a concurrency limit
  const concurrencyLimit = 3;
  const chunks = [];
  
  // Split sessions into chunks for concurrent processing
  for (let i = 0; i < result.length; i += concurrencyLimit) {
    chunks.push(result.slice(i, i + concurrencyLimit));
  }
  
  // Process chunks sequentially, but sessions within a chunk concurrently
  for (const chunk of chunks) {
    await Promise.all(chunk.map(async (session) => {
      const contentMatch = await searchSessionContent(session.id, searchTerm, caseSensitive);
      session.contentSearchMatch = contentMatch;
      
      processedCount++;
      if (onProgress) {
        onProgress(processedCount, result.length);
      }
    }));
  }
  
  return result;
}
