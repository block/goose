import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat2 from './BaseChat2';

interface PairProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  resumeSessionId?: string;
  initialMessage?: string;
}

export default function Pair({ setChat, setIsGoosehintsModalOpen, resumeSessionId }: PairProps) {
  return (
    <BaseChat2
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      resumeSessionId={resumeSessionId}
    />
  );
}
