import { useLocation, useParams } from 'react-router-dom';
import { useNavigation } from '@/hooks/useNavigation';
import { startNewSession } from '@/sessions';
import type { ChatType } from '@/types/chat';
import type { UserInput } from '@/types/message';
import { getInitialWorkingDir } from '@/utils/workingDir';
import BaseChat from './BaseChat';
import WelcomeState from './WelcomeState';

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
 * When no sessions exist, shows the WelcomeState landing page.
 * ChatInput is rendered globally in AppLayout.
 */
export default function ChatSessionsContainer({
  setChat,
  activeSessions,
}: ChatSessionsContainerProps) {
  const location = useLocation();
  const { sessionId: sessionIdParam } = useParams();
  const setView = useNavigation();
  const currentSessionId = sessionIdParam ? decodeURIComponent(sessionIdParam) : undefined;

  // No active sessions — show WelcomeState (ChatInput is in AppLayout)
  if (!currentSessionId && activeSessions.length === 0) {
    return (
      <div className="flex-1 overflow-auto">
        <WelcomeState
          onSubmit={(text: string) => {
            startNewSession(text, setView, getInitialWorkingDir());
          }}
        />
      </div>
    );
  }

  // Build the list of sessions to render
  let sessionsToRender = activeSessions;

  // If we have a currentSessionId that's not in activeSessions, add it (handles page refresh)
  if (currentSessionId && !activeSessions.some((s) => s.sessionId === currentSessionId)) {
    sessionsToRender = [...activeSessions, { sessionId: currentSessionId }];
  }

  // When we're on the sessions history route, there is no active session.
  // This prevents accidentally treating "history" as a session id.
  if (location.pathname === '/sessions/history') {
    sessionsToRender = activeSessions;
  }

  return (
    <div className="relative w-full h-full">
      {sessionsToRender.map((session) => {
        const isVisible = session.sessionId === currentSessionId;

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
              isActiveSession={isVisible}
            />
          </div>
        );
      })}
    </div>
  );
}
