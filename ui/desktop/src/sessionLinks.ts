import { fetchSharedSessionDetails, SharedSessionDetails } from './sharedSessions';
import { type View } from './App';

export interface SessionLinksViewOptions {
  sessionDetails?: SharedSessionDetails | null;
  error?: string;
  shareToken?: string;
  baseUrl?: string;
  [key: string]: unknown;
}

/**
 * Handles opening a shared session from a deep link
 * @param url The deep link URL (goose://sessions/:shareToken)
 * @param setView Function to set the current view
 * @param baseUrl Optional base URL for the session sharing API
 * @returns Promise that resolves when the session is opened
 */
export async function openSharedSessionFromDeepLink(
  url: string,
  setView: (view: View, options?: SessionLinksViewOptions) => void,
  baseUrl?: string
): Promise<SharedSessionDetails | null> {
  try {
    if (!url.startsWith('goose://sessions/')) {
      throw new Error('Invalid URL: URL must use the goose://sessions/ scheme');
    }

    // Extract the share token from the URL
    const shareToken: string = url.replace('goose://sessions/', '');

    if (!shareToken || shareToken.trim() === '') {
      throw new Error('Invalid URL: Missing share token');
    }

    // If no baseUrl is provided, check if there's one in localStorage
    if (!baseUrl) {
      const savedSessionConfig = localStorage.getItem('session_sharing_config');
      if (savedSessionConfig) {
        try {
          const config = JSON.parse(savedSessionConfig);
          if (config.enabled && config.baseUrl) {
            baseUrl = config.baseUrl;
          } else {
            throw new Error(
              'Session sharing is not enabled or base URL is not configured. Check the settings page.'
            );
          }
        } catch (error) {
          console.error('Error parsing session sharing config:', error);
          throw new Error(
            'Session sharing is not enabled or base URL is not configured. Check the settings page.'
          );
        }
      } else {
        throw new Error('Session sharing is not configured');
      }
    }

    // Fetch the shared session details
    const sessionDetails = await fetchSharedSessionDetails(baseUrl!, shareToken);

    // Navigate to the shared session view
    setView('sharedSession', {
      sessionDetails,
      shareToken,
      baseUrl,
    });

    return sessionDetails;
  } catch (error) {
    const errorMessage = `Failed to open shared session: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);

    // Navigate to the shared session view with the error instead of throwing
    setView('sharedSession', {
      sessionDetails: null,
      error: error instanceof Error ? error.message : 'Unknown error',
      shareToken: url.replace('goose://sessions/', ''),
      baseUrl,
    });

    return null;
  }
}
