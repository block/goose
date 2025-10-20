import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat2 from './BaseChat2';

interface PairProps {
  setChat: (chat: ChatType | ((prevChat: ChatType) => ChatType)) => void;
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
    <BaseChat2
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      sessionId={sessionId}
      initialMessage={initialMessage}
      suppressEmptyState={false}
    />
  );
}
