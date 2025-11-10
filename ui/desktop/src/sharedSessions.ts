import { Message } from './types/message';
import { safeJsonParse } from './utils/jsonUtils';

export interface SharedSessionDetails {
  share_token: string;
  created_at: number;
  base_url: string;
  description: string;
  working_dir: string;
  messages: Message[];
  message_count: number;
  total_tokens: number | null;
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

    const data = await safeJsonParse<SharedSessionDetails>(
      response,
      'Failed to parse shared session'
    );

    if (baseUrl != data.base_url) {
      throw new Error(`Base URL mismatch for shared session: ${baseUrl} != ${data.base_url}`);
    }

    return {
      share_token: data.share_token,
      created_at: data.created_at,
      base_url: data.base_url,
      description: data.description,
      working_dir: data.working_dir,
      messages: data.messages,
      message_count: data.message_count,
      total_tokens: data.total_tokens,
    };
  } catch (error) {
    console.error('Error fetching shared session:', error);
    throw error;
  }
}

/**
 * Creates a new shared session
 * @param baseUrl The base URL for session sharing API
 * @param workingDir The working directory for the shared session
 * @param messages The messages to include in the shared session
 * @param description Description for the shared session
 * @param totalTokens Total token count for the session, or null if not available
 * @param userName The user name for who is sharing the session
 * @returns Promise with the share token
 */
export async function createSharedSession(
  baseUrl: string,
  workingDir: string,
  messages: Message[],
  description: string,
  totalTokens: number | null
): Promise<string> {
  try {
    const response = await fetch(`${baseUrl}/sessions/share`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        working_dir: workingDir,
        messages,
        description: description,
        base_url: baseUrl,
        total_tokens: totalTokens ?? null,
      }),
    });

    if (!response.ok) {
      if (response.status === 302) {
        throw new Error(
          `Failed to create shared session. Please check that you are connected to VPN - ${response.status} ${response.statusText}`
        );
      }
      throw new Error(`Failed to create shared session: ${response.status} ${response.statusText}`);
    }

    const data = await safeJsonParse<{ share_token: string }>(
      response,
      'Failed to parse shared session response'
    );
    return data.share_token;
  } catch (error) {
    console.error('Error creating shared session:', error);
    throw error;
  }
}
