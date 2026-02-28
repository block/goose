import { useSearchParams } from 'react-router-dom';
import BaseChat from './BaseChat';
import BrowserPanel from './BrowserPanel';
import { BrowserProvider } from './BrowserContext';
import { ChatType } from '../types/chat';
import { UserInput } from '../types/message';

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
  const [searchParams] = useSearchParams();
  const currentSessionId = searchParams.get('resumeSessionId') ?? undefined;

  if (!currentSessionId && activeSessions.length === 0) {
    return null;
  }

  let sessionsToRender = activeSessions;

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
            className={`absolute inset-0 ${isVisible ? 'flex' : 'hidden'}`}
            data-session-id={session.sessionId}
          >
            <BrowserProvider>
              <div className="flex-1 min-w-0 relative">
                <BaseChat
                  setChat={setChat}
                  sessionId={session.sessionId}
                  initialMessage={session.initialMessage}
                  suppressEmptyState={false}
                  isActiveSession={isVisible}
                />
              </div>
              <BrowserPanel />
            </BrowserProvider>
          </div>
        );
      })}
    </div>
  );
}
