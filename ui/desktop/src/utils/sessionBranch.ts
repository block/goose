import { Message } from '../types/message';
import { generateSessionId } from '../sessions';

/**
 * Creates a new session branch from an existing session up to a specific message index
 * 
 * @param sessionId - ID of the source session to branch from
 * @param messages - All messages from the source session
 * @param messageIndex - Index of the last message to include in the branch
 * @returns The ID of the newly created session
 */
export async function createSessionBranch(
  sessionId: string,
  messages: Message[],
  messageIndex: number
): Promise<string> {
  try {
    // Generate a new session ID
    const newSessionId = generateSessionId();
    
    // Create the truncated messages array (include messages up to and including messageIndex)
    const truncatedMessages = messages.slice(0, messageIndex + 1);
    
    // Get the working directory from the current session metadata
    const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;
    
    // Open a new chat window with the branch session ID
    // This will create a new session file with the specified ID
    window.electron.createChatWindow(
      undefined, // query
      workingDir, // dir
      undefined, // version
      newSessionId, // resumeSessionId
      undefined, // recipeConfig
      undefined, // viewType
      { 
        branchData: {
          metadata: {
            description: `Branched from ${sessionId}`,
            message_count: truncatedMessages.length,
            total_tokens: null,
            working_dir: workingDir,
          },
          messages: truncatedMessages
        }
      } // additional data
    );
    
    return newSessionId;
  } catch (error) {
    console.error('Error creating session branch:', error);
    throw error;
  }
}