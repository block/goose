/**
 * Unified agent chat hook that routes between Goose and Pi backends.
 *
 * This hook provides a single interface for chat functionality,
 * delegating to either useChatStream (Goose/goosed) or usePiChat (Pi)
 * based on the backend setting.
 *
 * Usage:
 *   const chat = useAgentChat({ sessionId, backend: 'goose' }); // or 'pi'
 */

import { useChatStream } from './useChatStream';
import { usePiChat } from './usePiChat';
import { Message, Session, TokenState } from '../api';
import { ChatState } from '../types/chatState';
import { NotificationEvent, UserInput } from '../types/message';

export type AgentBackend = 'goose' | 'pi';

interface UseAgentChatProps {
  sessionId: string;
  backend: AgentBackend;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

interface UseAgentChatReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  setChatState: (state: ChatState) => void;
  handleSubmit: (input: UserInput) => Promise<void>;
  submitElicitationResponse: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<void>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState: TokenState;
  notifications: Map<string, NotificationEvent[]>;
  onMessageUpdate: (
    messageId: string,
    newContent: string,
    editType?: 'fork' | 'edit'
  ) => Promise<void>;
  backend: AgentBackend;
}

export function useAgentChat({
  sessionId,
  backend,
  onStreamFinish,
  onSessionLoaded,
}: UseAgentChatProps): UseAgentChatReturn {
  // Only activate the hook for the selected backend
  // This prevents both hooks from running simultaneously
  
  const gooseChat = useChatStream({
    sessionId: backend === 'goose' ? sessionId : '',
    onStreamFinish: backend === 'goose' ? onStreamFinish : () => {},
    onSessionLoaded: backend === 'goose' ? onSessionLoaded : undefined,
  });

  const piChat = usePiChat({
    sessionId: backend === 'pi' ? sessionId : '',
    onStreamFinish: backend === 'pi' ? onStreamFinish : () => {},
    onSessionLoaded: backend === 'pi' ? onSessionLoaded : undefined,
  });

  if (backend === 'pi') {
    return {
      session: piChat.session,
      messages: piChat.messages,
      chatState: piChat.chatState,
      setChatState: piChat.setChatState,
      handleSubmit: piChat.handleSubmit,
      submitElicitationResponse: piChat.submitElicitationResponse,
      setRecipeUserParams: piChat.setRecipeUserParams,
      stopStreaming: piChat.stopStreaming,
      sessionLoadError: piChat.sessionLoadError,
      tokenState: piChat.tokenState,
      notifications: piChat.notifications,
      onMessageUpdate: piChat.onMessageUpdate,
      backend: 'pi',
    };
  }

  // Default to Goose backend
  return {
    session: gooseChat.session,
    messages: gooseChat.messages,
    chatState: gooseChat.chatState,
    setChatState: gooseChat.setChatState,
    handleSubmit: gooseChat.handleSubmit,
    submitElicitationResponse: gooseChat.submitElicitationResponse,
    setRecipeUserParams: gooseChat.setRecipeUserParams,
    stopStreaming: gooseChat.stopStreaming,
    sessionLoadError: gooseChat.sessionLoadError,
    tokenState: gooseChat.tokenState,
    notifications: gooseChat.notifications,
    onMessageUpdate: gooseChat.onMessageUpdate,
    backend: 'goose',
  };
}
