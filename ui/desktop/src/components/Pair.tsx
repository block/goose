import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat from './BaseChat';

export interface PairRouteState {
  resumeSessionId?: string;
  initialMessage?: string;
}

interface PairProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  sessionId: string;
  initialMessage?: string;
}

export default function Pair({
  setChat,
  setIsGoosehintsModalOpen,
  sessionId,
  initialMessage,
}: PairProps) {
  return (
    <BaseChat
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
    />
  );
}
