import { Message, createUserMessage, createAssistantMessage } from './types/message';

export interface SharedSessionDetails {
  share_token: string;
  created_at: number;
  base_url: string;
  description: string;
  messages: Message[];
}

/**
 * Fetches details for a specific shared session
 * @param baseUrl The base URL for session sharing API
 * @param shareToken The share token of the session to fetch
 * @returns Promise with shared session details
 */
export async function fetchSharedSessionDetails(
  baseUrl: string,
  shareToken: string
): Promise<SharedSessionDetails> {
  try {
    const response = await fetch(`${baseUrl}/sessions/share/${shareToken}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        // Origin: 'http://localhost:5173', // required to bypass Cloudflare security filter
      },
      credentials: 'include',
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch shared session: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();

    if (baseUrl != data.base_url) {
      throw new Error(`Base URL mismatch for shared session: ${baseUrl} != ${data.base_url}`);
    }

    return {
      share_token: data.share_token,
      created_at: data.created_at,
      base_url: data.base_url,
      description: data.description || 'Shared Session',
      messages: data.messages,
    };
  } catch (error) {
    console.error('Error fetching shared session:', error);
    throw error;
  }
}

/**
 * Creates a new shared session
 * @param baseUrl The base URL for session sharing API
 * @param messages The messages to include in the shared session
 * @param description Optional description for the shared session
 * @returns Promise with the share token
 */
export async function createSharedSession(
  baseUrl: string,
  messages: Message[],
  description?: string
): Promise<string> {
  try {
    const response = await fetch(`${baseUrl}/sessions/share`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        messages,
        description: description || 'Shared Session',
        base_url: baseUrl,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to create shared session: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data.share_token;
  } catch (error) {
    console.error('Error creating shared session:', error);
    throw error;
  }
}
