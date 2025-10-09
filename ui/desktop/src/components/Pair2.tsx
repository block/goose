import { View, ViewOptions } from '../utils/navigationUtils';
import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import BaseChat2 from './BaseChat2';

export interface PairRouteState {
  resumeSessionId?: string;
  initialMessage?: string;
}

interface PairProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export default function Pair({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  resumeSessionId,
}: PairProps & PairRouteState) {
  return (
    <BaseChat2
      chat={chat}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      resumeSessionId={resumeSessionId}
    />
  );
}
