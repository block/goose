import { useEffect, useState } from 'react';
import { View, ViewOptions } from '../utils/navigationUtils';
import { AgentState, InitializationContext } from '../hooks/useAgent';
import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import { useSearchParams } from 'react-router-dom';
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
  setFatalError: (value: ((prevState: string | null) => string | null) | string | null) => void;
  setAgentWaitingMessage: (msg: string | null) => void;
  agentState: AgentState;
  loadCurrentChat: (context: InitializationContext) => Promise<ChatType>;
}

export default function Pair({
  setView,
  setIsGoosehintsModalOpen,
  setFatalError,
  setAgentWaitingMessage,
  agentState,
  loadCurrentChat,
  resumeSessionId,
}: PairProps & PairRouteState) {
  const [_searchParams, setSearchParams] = useSearchParams();
  const [chat, setChat] = useState<ChatType | null>(null);

  useEffect(() => {
    const initializeFromState = async () => {
      try {
        const chat = await loadCurrentChat({
          resumeSessionId,
          setAgentWaitingMessage,
        });
        setChat(chat);
        setSearchParams((prev) => {
          prev.set('resumeSessionId', chat.sessionId);
          return prev;
        });
      } catch (error) {
        setFatalError(`Agent init failure: ${error instanceof Error ? error.message : '' + error}`);
      }
    };
    initializeFromState();
  }, [
    agentState,
    setChat,
    setFatalError,
    setAgentWaitingMessage,
    loadCurrentChat,
    resumeSessionId,
    setSearchParams,
  ]);

  return (
    <BaseChat2
      chat={chat}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  );
}
