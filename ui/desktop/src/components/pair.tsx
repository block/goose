import { useEffect, useState } from 'react';
import { View, ViewOptions } from '../utils/navigationUtils';
import BaseChat from './BaseChat';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useIsMobile } from '../hooks/use-mobile';
import { useSidebar } from './ui/sidebar';
import { AgentState, InitializationContext } from '../hooks/useAgent';
import 'react-toastify/dist/ReactToastify.css';
import { cn } from '../utils';

import { ChatType } from '../types/chat';
import { useSearchParams, useLocation } from 'react-router-dom';
import { useMatrix } from '../contexts/MatrixContext';
import { Message } from '../types/message';

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
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  setFatalError,
  setAgentWaitingMessage,
  agentState,
  loadCurrentChat,
  resumeSessionId,
  initialMessage,
}: PairProps & PairRouteState) {
  const location = useLocation();
  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const [hasProcessedInitialInput, setHasProcessedInitialInput] = useState(false);
  const [shouldAutoSubmit, setShouldAutoSubmit] = useState(false);
  const [messageToSubmit, setMessageToSubmit] = useState<string | null>(null);
  const [isTransitioningFromHub, setIsTransitioningFromHub] = useState(false);
  const [loadingChat, setLoadingChat] = useState(false);
  const [_searchParams, setSearchParams] = useSearchParams();

  // Check if we're in Matrix mode
  const routeState = location.state as ViewOptions | undefined;
  const isMatrixMode = routeState?.matrixMode || false;
  const matrixRoomId = routeState?.matrixRoomId;
  const matrixRecipientId = routeState?.matrixRecipientId;

  // Matrix integration
  const { getRoomHistoryAsGooseMessages, isConnected, isReady } = useMatrix();
  const [isLoadingMatrixHistory, setIsLoadingMatrixHistory] = useState(false);
  const [hasLoadedMatrixHistory, setHasLoadedMatrixHistory] = useState(false);

  useEffect(() => {
    const initializeFromState = async () => {
      setLoadingChat(true);
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
        console.log(error);
        setFatalError(`Agent init failure: ${error instanceof Error ? error.message : '' + error}`);
      } finally {
        setLoadingChat(false);
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

  // Followed by sending the initialMessage if we have one. This will happen
  // only once, unless we reset the chat in step one.
  useEffect(() => {
    if (agentState !== AgentState.INITIALIZED || !initialMessage || hasProcessedInitialInput) {
      return;
    }

    setIsTransitioningFromHub(true);
    setHasProcessedInitialInput(true);
    setMessageToSubmit(initialMessage);
    setShouldAutoSubmit(true);
  }, [agentState, initialMessage, hasProcessedInitialInput]);

  useEffect(() => {
    if (agentState === AgentState.NO_PROVIDER) {
      setView('welcome');
    }
  }, [agentState, setView]);

  // Load Matrix room history when in Matrix mode
  useEffect(() => {
    const loadMatrixHistory = async () => {
      if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady || hasLoadedMatrixHistory || loadingChat) {
        return;
      }

      console.log('📜 Loading Matrix room history for collaboration:', matrixRoomId);
      setIsLoadingMatrixHistory(true);

      try {
        // Fetch room history from Matrix
        const roomHistory = await getRoomHistoryAsGooseMessages(matrixRoomId, 50);
        console.log('📜 Fetched', roomHistory.length, 'messages from Matrix room');

        if (roomHistory.length > 0) {
          // Convert Matrix messages to Goose message format
          const gooseMessages: Message[] = roomHistory.map((msg, index) => ({
            id: `matrix_${index}_${msg.timestamp.getTime()}`,
            role: msg.role as 'user' | 'assistant',
            created: Math.floor(msg.timestamp.getTime() / 1000),
            content: [
              {
                type: 'text' as const,
                text: msg.content,
              }
            ],
            sender: msg.metadata?.senderInfo ? {
              userId: msg.metadata.senderInfo.userId,
              displayName: msg.metadata.senderInfo.displayName,
              avatarUrl: msg.metadata.senderInfo.avatarUrl,
            } : undefined,
          }));

          // Update the chat with Matrix history
          const updatedChat: ChatType = {
            ...chat,
            messages: gooseMessages,
          };

          console.log('📜 Loaded Matrix collaboration history:', gooseMessages.length, 'messages');
          setChat(updatedChat);
        } else {
          console.log('📜 No previous messages found in Matrix room');
        }

        setHasLoadedMatrixHistory(true);
      } catch (error) {
        console.error('❌ Failed to load Matrix room history:', error);
      } finally {
        setIsLoadingMatrixHistory(false);
      }
    };

    loadMatrixHistory();
  }, [
    isMatrixMode,
    matrixRoomId,
    isConnected,
    isReady,
    hasLoadedMatrixHistory,
    loadingChat,
    getRoomHistoryAsGooseMessages,
    chat,
    setChat,
  ]);

  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(chat, chat.recipeConfig || null);

  const handleMessageSubmit = (message: string) => {
    // Clean up any auto submit state:
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false);
    setMessageToSubmit(null);
    console.log('Message submitted:', message);
  };

  const recipePrompt =
    agentState === 'initialized' && chat.messages.length === 0 && recipeInitialPrompt;

  const initialValue = messageToSubmit || recipePrompt || undefined;

  const customChatInputProps = {
    // Pass initial message from Hub or recipe prompt
    initialValue,
  };

  // Matrix collaboration header
  const renderMatrixHeader = () => {
    if (!isMatrixMode || !matrixRoomId) return null;
    
    const collaboratorName = matrixRecipientId?.split(':')[0]?.substring(1) || 'Unknown';
    
    return (
      <div className="flex items-center gap-3 p-4 border-b border-border-default bg-background-muted">
        <button
          onClick={() => setView('chat', { resetChat: true })}
          className="flex items-center gap-2 px-3 py-2 text-sm text-text-muted hover:text-text-default hover:bg-background-subtle rounded-lg transition-colors"
        >
          ← Back to Chat
        </button>
        <div className="flex-1">
          <h2 className="text-lg font-medium text-text-default">
            🤝 Matrix Collaboration
          </h2>
          <p className="text-sm text-text-muted">
            Room: {matrixRoomId} • Collaborating with {collaboratorName}
          </p>
        </div>
      </div>
    );
  };

  return (
    <BaseChat
      chat={chat}
      loadingChat={loadingChat || isLoadingMatrixHistory} // Include Matrix history loading
      autoSubmit={shouldAutoSubmit}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      onMessageSubmit={handleMessageSubmit}
      customChatInputProps={customChatInputProps}
      renderHeader={renderMatrixHeader} // Add Matrix header when in Matrix mode
      contentClassName={cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
      showPopularTopics={!isTransitioningFromHub && !isMatrixMode} // Don't show popular topics in Matrix mode or when transitioning from Hub
      suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
    />
  );
}
