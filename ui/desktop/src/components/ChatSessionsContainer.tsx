import { useSearchParams } from 'react-router-dom';
import BaseChat from './BaseChat';
import { ChatType } from '../types/chat';

interface ChatSessionsContainerProps {
  setChat: (chat: ChatType) => void;
  activeSessions: Array<{ sessionId: string; initialMessage?: string }>;
}

/**
 * Container that mounts only the active chat session to reduce DOM overhead.
 * The web worker continues to manage all session states in the background,
 * allowing multiple sessions to stream simultaneously.
 */
export default function ChatSessionsContainer({
  setChat,
  activeSessions,
}: ChatSessionsContainerProps) {
  const [searchParams] = useSearchParams();
  const currentSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const activeSession = activeSessions.find((s) => s.sessionId === currentSessionId);

  // If we have a currentSessionId but no activeSession, we still want to render BaseChat
  // This handles the case where we refresh the page on a session URL
  if (!currentSessionId) {
    return null;
  }

  // If we have an activeSession in our state, use its data
  // Otherwise, we're resuming a session after refresh - BaseChat will handle loading
  const sessionId = activeSession?.sessionId || currentSessionId;
  // Pass initial message if it exists - BaseChat will determine whether to submit it
  // based on whether the session has messages already
  const initialMessage = activeSession?.initialMessage;

  return (
    <BaseChat
      key={sessionId}
      setChat={setChat}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
      isActiveSession={true}
    />
  );
}
