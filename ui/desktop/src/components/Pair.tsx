import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat from './BaseChat';

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
  return (
    <BaseChat
      setChat={setChat}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
    />
  );
}
