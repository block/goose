import { useSearchParams } from 'react-router-dom';
import BaseChat from './BaseChat';
import WelcomeState from './WelcomeState';
import { ChatType } from '../types/chat';
import { UserInput } from '../types/message';
import { startNewSession } from '../sessions';
import { useNavigation } from '../hooks/useNavigation';
import { getInitialWorkingDir } from '../utils/workingDir';

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
 */
export default function ChatSessionsContainer({
  setChat,
  activeSessions,
}: ChatSessionsContainerProps) {
  const [searchParams] = useSearchParams();
  const setView = useNavigation();
  const currentSessionId = searchParams.get('resumeSessionId') ?? undefined;

  // No active sessions — show WelcomeState with capability cards
  if (!currentSessionId && activeSessions.length === 0) {
    return (
      <WelcomeState
        onSubmit={(text) => {
          startNewSession(text, setView, getInitialWorkingDir());
        }}
      />
    );
  }

  // Build the list of sessions to render
  let sessionsToRender = activeSessions;

  // If we have a currentSessionId that's not in activeSessions, add it (handles page refresh)
  if (currentSessionId && !activeSessions.some((s) => s.sessionId === currentSessionId)) {
    sessionsToRender = [...activeSessions, { sessionId: currentSessionId }];
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
