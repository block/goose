/** Minimum age before an empty untitled session is treated as a leftover phantom. */
export const PHANTOM_SESSION_MIN_AGE_MS = 60_000;

export interface PhantomPurgeCandidate {
  id: string;
  messageCount: number;
  userSetName?: boolean;
  hasRecipe?: boolean;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Startup cleanup for empty untitled sessions left by prior runs.
 * Never returns sessions that are actively protected or younger than minAgeMs —
 * Hub creates messageCount===0 sessions that must survive the in-flight list race.
 */
export function selectPhantomSessionsForPurge(
  sessions: PhantomPurgeCandidate[],
  protectedIds: ReadonlySet<string>,
  nowMs: number = Date.now(),
  minAgeMs: number = PHANTOM_SESSION_MIN_AGE_MS
): PhantomPurgeCandidate[] {
  return sessions.filter((session) => {
    if (session.messageCount !== 0 || session.userSetName || session.hasRecipe) {
      return false;
    }
    if (protectedIds.has(session.id)) {
      return false;
    }

    const createdMs = Date.parse(session.createdAt || session.updatedAt || '');
    // Missing timestamps: skip purge rather than risk deleting a just-created session.
    if (Number.isNaN(createdMs) || nowMs - createdMs < minAgeMs) {
      return false;
    }

    return true;
  });
}
