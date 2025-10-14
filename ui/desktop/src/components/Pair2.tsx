import { useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
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
  const [_searchParams, setSearchParams] = useSearchParams();

  // Update URL with sessionId to persist across refreshes
  // Only update if resumeSessionId is not already set (to avoid overwriting it on mount)
  useEffect(() => {
    if (chat.sessionId && !resumeSessionId) {
      setSearchParams((prev) => {
        prev.set('resumeSessionId', chat.sessionId);
        return prev;
      });
    }
  }, [chat.sessionId, resumeSessionId, setSearchParams]);

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
