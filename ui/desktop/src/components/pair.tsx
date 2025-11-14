import { useEffect, useState, useCallback } from 'react';
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
  
  // Track all message IDs to prevent duplicates across all sources
  const [processedMessageIds, setProcessedMessageIds] = useState<Set<string>>(new Set());
  
  // Track if we've already initialized the chat to prevent reloads from clearing Matrix messages
  const [hasInitializedChat, setHasInitializedChat] = useState(false);

  // Centralized message management function
  const addMessagesToChat = useCallback((newMessages: Message[], source: string) => {
    console.log(`📝 Adding ${newMessages.length} messages from ${source}`);
    
    setChat(prevChat => {
      // Create comprehensive deduplication checks
      const existingMessages = prevChat.messages;
      
      // Filter out duplicates using multiple criteria
      const uniqueNewMessages = newMessages.filter(msg => {
        // Check 1: Exact ID match
        const idDuplicate = existingMessages.some(existing => existing.id === msg.id);
        if (idDuplicate) {
          console.log(`📝 Skipping duplicate message from ${source} (ID match):`, msg.id);
          return false;
        }
        
        // Check 2: Already processed ID
        if (processedMessageIds.has(msg.id)) {
          console.log(`📝 Skipping already processed message from ${source}:`, msg.id);
          return false;
        }
        
        // Check 3: Content + timestamp + role match (for messages with different IDs but same content)
        const contentText = Array.isArray(msg.content) 
          ? msg.content.map(c => c.type === 'text' ? c.text : '').join('')
          : msg.content;
        
        const contentDuplicate = existingMessages.some(existing => {
          const existingText = Array.isArray(existing.content)
            ? existing.content.map(c => c.type === 'text' ? c.text : '').join('')
            : existing.content;
          
          // Match if same content, role, and timestamp within 5 seconds
          return existingText === contentText && 
                 existing.role === msg.role && 
                 Math.abs(existing.created - msg.created) <= 5;
        });
        
        if (contentDuplicate) {
          console.log(`📝 Skipping duplicate message from ${source} (content match):`, {
            content: contentText.substring(0, 50) + '...',
            role: msg.role,
            timestamp: msg.created
          });
          return false;
        }
        
        return true;
      });
      
      if (uniqueNewMessages.length === 0) {
        console.log(`📝 No new unique messages from ${source}`);
        return prevChat;
      }
      
      // Combine all messages and sort by timestamp
      const allMessages = [...prevChat.messages, ...uniqueNewMessages]
        .sort((a, b) => a.created - b.created);
      
      // Update processed message IDs
      setProcessedMessageIds(prev => {
        const newSet = new Set(prev);
        uniqueNewMessages.forEach(msg => newSet.add(msg.id));
        return newSet;
      });
      
      console.log(`📝 Added ${uniqueNewMessages.length} unique messages from ${source}. Total: ${allMessages.length}`);
      console.log(`📝 Message details:`, uniqueNewMessages.map(msg => ({
        id: msg.id,
        role: msg.role,
        content: Array.isArray(msg.content) ? msg.content[0]?.text?.substring(0, 30) + '...' : 'N/A',
        timestamp: msg.created
      })));
      
      return {
        ...prevChat,
        messages: allMessages,
      };
    });
  }, [setChat, processedMessageIds]);

  // Session sharing hook for Matrix collaboration
  // In Matrix mode, we need to use a session ID that matches what's being sent in Matrix messages
  const effectiveSessionId = isMatrixMode && matrixRoomId ? matrixRoomId : (chat.sessionId || 'default');
  
  console.log('🔧 useSessionSharing configuration:', {
    effectiveSessionId,
    isMatrixMode,
    matrixRoomId,
    chatSessionId: chat.sessionId
  });
  
  const sessionSharing = useSessionSharing({
    sessionId: effectiveSessionId,
    sessionTitle: chat.title || `Matrix Collaboration ${matrixRoomId?.substring(0, 8) || 'Session'}`,
    messages: chat.messages,
    onMessageSync: (message) => {
      // Handle synced messages from Matrix session participants
      console.log('💬 Synced message from Matrix shared session:', message);
      console.log('💬 Session ID match check:', { effectiveSessionId, isMatrixMode, matrixRoomId });
      
      // Only add real-time messages, not historical ones (avoid duplicates with history loading)
      // Skip messages that are older than 30 seconds to avoid processing historical messages as real-time
      const messageAge = Date.now() / 1000 - message.created;
      const isRecentMessage = messageAge < 30; // Messages within last 30 seconds are considered real-time
      
      console.log('💬 Message age check:', { messageAge, isRecentMessage, messageId: message.id });
      
      if (isRecentMessage) {
        // Use centralized message management for real-time messages
        addMessagesToChat([message], 'real-time-sync');
      } else {
        console.log('💬 Skipping old message (likely from history):', message.id);
      }
    },
  });

  useEffect(() => {
    const initializeFromState = async () => {
      console.log('🔄 initializeFromState called with:', { 
        agentState, 
        resumeSessionId, 
        isMatrixMode, 
        hasInitializedChat,
        currentChatMessagesCount: chat.messages?.length || 0 
      });
      
      // Skip initialization if we're in Matrix mode and have already initialized
      if (isMatrixMode && hasInitializedChat && chat.messages.length > 0) {
        console.log('⚠️ Skipping chat reload in Matrix mode - already initialized with messages');
        return;
      }
      
      setLoadingChat(true);
      try {
        const loadedChat = await loadCurrentChat({
          resumeSessionId,
          setAgentWaitingMessage,
        });
        
        console.log('📥 loadCurrentChat returned:', { 
          sessionId: loadedChat.sessionId, 
          messagesCount: loadedChat.messages?.length || 0,
          isMatrixMode 
        });
        
        setChat(loadedChat);
        setHasInitializedChat(true);
        
        setSearchParams((prev) => {
          prev.set('resumeSessionId', loadedChat.sessionId);
          return prev;
        });
      } catch (error) {
        console.log('❌ loadCurrentChat error:', error);
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
          const gooseMessages: Message[] = roomHistory.map((msg, index) => {
            // Create more stable ID based on content and timestamp to help with deduplication
            const contentHash = msg.content.substring(0, 50).replace(/[^a-zA-Z0-9]/g, '');
            const stableId = `matrix_${msg.timestamp.getTime()}_${msg.role}_${contentHash}`;
            
            return {
              id: stableId,
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
            };
          });

          // Use centralized message management for Matrix history
          addMessagesToChat(gooseMessages, 'matrix-history');
          
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
    addMessagesToChat,
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
