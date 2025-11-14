import { useEffect, useState, useCallback, useMemo } from 'react';
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
  const { getRoomHistoryAsGooseMessages, sendMessage, sendGooseMessage, isConnected, isReady, onMessage, onSessionMessage } = useMatrix();
  const [isLoadingMatrixHistory, setIsLoadingMatrixHistory] = useState(false);
  const [hasLoadedMatrixHistory, setHasLoadedMatrixHistory] = useState(false);
  
  // Track all message IDs to prevent duplicates across all sources
  const [processedMessageIds, setProcessedMessageIds] = useState<Set<string>>(new Set());
  
  // Track if we've already initialized the chat to prevent reloads from clearing Matrix messages
  const [hasInitializedChat, setHasInitializedChat] = useState(false);
  
  // Track which messages have been synced to Matrix to prevent duplicates
  const [syncedMessageIds, setSyncedMessageIds] = useState<Set<string>>(new Set());

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
        // Only apply strict content matching for historical sources, be more lenient for real-time
        if (source === 'matrix-history') {
          const contentText = Array.isArray(msg.content) 
            ? msg.content.map(c => c.type === 'text' ? c.text : '').join('')
            : msg.content;
          
          const contentDuplicate = existingMessages.some(existing => {
            const existingText = Array.isArray(existing.content)
              ? existing.content.map(c => c.type === 'text' ? c.text : '').join('')
              : existing.content;
            
            // Match if same content, role, and timestamp within 10 seconds (only for history)
            return existingText === contentText && 
                   existing.role === msg.role && 
                   Math.abs(existing.created - msg.created) <= 10;
          });
          
          if (contentDuplicate) {
            console.log(`📝 Skipping duplicate message from ${source} (content match):`, {
              content: contentText.substring(0, 50) + '...',
              role: msg.role,
              timestamp: msg.created
            });
            return false;
          }
        } else if (source === 'real-time-sync') {
          // For real-time sync messages, be very lenient - only block exact duplicates with same ID prefix
          const contentText = Array.isArray(msg.content) 
            ? msg.content.map(c => c.type === 'text' ? c.text : '').join('')
            : msg.content;
          
          // Only block if we have the exact same message from the same source (same ID prefix)
          const exactDuplicate = existingMessages.some(existing => {
            // Check if both messages are from the same source type
            const newIsFromMatrix = msg.id.startsWith('matrix_') || msg.id.startsWith('shared-');
            const existingIsFromMatrix = existing.id.startsWith('matrix_') || existing.id.startsWith('shared-');
            
            // Only compare messages from the same source type
            if (newIsFromMatrix !== existingIsFromMatrix) {
              return false;
            }
            
            const existingText = Array.isArray(existing.content)
              ? existing.content.map(c => c.type === 'text' ? c.text : '').join('')
              : existing.content;
            
            // Only block if exact same content, role, and from same source type
            const isSameContent = existingText === contentText;
            const isSameRole = existing.role === msg.role;
            const isSameSourceType = newIsFromMatrix === existingIsFromMatrix;
            
            const wouldBlock = isSameContent && isSameRole && isSameSourceType;
            
            if (wouldBlock) {
              console.log(`🔍 DEDUP: Found exact duplicate for ${source} (same source type):`, {
                newMessage: {
                  id: msg.id,
                  content: contentText.substring(0, 30) + '...',
                  role: msg.role,
                  timestamp: msg.created,
                  isFromMatrix: newIsFromMatrix
                },
                existingMessage: {
                  id: existing.id,
                  content: existingText.substring(0, 30) + '...',
                  role: existing.role,
                  timestamp: existing.created,
                  isFromMatrix: existingIsFromMatrix
                }
              });
            }
            
            return wouldBlock;
          });
          
          if (exactDuplicate) {
            console.log(`📝 Skipping exact duplicate from ${source}:`, {
              content: contentText.substring(0, 50) + '...',
              role: msg.role,
              timestamp: msg.created,
              messageId: msg.id
            });
            return false;
          }
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
  // In Matrix mode, we MUST use the Matrix room ID as the session ID for proper message routing
  const effectiveSessionId = useMemo(() => {
    if (isMatrixMode && matrixRoomId) {
      console.log('🔧 Matrix mode: Using Matrix room ID as session ID:', matrixRoomId);
      return matrixRoomId;
    } else {
      console.log('🔧 Regular mode: Using chat session ID:', chat.sessionId);
      return chat.sessionId || 'default';
    }
  }, [isMatrixMode, matrixRoomId, chat.sessionId]);
  
  console.log('🔧 useSessionSharing configuration:', {
    effectiveSessionId,
    isMatrixMode,
    matrixRoomId,
    chatSessionId: chat.sessionId,
    willUseMatrixRoomId: isMatrixMode && matrixRoomId
  });
  
  // For Matrix mode, we use periodic refresh instead of complex real-time sync
  // This is simpler and more reliable than trying to sync individual messages

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
      // In Matrix mode, load history as soon as Matrix is ready, don't wait for regular chat
      if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady || hasLoadedMatrixHistory) {
        return;
      }

      // For Matrix mode, we don't need to wait for chat.sessionId since we're using the Matrix room as the session
      console.log('📜 Loading Matrix room history for collaboration:', matrixRoomId);
      setIsLoadingMatrixHistory(true);

      try {
        // Fetch room history from Matrix
        const roomHistory = await getRoomHistoryAsGooseMessages(matrixRoomId, 100); // Increased limit to get more history
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
        setHasLoadedMatrixHistory(true); // Set to true even on error to prevent infinite retries
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
    // Removed loadingChat and chat.sessionId dependencies for Matrix mode
    getRoomHistoryAsGooseMessages,
    addMessagesToChat,
  ]);

  // Periodic Matrix room refresh for real-time sync
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady) {
      return;
    }

    console.log('🔄 Setting up periodic Matrix room refresh for real-time sync');

    const refreshRoomMessages = async () => {
      try {
        console.log('🔄 Refreshing Matrix room messages...');
        
        // Fetch the latest room history
        const roomHistory = await getRoomHistoryAsGooseMessages(matrixRoomId, 100);
        console.log('🔄 Fetched', roomHistory.length, 'messages from Matrix room for refresh');

        if (roomHistory.length > 0) {
          // Convert all Matrix messages to Goose message format
          const gooseMessages: Message[] = roomHistory.map((msg) => {
            // Create stable ID based on content and timestamp
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

          // Replace all messages with the fresh room state
          console.log('🔄 Updating chat with fresh Matrix room state:', gooseMessages.length, 'messages');
          
          setChat(prevChat => ({
            ...prevChat,
            messages: gooseMessages.sort((a, b) => a.created - b.created),
          }));
        }
      } catch (error) {
        console.error('❌ Failed to refresh Matrix room messages:', error);
      }
    };

    // Initial refresh after a short delay
    const initialTimeout = setTimeout(refreshRoomMessages, 2000);
    
    // Set up periodic refresh every 3 seconds
    const refreshInterval = setInterval(refreshRoomMessages, 3000);

    return () => {
      clearTimeout(initialTimeout);
      clearInterval(refreshInterval);
      console.log('🔄 Cleaned up Matrix room refresh interval');
    };
  }, [
    isMatrixMode,
    matrixRoomId,
    isConnected,
    isReady,
    getRoomHistoryAsGooseMessages,
    setChat,
  ]);

  // Sync Goose responses to Matrix when they're added to the chat
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId) {
      return;
    }

    // Find any new assistant messages that haven't been synced to Matrix yet
    const lastMessage = chat.messages[chat.messages.length - 1];
    
    if (lastMessage && lastMessage.role === 'assistant') {
      // Check if this message was generated locally (not from Matrix) and hasn't been synced yet
      const isFromMatrix = lastMessage.id.startsWith('matrix_');
      const alreadySynced = syncedMessageIds.has(lastMessage.id);
      
      if (!isFromMatrix && !alreadySynced) {
        console.log('🤖 Syncing new Goose response to Matrix as separate user:', {
          messageId: lastMessage.id,
          role: lastMessage.role,
          content: Array.isArray(lastMessage.content) ? lastMessage.content[0]?.text?.substring(0, 50) + '...' : 'N/A'
        });
        
        // Mark this message as being synced to prevent duplicates
        setSyncedMessageIds(prev => new Set(prev).add(lastMessage.id));
        
        // Get the message content
        const messageContent = Array.isArray(lastMessage.content) 
          ? lastMessage.content.map(c => c.type === 'text' ? c.text : '').join('')
          : '';
        
        // Send as a Goose message (this will make Goose appear as a separate user)
        // Use the Matrix context's sendGooseMessage method instead of generic sendMessage
        sendGooseMessage(matrixRoomId, messageContent, 'goose.chat', {
          metadata: {
            originalMessageId: lastMessage.id,
            timestamp: lastMessage.created,
            isGooseResponse: true,
          }
        })
          .then(() => {
            console.log('✅ Goose response synced to Matrix as separate user successfully');
          })
          .catch((error) => {
            console.error('❌ Failed to sync Goose response to Matrix as separate user:', error);
            // Remove from synced set if sync failed so we can retry
            setSyncedMessageIds(prev => {
              const newSet = new Set(prev);
              newSet.delete(lastMessage.id);
              return newSet;
            });
            // Fallback to regular message if Goose message fails
            return sendMessage(matrixRoomId, messageContent);
          });
      } else if (alreadySynced) {
        console.log('🤖 Skipping Matrix sync - message already synced:', lastMessage.id);
      } else {
        console.log('🤖 Skipping Matrix sync for message from Matrix:', lastMessage.id);
      }
    }
  }, [chat.messages, isMatrixMode, matrixRoomId, sendMessage, sendGooseMessage, syncedMessageIds]);

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

  // Add a diagnostic function to test Matrix message reception
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId) return;

    // Create a test function that can be called from the browser console
    (window as any).testMatrixListeners = () => {
      console.log('🔍 DIAGNOSTIC: Testing Matrix message listeners');
      console.log('🔍 Matrix connection state:', { isConnected, isReady });
      console.log('🔍 Matrix room state:', {
        roomId: matrixRoomId,
        effectiveSessionId,
        currentMessagesCount: chat.messages.length
      });
      
      // Test if Matrix service is receiving events
      const testCallback = (data: any) => {
        console.log('🔍 DIAGNOSTIC: Received test message:', data);
      };
      
      const cleanup1 = onMessage(testCallback);
      const cleanup2 = onSessionMessage(testCallback);
      
      console.log('🔍 DIAGNOSTIC: Set up test listeners, send a message from Matrix web to test');
      
      // Clean up after 30 seconds
      setTimeout(() => {
        cleanup1();
        cleanup2();
        console.log('🔍 DIAGNOSTIC: Cleaned up test listeners');
      }, 30000);
    };

    return () => {
      delete (window as any).testMatrixListeners;
    };
  }, [isMatrixMode, matrixRoomId, isConnected, isReady, effectiveSessionId, onMessage, onSessionMessage, chat.messages.length]);

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
