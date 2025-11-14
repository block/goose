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
  const { getRoomHistoryAsGooseMessages, sendMessage, isConnected, isReady, onMessage, onSessionMessage, onGooseMessage } = useMatrix();
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
          console.log('📜 Sample message structure:', gooseMessages[0]);
          console.log('📜 All converted messages:', gooseMessages);
          console.log('📜 Updated chat object:', updatedChat);
          console.log('📜 Chat messages array length:', updatedChat.messages.length);
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

  // Listen for incoming Matrix messages in real-time when in Matrix mode
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId) {
      return;
    }

    console.log('👂 Setting up Matrix message listeners for room:', matrixRoomId);

    // Handle regular messages
    const unsubscribeMessage = onMessage((messageData: any) => {
      console.log('📨 Received Matrix message:', messageData);
      
      // Only process messages from the current Matrix room
      if (messageData.roomId !== matrixRoomId) {
        console.log('📨 Ignoring message from different room:', messageData.roomId);
        return;
      }

      // Don't process messages from self to avoid duplicates
      if (messageData.isFromSelf) {
        console.log('📨 Ignoring message from self');
        return;
      }

      // Convert Matrix message to Goose message format
      const newMessage: Message = {
        id: `matrix_live_${messageData.timestamp.getTime()}_${Math.random()}`,
        role: 'user',
        created: Math.floor(messageData.timestamp.getTime() / 1000),
        content: [
          {
            type: 'text' as const,
            text: messageData.content,
          }
        ],
        sender: messageData.senderInfo ? {
          userId: messageData.senderInfo.userId,
          displayName: messageData.senderInfo.displayName,
          avatarUrl: messageData.senderInfo.avatarUrl,
        } : undefined,
      };

      console.log('📨 Adding new message to chat:', newMessage);

      // Add the message to the current chat
      setChat(prevChat => ({
        ...prevChat,
        messages: [...prevChat.messages, newMessage],
      }));
    });

    // Handle session messages (from useSessionSharing)
    const unsubscribeSessionMessage = onSessionMessage((messageData: any) => {
      console.log('📝 Received Matrix session message:', messageData);
      
      // Only process messages from the current Matrix room
      if (messageData.roomId !== matrixRoomId) {
        console.log('📝 Ignoring session message from different room:', messageData.roomId);
        return;
      }

      // Don't process messages from self
      if (messageData.isFromSelf) {
        console.log('📝 Ignoring session message from self');
        return;
      }

      // Session messages are handled by useSessionSharing, so we don't need to add them to chat here
      // They will be processed and added through the normal Goose message flow
      console.log('📝 Session message will be processed by useSessionSharing');
    });

    // Handle Goose messages (AI responses, etc.)
    const unsubscribeGooseMessage = onGooseMessage((gooseMessage: any) => {
      console.log('🦆 Received Goose message:', gooseMessage);
      
      // Only process messages from the current Matrix room
      if (gooseMessage.roomId !== matrixRoomId) {
        console.log('🦆 Ignoring Goose message from different room:', gooseMessage.roomId);
        return;
      }

      // Don't process messages from self
      if (gooseMessage.metadata?.isFromSelf) {
        console.log('🦆 Ignoring Goose message from self');
        return;
      }

      // Convert Goose message to chat message format
      const newMessage: Message = {
        id: `matrix_goose_${gooseMessage.timestamp.getTime()}_${Math.random()}`,
        role: 'assistant',
        created: Math.floor(gooseMessage.timestamp.getTime() / 1000),
        content: [
          {
            type: 'text' as const,
            text: gooseMessage.content,
          }
        ],
        sender: {
          userId: gooseMessage.sender,
          displayName: gooseMessage.sender.split(':')[0]?.substring(1) || 'Goose',
        },
      };

      console.log('🦆 Adding new Goose message to chat:', newMessage);

      // Add the message to the current chat
      setChat(prevChat => ({
        ...prevChat,
        messages: [...prevChat.messages, newMessage],
      }));
    });

    // Cleanup function
    return () => {
      console.log('👂 Cleaning up Matrix message listeners');
      unsubscribeMessage();
      unsubscribeSessionMessage();
      unsubscribeGooseMessage();
    };
  }, [isMatrixMode, matrixRoomId, onMessage, onSessionMessage, onGooseMessage, setChat]);

  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(chat, chat.recipeConfig || null);

  const handleMessageSubmit = async (message: string) => {
    // Clean up any auto submit state:
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false);
    setMessageToSubmit(null);
    
    console.log('💬 Message submitted:', message);
    
    // If in Matrix mode, also send the message to Matrix room
    if (isMatrixMode && matrixRoomId && message.trim()) {
      try {
        console.log('📤 Sending message to Matrix room:', matrixRoomId);
        await sendMessage(matrixRoomId, message);
        console.log('✅ Message sent to Matrix successfully');
      } catch (error) {
        console.error('❌ Failed to send message to Matrix:', error);
      }
    }
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
      renderHeader={renderMatrixHeader} // Add Matrix header when in Matrix mode
      contentClassName={cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
      showPopularTopics={!isTransitioningFromHub && !isMatrixMode} // Don't show popular topics in Matrix mode or when transitioning from Hub
      suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
    />
  );
}
