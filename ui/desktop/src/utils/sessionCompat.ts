import type { Session } from '../api/types.gen';

/**
 * Get the display name for a session, handling both old and new formats
 * @param session - Session object that may have either 'name' or 'description' field
 * @returns The session's display name
 */
export function getSessionName(session: Session | null | undefined): string {
  if (!session) return '';
  // Check for 'name' first (new format), then fall back to 'description' (old format), then id
  // @ts-expect-error - description might not exist in newer types but can exist in old session data
  return session.name || session.description || session.id;
}

/**
 * Check if a session has a user-provided name
 * @param session - Session object
 * @returns true if the session has a user-set name
 */
export function hasUserSetName(session: Session | null | undefined): boolean {
  if (!session) return false;
  return session.user_set_name === true;
}
