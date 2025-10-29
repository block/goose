import { Session } from '../api';
import { getApiUrl } from '../config';

/**
 * In-memory cache for session data
 * Maps session ID to Session object
 */
const sessionCache = new Map<string, Session>();

/**
 * In-flight request tracking to prevent duplicate fetches
 * Maps session ID to Promise of Session
 */
const inFlightRequests = new Map<string, Promise<Session>>();

/**
 * Load a session from the server using the /agent/resume endpoint
 * Implements caching to avoid redundant fetches
 *
 * @param sessionId - The unique identifier for the session
 * @param forceRefresh - If true, bypass cache and fetch fresh data
 * @returns Promise resolving to the Session object
 * @throws Error if the request fails or session not found
 */
export async function loadSession(sessionId: string, forceRefresh = false): Promise<Session> {
  if (!forceRefresh && sessionCache.has(sessionId)) {
    return sessionCache.get(sessionId)!;
  }

  if (inFlightRequests.has(sessionId)) {
    return inFlightRequests.get(sessionId)!;
  }

  const fetchPromise = (async () => {
    try {
      const url = getApiUrl('/agent/resume');
      const secretKey = await window.electron.getSecretKey();

      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify({
          session_id: sessionId,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text().catch(() => 'Unknown error');
        throw new Error(`Failed to load session: HTTP ${response.status} - ${errorText}`);
      }

      const session: Session = await response.json();
      sessionCache.set(sessionId, session);

      return session;
    } catch (error) {
      if (error instanceof Error) {
        throw new Error(`Error loading session ${sessionId}: ${error.message}`);
      }
      throw new Error(`Error loading session ${sessionId}: Unknown error`);
    } finally {
      inFlightRequests.delete(sessionId);
    }
  })();

  inFlightRequests.set(sessionId, fetchPromise);
  return fetchPromise;
}

/**
 * Clear a specific session from the cache
 * Useful when a session has been updated and needs to be refetched
 *
 * @param sessionId - The unique identifier for the session to clear
 */
export function clearSessionCache(sessionId: string): void {
  sessionCache.delete(sessionId);
}

/**
 * Clear all sessions from the cache
 * Useful for logout or when switching contexts
 */
export function clearAllSessionCache(): void {
  sessionCache.clear();
}

/**
 * Check if a session is currently cached
 *
 * @param sessionId - The unique identifier for the session
 * @returns true if the session is in cache, false otherwise
 */
export function isSessionCached(sessionId: string): boolean {
  return sessionCache.has(sessionId);
}

/**
 * Get a session from cache without fetching
 * Returns undefined if not cached
 *
 * @param sessionId - The unique identifier for the session
 * @returns The cached Session object or undefined
 */
export function getCachedSession(sessionId: string): Session | undefined {
  return sessionCache.get(sessionId);
}

/**
 * Preload a session into cache
 * Useful when you already have session data from another source
 *
 * @param session - The Session object to cache
 */
export function preloadSession(session: Session): void {
  sessionCache.set(session.id, session);
}

/**
 * Get the current cache size
 * Useful for debugging and monitoring
 *
 * @returns The number of sessions currently cached
 */
export function getCacheSize(): number {
  return sessionCache.size;
}
