import { useSearchParams } from 'react-router-dom';
import BaseChat from './BaseChat';
import { ChatType } from '../types/chat';

interface ChatSessionsContainerProps {
  setChat: (chat: ChatType) => void;
  activeSessions: Array<{ sessionId: string; initialMessage?: string }>;
}

/**
 * Container that keeps all active chat sessions mounted but only displays
 * the one matching the current URL parameter. This allows sessions to continue
 * streaming in the background without being unmounted.
 */
export default function ChatSessionsContainer({
  setChat,
  activeSessions,
}: ChatSessionsContainerProps) {
  const [searchParams] = useSearchParams();
  const currentSessionId = searchParams.get('resumeSessionId') ?? undefined;
  if (activeSessions.length === 0) {
    return null;
  }

  return (
    <>
      {activeSessions.map(({ sessionId, initialMessage }) => {
        const isActive = sessionId === currentSessionId;

        return (
          <div
            key={sessionId}
            style={{
              display: isActive ? 'flex' : 'none',
              flexDirection: 'column',
              width: '100%',
              height: '100%',
            }}
          >
            <BaseChat
              setChat={setChat}
              sessionId={sessionId}
              initialMessage={initialMessage}
              suppressEmptyState={false}
              isActiveSession={isActive}
            />
          </div>
        );
      })}
    </>
  );
}
