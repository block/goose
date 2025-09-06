import { branchSession, getSessionHistory } from '../api/sdk.gen';
import { BranchSessionRequest } from '../api/types.gen';

/**
 * Create a branch from a specific message in a session
 * @param sessionId - The session ID to branch from
 * @param messageIndex - The index of the message to branch from (inclusive)
 * @param description - Optional description for the new branch
 * @returns Promise with the new branch session ID
 */
export async function createSessionBranch(
  sessionId: string,
  messageIndex: number,
  description?: string
): Promise<string> {
  const request: BranchSessionRequest = {
    messageIndex,
    description: description || undefined,
  };

  const response = await branchSession({
    path: { session_id: sessionId },
    body: request,
  });

  if (response.error) {
    throw new Error(`Failed to create branch: ${response.error}`);
  }

  if (!response.data) {
    throw new Error('No response data received from branch API');
  }

  return response.data.branchSessionId;
}

/**
 * Fetch session details including messages with branching metadata
 * @param sessionId - The session ID to fetch
 * @returns Promise with the session details
 */
export async function fetchSessionDetails(sessionId: string) {
  const response = await getSessionHistory({
    path: { session_id: sessionId },
  });

  if (response.error) {
    throw new Error(`Failed to fetch session details: ${response.error}`);
  }

  return response.data;
}

/**
 * Open a session in a new window/tab
 * @param sessionId - The session ID to open
 */
export function openSessionInNewWindow(sessionId: string) {
  // Use Electron's createChatWindow API to create a new window with the session
  const workingDir = window.appConfig?.get('GOOSE_WORKING_DIR') as string;
  window.electron.createChatWindow(
    undefined, // query
    workingDir, // dir
    undefined, // version
    sessionId, // resumeSessionId - this will load the session in the new window
    undefined, // recipe
    undefined // viewType
  );
}
