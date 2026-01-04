import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat from './BaseChat';
import { useMemo } from 'react';
import { useChatStream } from '../hooks/useChatStream';

export interface PairRouteState {
  resumeSessionId?: string;
  initialMessage?: string;
}

interface PairProps {
  setChat: (chat: ChatType) => void;
  sessionId: string;
  initialMessage?: string;
}

export default function Pair({ setChat, sessionId, initialMessage }: PairProps) {
  const { session } = useChatStream({ sessionId, onStreamFinish: () => {} });

  const shouldAutoSubmit = useMemo(() => {
    if (initialMessage) return false;
    const recipe = session?.recipe;
    const hasMessages = (session?.conversation?.length ?? 0) > 0;
    return !!(recipe?.prompt && !hasMessages);
  }, [session, initialMessage]);

  return (
    <BaseChat
      setChat={setChat}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
      autoSubmit={shouldAutoSubmit}
    />
  );
}
