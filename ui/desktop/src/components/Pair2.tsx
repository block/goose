import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat2 from './BaseChat2';

export interface PairRouteState {
  resumeSessionId?: string;
  initialMessage?: string;
}

interface PairProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export default function Pair({
  setChat,
  setIsGoosehintsModalOpen,
  resumeSessionId,
}: PairProps & PairRouteState) {
  return (
    <BaseChat2
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      resumeSessionId={resumeSessionId}
    />
  );
}
