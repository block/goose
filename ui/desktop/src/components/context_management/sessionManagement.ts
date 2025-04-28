import { Message } from '../../types/message';
import { getApiUrl } from '../../config';

/**
 * Generates a new session ID
 * (Copied from useChat implementation)
 */
export function generateSessionId() {
    return `session_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
}

/**
 * Creates a new session while preserving the current one
 * This is specifically for handling context length exceeded scenarios
 */
export async function createContinuationSession({
                                                    originalSessionId,
                                                    messages,
                                                    workingDir,
                                                    title = "Continued Session",
                                                }: {
    originalSessionId: string;
    messages: Message[];
    workingDir: string;
    title?: string;
}): Promise<{
    success: boolean;
    newSessionId?: string;
    error?: string;
}> {
    try {
        // Generate a new session ID that indicates it's a continuation
        const newSessionId = `${originalSessionId}_continued_${Date.now()}`;

        // Make API call to create the new session
        const response = await fetch(getApiUrl('/sessions/create'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: newSessionId,
                messages: messages,
                metadata: {
                    description: title,
                    working_dir: workingDir,
                    //parent_session_id: originalSessionId // todo: decide if we want relationships in the metadata
                }
            })
        });

        if (!response.ok) {
            throw new Error(`Failed to create continuation session: ${response.statusText}`);
        }

        const data = await response.json();
        return { success: true, newSessionId: data.session_id || newSessionId };
    } catch (error) {
        console.error('Failed to create continuation session:', error);
        // Fix the TypeScript error by handling the unknown error type
        const errorMessage = error instanceof Error ? error.message : String(error);
        return { success: false, error: errorMessage };
    }
}