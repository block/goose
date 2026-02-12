import { useEffect, useRef } from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom';
import BaseChat from './BaseChat';
import { ChatType } from '../types/chat';
import { UserInput } from '../types/message';
import { AppEvents } from '../constants/events';
import { clearDeletedSessionFromCreatedDetail, markSessionDeleted } from '../utils/activeSessions';

interface ChatSessionsContainerProps {
  setChat: (chat: ChatType) => void;
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
}

/**
 * Container that mounts ALL active chat sessions to keep them alive.
 * Uses CSS to show/hide sessions based on the current URL parameter.
 * This allows multiple sessions to stream simultaneously in the background.
 */
export default function ChatSessionsContainer({
  setChat,
  activeSessions,
}: ChatSessionsContainerProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const currentSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const isOnPairRoute = location.pathname === '/pair';

  // Track deleted session IDs to prevent resurrection from stale URL params
  const deletedSessionIds = useRef<Set<string>>(new Set());

  useEffect(() => {
    const handleSessionDeleted = (event: Event) => {
      const { sessionId } = (event as CustomEvent<{ sessionId: string }>).detail;
      markSessionDeleted(deletedSessionIds.current, sessionId);
    };
    const handleSessionCreated = (event: Event) => {
      const detail = (event as CustomEvent<{ session?: { id?: string }; sessionId?: string }>)
        .detail;
      clearDeletedSessionFromCreatedDetail(deletedSessionIds.current, detail);
    };
    window.addEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
    window.addEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
    return () => {
      window.removeEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
      window.removeEventListener(AppEvents.SESSION_CREATED, handleSessionCreated);
    };
  }, []);

  useEffect(() => {
    if (!isOnPairRoute) {
      return;
    }

    const navigateToSession = (sessionId: string) => {
      const params = new URLSearchParams();
      params.set('resumeSessionId', sessionId);
      navigate(`/pair?${params.toString()}`, { replace: true });
    };

    const nonDeletedActiveSessions = activeSessions.filter(
      (session) => !deletedSessionIds.current.has(session.sessionId)
    );

    // If /pair has no explicit active session, recover to the most-recent active session.
    if (!currentSessionId) {
      const fallback = nonDeletedActiveSessions[nonDeletedActiveSessions.length - 1];
      if (fallback?.sessionId) {
        navigateToSession(fallback.sessionId);
      } else {
        navigate('/', { replace: true });
      }
      return;
    }

    // If URL points at a deleted session, recover instead of showing a blank panel.
    if (
      deletedSessionIds.current.has(currentSessionId) &&
      !nonDeletedActiveSessions.some((session) => session.sessionId === currentSessionId)
    ) {
      const fallback = [...nonDeletedActiveSessions].reverse()[0];

      if (fallback?.sessionId) {
        navigateToSession(fallback.sessionId);
      } else {
        navigate('/', { replace: true });
      }
    }
  }, [isOnPairRoute, currentSessionId, activeSessions, navigate]);

  // Always render active sessions to keep SSE connections alive, even when not on /pair route
  if (!currentSessionId && activeSessions.length === 0) {
    return null;
  }

  // Build the list of sessions to render
  let sessionsToRender = activeSessions.filter(
    (session) => !deletedSessionIds.current.has(session.sessionId)
  );

  // If we have a currentSessionId that's not in activeSessions, add it (handles page refresh)
  // But never restore a session that was explicitly deleted
  if (
    currentSessionId &&
    !sessionsToRender.some((s) => s.sessionId === currentSessionId) &&
    !deletedSessionIds.current.has(currentSessionId)
  ) {
    sessionsToRender = [...sessionsToRender, { sessionId: currentSessionId }];
  }

  const visibleSessionId = currentSessionId ?? activeSessions[activeSessions.length - 1]?.sessionId;

  return (
    <div className="relative w-full h-full">
      {sessionsToRender.map((session) => {
        const isVisible = session.sessionId === visibleSessionId;

        return (
          <div
            key={session.sessionId}
            className={`absolute inset-0 ${isVisible ? 'block' : 'hidden'}`}
            data-session-id={session.sessionId}
          >
            <BaseChat
              setChat={setChat}
              sessionId={session.sessionId}
              initialMessage={session.initialMessage}
              suppressEmptyState={false}
              isActiveSession={isVisible}
            />
          </div>
        );
      })}
    </div>
  );
}
