import { UserInput } from '../types/message';

export interface ActiveSessionEntry {
  sessionId: string;
  initialMessage?: UserInput;
}

export interface AddActiveSessionPayload {
  sessionId: string;
  initialMessage?: UserInput;
}

export interface SessionCreatedDetail {
  session?: {
    id?: string;
  };
  sessionId?: string;
}

export function markSessionDeleted(deletedSessionIds: Set<string>, sessionId: string): void {
  deletedSessionIds.add(sessionId);
}

export function clearDeletedSessionFromCreatedDetail(
  deletedSessionIds: Set<string>,
  detail?: SessionCreatedDetail
): string | undefined {
  const sessionId = getCreatedSessionId(detail);
  if (sessionId) {
    deletedSessionIds.delete(sessionId);
  }
  return sessionId;
}

export function addActiveSession(
  prev: ActiveSessionEntry[],
  payload: AddActiveSessionPayload,
  deletedSessionIds: Set<string>,
  maxActiveSessions: number
): ActiveSessionEntry[] {
  const { sessionId, initialMessage } = payload;

  if (deletedSessionIds.has(sessionId)) {
    return prev;
  }

  const existingIndex = prev.findIndex((s) => s.sessionId === sessionId);

  if (existingIndex !== -1) {
    const existing = prev[existingIndex];
    const updatedExisting =
      !existing.initialMessage && initialMessage ? { ...existing, initialMessage } : existing;
    return [...prev.slice(0, existingIndex), ...prev.slice(existingIndex + 1), updatedExisting];
  }

  const newSession = { sessionId, initialMessage };
  const updated = [...prev, newSession];
  if (updated.length > maxActiveSessions) {
    return updated.slice(updated.length - maxActiveSessions);
  }
  return updated;
}

export function clearInitialMessage(
  prev: ActiveSessionEntry[],
  sessionId: string
): ActiveSessionEntry[] {
  return prev.map((session) => {
    if (session.sessionId === sessionId) {
      return { ...session, initialMessage: undefined };
    }
    return session;
  });
}

export function removeActiveSession(
  prev: ActiveSessionEntry[],
  sessionId: string
): ActiveSessionEntry[] {
  return prev.filter((session) => session.sessionId !== sessionId);
}

export function getCreatedSessionId(detail?: SessionCreatedDetail): string | undefined {
  return detail?.session?.id ?? detail?.sessionId;
}
