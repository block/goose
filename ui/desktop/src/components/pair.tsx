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
import { useSessionSharing } from '../hooks/useSessionSharing';

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
  
  // Check if we're in Matrix mode using URL parameters
  const [searchParams, setSearchParams] = useSearchParams();
  const isMatrixMode = searchParams.get('matrixMode') === 'true';
  const matrixRoomId = searchParams.get('matrixRoomId');
  const matrixRecipientId = searchParams.get('matrixRecipientId');

  // Debug Matrix mode detection
  console.log('🔍 Matrix mode detection:');
  console.log('🔍 URL search params:', Object.fromEntries(searchParams.entries()));
  console.log('🔍 matrixMode param:', searchParams.get('matrixMode'));
  console.log('🔍 matrixRoomId param:', searchParams.get('matrixRoomId'));
  console.log('🔍 matrixRecipientId param:', searchParams.get('matrixRecipientId'));
  console.log('🔍 isMatrixMode result:', isMatrixMode);

  // Matrix integration
  const { getRoomHistoryAsGooseMessages, sendMessage, isConnected, isReady } = useMatrix();
  const [isLoadingMatrixHistory, setIsLoadingMatrixHistory] = useState(false);
  const [hasLoadedMatrixHistory, setHasLoadedMatrixHistory] = useState(false);

  // Session sharing hook for Matrix collaboration
  const sessionSharing = useSessionSharing({
    sessionId: chat.sessionId || 'default',
    sessionTitle: chat.title || `Matrix Collaboration ${matrixRoomId?.substring(0, 8) || 'Session'}`,
    messages: chat.messages,
    onMessageSync: (message) => {
      // Handle synced messages from Matrix session participants
      console.log('💬 Synced message from Matrix shared session:', message);
      // Add the synced message to local chat
      setChat(prevChat => ({
        ...prevChat,
        messages: [...prevChat.messages, message],
      }));
    },
  });

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
      // Wait for regular chat to load first, then load Matrix history
      if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady || hasLoadedMatrixHistory || loadingChat) {
        return;
      }

      // Also ensure we have a valid chat session before loading Matrix history
      if (!chat.sessionId) {
        console.log('📜 Waiting for chat session to initialize before loading Matrix history');
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

          // Merge Matrix history with existing chat messages (avoid duplicates)
          setChat(prevChat => {
            // Filter out any existing Matrix messages to avoid duplicates
            const nonMatrixMessages = prevChat.messages.filter(msg => !msg.id.startsWith('matrix_'));
            
            // Combine and sort by timestamp
            const allMessages = [...nonMatrixMessages, ...gooseMessages].sort((a, b) => a.created - b.created);
            
            console.log('📜 Merged Matrix history with existing chat:', {
              existingMessages: nonMatrixMessages.length,
              matrixMessages: gooseMessages.length,
              totalMessages: allMessages.length
            });

            return {
              ...prevChat,
              messages: allMessages,
            };
          });

          console.log('📜 Loaded Matrix collaboration history:', gooseMessages.length, 'messages');
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
    chat.sessionId, // Add this dependency to wait for chat initialization
    getRoomHistoryAsGooseMessages,
    setChat,
  ]);

  // Matrix real-time messages are handled by useSessionSharing hook
  // No need for separate Matrix listeners since useSessionSharing processes session messages correctly

  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(chat, chat.recipeConfig || null);

  const handleMessageSubmit = async (message: string) => {
    // Clean up any auto submit state:
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false);
    setMessageToSubmit(null);
    
    console.log('💬 Message submitted in Matrix mode:', { message, isMatrixMode, matrixRoomId });
    
    // If in Matrix mode, also send the message to Matrix room
    if (isMatrixMode && matrixRoomId && message.trim()) {
      try {
        console.log('📤 Sending message to Matrix room:', matrixRoomId, 'Message:', message);
        await sendMessage(matrixRoomId, message);
        console.log('✅ Message sent to Matrix successfully');
      } catch (error) {
        console.error('❌ Failed to send message to Matrix:', error);
      }
    } else {
      console.log('📤 Not sending to Matrix:', { isMatrixMode, matrixRoomId, hasMessage: !!message.trim() });
    }
  };

  const recipePrompt =
    agentState === 'initialized' && chat.messages.length === 0 && recipeInitialPrompt;

  const initialValue = messageToSubmit || recipePrompt || undefined;

  const customChatInputProps = {
    // Pass initial message from Hub or recipe prompt
    initialValue,
  };

  // Matrix collaboration should be invisible - just a regular chat with Matrix sync
  // No special header needed - users should see normal Goose chat interface

  // Debug the chat state before rendering
  console.log('🎯 Pair component rendering with chat:', chat);
  console.log('🎯 Chat messages count:', chat.messages?.length || 0);
  console.log('🎯 Is Matrix mode:', isMatrixMode);
  console.log('🎯 Loading states:', { loadingChat, isLoadingMatrixHistory });

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
      contentClassName={cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
      showPopularTopics={!isTransitioningFromHub} // Show popular topics in all modes, including Matrix
      suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
    />
  );
}
