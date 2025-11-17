import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
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
import { sessionMappingService } from '../services/SessionMappingService';

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
  const { getRoomHistoryAsGooseMessages, sendMessage, sendGooseMessage, isConnected, isReady, onMessage, onSessionMessage, currentUser, rooms } = useMatrix();
  const [isLoadingMatrixHistory, setIsLoadingMatrixHistory] = useState(false);
  const [hasLoadedMatrixHistory, setHasLoadedMatrixHistory] = useState(false);
  
  // Track all message IDs to prevent duplicates across all sources
  const [processedMessageIds, setProcessedMessageIds] = useState<Set<string>>(new Set());
  
  // Track if we've already initialized the chat to prevent reloads from clearing Matrix messages
  const [hasInitializedChat, setHasInitializedChat] = useState(false);
  
  // Track which messages have been synced to Matrix to prevent duplicates
  const [syncedMessageIds, setSyncedMessageIds] = useState<Set<string>>(new Set());
  
  // Track pending sync operations to debounce rapid message changes
  const syncTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastSyncedContentRef = useRef<string>('');

  // Reset state when Matrix room changes (switching between different peer DMs)
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId) {
      return;
    }
    
    console.log('🔄 Matrix room changed, resetting state and loading new session for room:', matrixRoomId);
    
    // Reset all Matrix-related state when room changes
    setHasLoadedMatrixHistory(false);
    setHasInitializedChat(false);
    setProcessedMessageIds(new Set());
    setSyncedMessageIds(new Set());
    
    // Clear any pending sync timeout
    if (syncTimeoutRef.current) {
      clearTimeout(syncTimeoutRef.current);
      syncTimeoutRef.current = null;
    }
    
    // Reset loading state
    setIsLoadingMatrixHistory(false);
    
    // CRITICAL: Clear the current chat messages to prevent showing wrong session
    setChat(prevChat => ({
      ...prevChat,
      messages: [], // Clear messages immediately
      sessionId: matrixRoomId, // Set to Matrix room ID temporarily
    }));
    
    console.log('✅ State reset complete for Matrix room:', matrixRoomId);
    
    // Force initialization of the new session
    const initializeNewSession = async () => {
      console.log('🔄 Force initializing new session for Matrix room:', matrixRoomId);
      
      // Get or create the session mapping
      let gooseSessionId = sessionMappingService.getGooseSessionId(matrixRoomId);
      
      if (!gooseSessionId) {
        console.log('🔄 No mapping found, creating NEW DM-specific session mapping for Matrix room:', matrixRoomId);
        
        // Create a mapping for this Matrix room - ensure it's DM-specific
        const currentRoom = rooms.find(room => room.roomId === matrixRoomId);
        const isDM = currentRoom?.members?.length === 2 || true; // Assume DM for peer chats
        const roomName = isDM ? `DM with ${matrixRecipientId || 'User'}` : currentRoom?.name || `Matrix Room ${matrixRoomId.substring(1, 8)}`;
        
        try {
          // ALWAYS create a NEW backend session for DMs to avoid shared session contamination
          console.log('🔄 Creating FRESH backend session for DM:', { matrixRoomId, roomName, isDM });
          const mapping = await sessionMappingService.createMappingWithBackendSession(matrixRoomId, [], roomName);
          gooseSessionId = mapping.gooseSessionId;
          console.log('✅ Created NEW backend session mapping for DM:', { matrixRoomId, gooseSessionId });
        } catch (error) {
          console.error('❌ Failed to create backend session mapping, using fallback:', error);
          // Fallback to regular mapping
          const mapping = sessionMappingService.createMapping(matrixRoomId, [], roomName);
          gooseSessionId = mapping.gooseSessionId;
        }
      } else {
        console.log('🔄 Found existing session mapping for Matrix room:', { matrixRoomId, gooseSessionId });
      }
      
      // Load the backend session for this Matrix DM to get any existing messages
      setLoadingChat(true);
      try {
        const loadedChat = await loadCurrentChat({
          resumeSessionId: gooseSessionId,
          setAgentWaitingMessage,
        });
        
        console.log('📥 Matrix DM: loadCurrentChat returned for room:', { 
          sessionId: loadedChat.sessionId, 
          messagesCount: loadedChat.messages?.length || 0,
          matrixRoomId,
          gooseSessionId,
        });
        
        // IMPORTANT: For DMs, start with empty messages and only load from Matrix history
        // This prevents loading shared session history that might not belong to this DM
        const matrixChat: ChatType = {
          ...loadedChat,
          messages: [], // Start empty - Matrix history will be loaded separately
          aiEnabled: false, // AI is disabled by default for Matrix DMs - use @goose to enable
        };
        
        setChat(matrixChat);
        setHasInitializedChat(true);
        
        // Update URL params to reflect the proper session ID
        setSearchParams((prev) => {
          prev.set('resumeSessionId', gooseSessionId);
          return prev;
        });
        
        console.log('✅ Matrix DM session initialized with empty messages - Matrix history will load separately');
      } catch (error) {
        console.log('❌ Matrix DM loadCurrentChat error for new room:', error);
        setFatalError(`Matrix DM init failure: ${error instanceof Error ? error.message : '' + error}`);
      } finally {
        setLoadingChat(false);
      }
    };
    
    // Initialize the new session after a short delay to ensure state is reset
    setTimeout(initializeNewSession, 100);
    
  }, [matrixRoomId, isMatrixMode]); // Depend on both matrixRoomId and isMatrixMode

  // Centralized message management function
  const addMessagesToChat = useCallback((newMessages: Message[], source: string) => {
    console.log(`🔍 addMessagesToChat called with source: "${source}", isMatrixMode: ${isMatrixMode}`);
    
    // In Matrix mode, only skip the old real-time-sync messages, but allow matrix-real-time from custom events
    // The matrix-real-time source now comes from ChatInput via custom events, so we should process it
    if (isMatrixMode && source === 'real-time-sync') {
      console.log(`🚫 Skipping ${source} in Matrix mode - handled by useSessionSharing + custom events`);
      return;
    }
    
    // Additional check for matrix-real-time to ensure it's processed
    if (isMatrixMode && source === 'matrix-real-time') {
      console.log(`✅ Processing ${source} in Matrix mode - from ChatInput custom event`);
    }
    
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
  }, [setChat, processedMessageIds, isMatrixMode]);

  // Session sharing hook for Matrix collaboration
  // In Matrix mode, we need to get the proper Goose session ID for the Matrix room
  const effectiveSessionId = useMemo(() => {
    if (isMatrixMode && matrixRoomId) {
      // Get the mapped Goose session ID for this Matrix room
      const gooseSessionId = sessionMappingService.getGooseSessionId(matrixRoomId);
      if (gooseSessionId) {
        console.log('🔧 Matrix mode: Using mapped Goose session ID:', { matrixRoomId, gooseSessionId });
        return gooseSessionId;
      } else {
        console.log('🔧 Matrix mode: No mapping found for Matrix room, using room ID for session sharing only:', matrixRoomId);
        // For session sharing (useSessionSharing), we still use the Matrix room ID
        // This allows message routing to work, but backend calls will be skipped
        return matrixRoomId;
      }
    } else {
      console.log('🔧 Regular mode: Using chat session ID:', chat.sessionId);
      return chat.sessionId || 'default';
    }
  }, [isMatrixMode, matrixRoomId, chat.sessionId]);
  
  console.log('🔧 Matrix configuration:', {
    effectiveSessionId,
    isMatrixMode,
    matrixRoomId,
    chatSessionId: chat.sessionId,
    willUseMatrixRoomId: isMatrixMode && matrixRoomId
  });
  
  // For Matrix mode, we use periodic refresh instead of complex real-time sync
  // This is simpler and more reliable than trying to sync individual messages

  useEffect(() => {
    // Skip this initialization effect if we're in Matrix mode and already have initialized chat
    // The Matrix room change effect handles initialization for Matrix mode
    if (isMatrixMode && hasInitializedChat) {
      console.log('⚠️ Skipping initializeFromState in Matrix mode - already initialized');
      return;
    }
    
    const initializeFromState = async () => {
      console.log('🔄 initializeFromState called with:', { 
        agentState, 
        resumeSessionId, 
        isMatrixMode, 
        hasInitializedChat,
        currentChatMessagesCount: chat.messages?.length || 0 
      });
      
      // Skip if we're in Matrix mode - handled by the Matrix room change effect
      if (isMatrixMode && matrixRoomId) {
        console.log('⚠️ Skipping initializeFromState - Matrix mode handled by room change effect');
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
    // Removed isMatrixMode and matrixRoomId to prevent conflicts with Matrix room change effect
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
      // In Matrix mode, load history ONLY after backend session is initialized
      if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady || hasLoadedMatrixHistory || !hasInitializedChat) {
        return;
      }

      // Wait for the backend session to be properly set up before loading Matrix history
      console.log('📜 Loading Matrix room history for DM (after backend session init):', matrixRoomId);
      setIsLoadingMatrixHistory(true);

      try {
        // Fetch room history from Matrix - increased limit for DM rooms
        const roomHistory = await getRoomHistoryAsGooseMessages(matrixRoomId, 1000); // Much higher limit for DM history
        console.log('📜 Fetched', roomHistory.length, 'messages from Matrix room');

        if (roomHistory.length > 0) {
          // Convert Matrix messages to Goose message format
          const gooseMessages: Message[] = roomHistory.map((msg, index) => {
            // Detect if this is a Goose message and correct the role if needed
            let messageRole = msg.role as 'user' | 'assistant';
            
            // Check if the message content indicates it's from Goose
            const isGooseMessage = msg.content && (
              msg.content.startsWith('🦆 Goose:') ||
              msg.content.includes('🦆 Goose:') ||
              msg.content.startsWith('🤖') ||
              (msg.metadata?.senderInfo?.displayName && msg.metadata.senderInfo.displayName.toLowerCase().includes('goose')) ||
              (msg.sender && msg.sender.toLowerCase().includes('goose'))
            );
            
            // Override role if it's detected as a Goose message
            if (isGooseMessage && messageRole === 'user') {
              messageRole = 'assistant';
              console.log('📜 Corrected role from user to assistant for Goose message in history:', msg.content.substring(0, 50) + '...');
            }
            
            // Create more stable ID based on content and timestamp to help with deduplication
            const contentHash = msg.content.substring(0, 50).replace(/[^a-zA-Z0-9]/g, '');
            const stableId = `matrix_${msg.timestamp.getTime()}_${messageRole}_${contentHash}`;
            
            return {
              id: stableId,
              role: messageRole,
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
    hasInitializedChat, // Wait for backend session to be initialized
    getRoomHistoryAsGooseMessages,
    // Removed addMessagesToChat from dependencies to prevent re-render loop
  ]);

  // Listen for Matrix room messages directly and update chat state
  // NOTE: Only for multi-user rooms (>2 people). 1-on-1 chats are handled by MatrixService message events
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId || !isConnected || !isReady) {
      return;
    }

    // Check if this is a multi-user room (more than 2 people)
    const currentRoom = rooms.find(room => room.roomId === matrixRoomId);
    const memberCount = currentRoom?.members?.length || 0;
    
    if (memberCount <= 2) {
      console.log('🔄 Skipping direct Matrix listener for 1-on-1 chat - handled by MatrixService message events');
      return;
    }

    console.log('🔄 Setting up direct Matrix room message listener in pair.tsx for multi-user room');

    const handleMatrixRoomMessage = (messageData: any) => {
      const { content, sender, roomId, timestamp, senderInfo } = messageData;
      
      // Only process messages from our specific Matrix room
      if (roomId !== matrixRoomId) {
        return;
      }
      
      // Skip messages from ourselves
      if (sender === currentUser?.userId) {
        return;
      }
      
      console.log('📨 pair.tsx: Received Matrix room message:', {
        sender,
        content: content?.substring(0, 50) + '...',
        roomId
      });
      
      // Determine message role based on content
      let messageRole: 'user' | 'assistant' = 'user';
      
      // Detect Goose messages
      const isGooseResponse = content && (
        content.includes('🦆 Goose:') ||
        content.startsWith('🦆 Goose:') ||
        content.includes('🤖') ||
        (senderInfo?.displayName && senderInfo.displayName.toLowerCase().includes('goose')) ||
        (sender && sender.toLowerCase().includes('goose'))
      );
      
      if (isGooseResponse) {
        messageRole = 'assistant';
      }
      
      // Create message in Goose format
      const gooseMessage: Message = {
        id: `matrix-${timestamp.getTime()}-${Math.random().toString(36).substr(2, 9)}`,
        role: messageRole,
        created: Math.floor(timestamp.getTime() / 1000),
        content: [{
          type: 'text',
          text: content,
        }],
        sender: senderInfo ? {
          userId: senderInfo.userId,
          displayName: senderInfo.displayName,
          avatarUrl: senderInfo.avatarUrl,
        } : {
          userId: sender,
          displayName: sender.split(':')[0].substring(1),
        },
        metadata: {
          skipLocalResponse: true,
          isFromCollaborator: true,
          preventAutoResponse: true,
          isMatrixSharedSession: true,
          matrixRoomId: matrixRoomId
        }
      };
      
      // Add directly to chat state using the callback to avoid dependency on addMessagesToChat
      addMessagesToChat([gooseMessage], 'matrix-room-direct');
    };

    // Listen for Matrix room messages
    const cleanup = onMessage(handleMatrixRoomMessage);

    return () => {
      console.log('🔄 Cleaning up direct Matrix room message listener in pair.tsx');
      cleanup();
    };
  }, [
    isMatrixMode,
    matrixRoomId,
    isConnected,
    isReady,
    currentUser?.userId,
    onMessage,
    rooms,
    // Removed addMessagesToChat from dependencies to prevent re-render loop
  ]);

  // Sync Goose responses to Matrix when they're added to the chat (simplified, no complex debouncing)
  // Only sync if the current user asked the last question
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId || !currentUser) {
      console.log('🤖 Skipping Goose sync - missing requirements:', {
        isMatrixMode,
        hasMatrixRoomId: !!matrixRoomId,
        hasCurrentUser: !!currentUser
      });
      return;
    }

    // Find any new assistant messages that haven't been synced to Matrix yet
    const lastMessage = chat.messages[chat.messages.length - 1];
    
    if (!lastMessage) {
      console.log('🤖 No messages in chat yet');
      return;
    }
    
    if (lastMessage.role !== 'assistant') {
      console.log('🤖 Last message is not from assistant:', lastMessage.role);
      return;
    }
    
    console.log('🤖 Checking assistant message for Matrix sync:', {
      messageId: lastMessage.id,
      role: lastMessage.role,
      content: Array.isArray(lastMessage.content) ? lastMessage.content[0]?.text?.substring(0, 50) + '...' : 'N/A',
      alreadySynced: syncedMessageIds.has(lastMessage.id)
    });
    
    // Check if this message was generated locally (not from Matrix)
    const isFromMatrix = lastMessage.id.startsWith('matrix_');
    
    if (isFromMatrix) {
      console.log('🤖 Skipping Matrix sync for message from Matrix:', lastMessage.id);
      return;
    }
    
    // Check if already synced
    if (syncedMessageIds.has(lastMessage.id)) {
      console.log('🤖 Message already synced to Matrix:', lastMessage.id);
      return;
    }
    
    // Find the last user message to determine who asked the question
    const lastUserMessage = [...chat.messages].reverse().find(msg => msg.role === 'user');
    
    if (!lastUserMessage) {
      console.log('🤖 No user message found to determine who asked the question');
      return;
    }
    
    // Only sync Goose response if the current user asked the last question
    const shouldSync = lastUserMessage && (
      // If the message has sender info, check if it's from current user
      lastUserMessage.sender?.userId === currentUser.userId ||
      // If no sender info, assume it's from current user (local message)
      !lastUserMessage.sender
    );
    
    console.log('🤖 User check for Goose response sync:', {
      lastUserMessageSender: lastUserMessage?.sender?.userId || 'local',
      currentUser: currentUser.userId,
      lastUserMessageId: lastUserMessage?.id,
      shouldSync
    });
    
    if (!shouldSync) {
      console.log('🤖 Skipping Goose response sync - not responding to current user\'s question');
      return;
    }
    
    console.log('🤖 ✅ Goose response should sync - responding to current user\'s question');
    
    // Get the message content
    const messageContent = Array.isArray(lastMessage.content) 
      ? lastMessage.content.map(c => c.type === 'text' ? c.text : '').join('')
      : '';
    
    console.log('🤖 Processing Goose response for sync:', {
      messageId: lastMessage.id,
      contentLength: messageContent.length,
      content: messageContent.substring(0, 100) + '...',
      hasExistingTimeout: !!syncTimeoutRef.current
    });
    
    // Clear any existing timeout to restart the debounce
    if (syncTimeoutRef.current) {
      console.log('🤖 Clearing existing sync timeout for message:', lastMessage.id);
      clearTimeout(syncTimeoutRef.current);
    }
    
    // Set up simple debounced sync - wait for message to stabilize
    syncTimeoutRef.current = setTimeout(async () => {
      console.log('🤖 Debounce timeout triggered, attempting sync for message:', lastMessage.id);
      
      // Check if already synced (this should be the primary check)
      if (syncedMessageIds.has(lastMessage.id)) {
        console.log('🤖 Message already synced to Matrix, skipping:', lastMessage.id);
        syncTimeoutRef.current = null;
        return;
      }
      
      // Get the current content
      const currentContent = Array.isArray(lastMessage.content) 
        ? lastMessage.content.map(c => c.type === 'text' ? c.text : '').join('')
        : '';
      
      console.log('🤖 Final sync attempt for Goose response:', {
        messageId: lastMessage.id,
        contentLength: currentContent.length,
        content: currentContent.substring(0, 100) + '...'
      });
      
      // Mark as synced BEFORE attempting to send to prevent duplicate attempts
      setSyncedMessageIds(prev => new Set(prev).add(lastMessage.id));
      
      try {
        // Send as a Goose message (this will make Goose appear as a separate user)
        await sendGooseMessage(matrixRoomId, currentContent, 'goose.chat', {
          metadata: {
            originalMessageId: lastMessage.id,
            timestamp: lastMessage.created,
            isGooseResponse: true,
            respondingToUser: currentUser.userId,
            respondingToDisplayName: currentUser.displayName || currentUser.userId,
            inResponseToMessageId: lastUserMessage?.id,
          }
        });
        
        console.log('✅ Goose response synced to Matrix as separate user successfully:', {
          messageId: lastMessage.id,
          contentLength: currentContent.length
        });
        
      } catch (error) {
        console.error('❌ Failed to sync Goose response to Matrix as separate user:', error);
        console.error('❌ Error details:', {
          errorMessage: error instanceof Error ? error.message : String(error),
          matrixRoomId,
          messageLength: currentContent.length,
          currentUser: currentUser.userId
        });
        
        // Remove from synced set since sync failed
        setSyncedMessageIds(prev => {
          const newSet = new Set(prev);
          newSet.delete(lastMessage.id);
          return newSet;
        });
        
        // Fallback to regular message
        try {
          console.log('🔄 Attempting fallback sync as regular message...');
          await sendMessage(matrixRoomId, currentContent);
          console.log('✅ Goose response synced to Matrix as regular message (fallback)');
          // Re-mark as synced since fallback succeeded
          setSyncedMessageIds(prev => new Set(prev).add(lastMessage.id));
        } catch (fallbackError) {
          console.error('❌ Failed to sync Goose response as fallback:', fallbackError);
        }
      }
      
      syncTimeoutRef.current = null;
    }, 1000); // Reduced to 1 second for faster response
    
    // Cleanup timeout on unmount
    return () => {
      if (syncTimeoutRef.current) {
        clearTimeout(syncTimeoutRef.current);
        syncTimeoutRef.current = null;
      }
    };
  }, [chat.messages, isMatrixMode, matrixRoomId, currentUser, sendMessage, sendGooseMessage, syncedMessageIds]);

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
  console.log('🎯 Last 3 message IDs:', chat.messages?.slice(-3).map(m => ({ id: m.id, content: m.content?.[0]?.text?.substring(0, 20) + '...' })));
  console.log('🎯 Is Matrix mode:', isMatrixMode);
  console.log('🎯 Loading states:', { loadingChat, isLoadingMatrixHistory });

  // Add diagnostic functions for debugging blank chat issues
  useEffect(() => {
    if (!isMatrixMode || !matrixRoomId) return;

    // Create diagnostic functions that can be called from the browser console
    (window as any).debugBlankChat = () => {
      console.log('🔍 BLANK CHAT DIAGNOSTIC:');
      console.log('🔍 Loading states:', {
        loadingChat,
        isLoadingMatrixHistory,
        hasLoadedMatrixHistory,
        hasInitializedChat
      });
      console.log('🔍 Matrix connection:', { isConnected, isReady });
      console.log('🔍 Chat state:', {
        sessionId: chat.sessionId,
        messagesCount: chat.messages?.length || 0,
        messages: chat.messages?.slice(-3).map(m => ({
          id: m.id,
          role: m.role,
          content: Array.isArray(m.content) ? m.content[0]?.text?.substring(0, 30) + '...' : 'N/A'
        }))
      });
      console.log('🔍 Matrix room:', {
        matrixRoomId,
        effectiveSessionId,
        isMatrixMode
      });
      console.log('🔍 Agent state:', agentState);
      
      // Check what BaseChat will render
      const shouldShowMessages = chat.messages?.length > 0;
      const shouldShowPopularTopics = !chat.recipeConfig && true; // showPopularTopics is true in Pair
      console.log('🔍 Rendering logic:', {
        shouldShowMessages,
        shouldShowPopularTopics,
        hasRecipeConfig: !!chat.recipeConfig,
        willRenderMessages: !loadingChat && shouldShowMessages,
        willRenderPopularTopics: !loadingChat && !shouldShowMessages && shouldShowPopularTopics,
        willRenderNothing: loadingChat || (!shouldShowMessages && !shouldShowPopularTopics)
      });
    };

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
      delete (window as any).debugBlankChat;
      delete (window as any).testMatrixListeners;
    };
  }, [isMatrixMode, matrixRoomId, isConnected, isReady, effectiveSessionId, onMessage, onSessionMessage, chat.messages.length, loadingChat, isLoadingMatrixHistory, hasLoadedMatrixHistory, hasInitializedChat, agentState]);

  // Create custom chat input props for Matrix mode
  const matrixChatInputProps = useMemo(() => {
    if (isMatrixMode && matrixRoomId) {
      // Get the proper Goose session ID for backend calls
      const gooseSessionId = sessionMappingService.getGooseSessionId(matrixRoomId);
      
      if (gooseSessionId) {
        console.log('🔧 ChatInput: Using mapped Goose session ID for backend calls:', { matrixRoomId, gooseSessionId });
        return {
          ...customChatInputProps,
          // Use the proper Goose session ID for ChatInput (backend calls)
          sessionId: gooseSessionId,
        };
      } else {
        console.log('🔧 ChatInput: No mapping found, creating session mapping for Matrix room:', matrixRoomId);
        
        // Create a mapping for this Matrix room if it doesn't exist
        // This will create a backend session that can handle the chat
        const currentRoom = rooms.find(room => room.roomId === matrixRoomId);
        const roomName = currentRoom?.name || `Matrix Room ${matrixRoomId.substring(1, 8)}`;
        
        // Create the mapping asynchronously and use Matrix room ID as fallback for now
        sessionMappingService.createMappingWithBackendSession(matrixRoomId, [], roomName)
          .then(mapping => {
            console.log('✅ Created backend session mapping for Matrix room:', mapping);
            // Force a re-render to pick up the new mapping
            setChat(prevChat => ({ ...prevChat }));
          })
          .catch(error => {
            console.error('❌ Failed to create backend session mapping:', error);
          });
        
        // For now, return without sessionId to prevent backend calls until mapping is created
        console.log('🔧 ChatInput: Temporarily skipping sessionId until mapping is created');
        return {
          ...customChatInputProps,
          // Don't pass sessionId until we have a proper mapping
          // This will prevent backend calls from failing with Matrix room ID
        };
      }
    }
    return customChatInputProps;
  }, [customChatInputProps, isMatrixMode, matrixRoomId, rooms]);

  return (
    <BaseChat
      chat={chat} // Keep original chat with backend session ID
      loadingChat={loadingChat || isLoadingMatrixHistory} // Include Matrix history loading
      autoSubmit={shouldAutoSubmit}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      onMessageSubmit={handleMessageSubmit}
      customChatInputProps={matrixChatInputProps}
      contentClassName={cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
      showPopularTopics={!isTransitioningFromHub} // Show popular topics in all modes, including Matrix
      suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
      showParticipantsBar={isMatrixMode} // Show participants bar when in Matrix mode
      matrixRoomId={matrixRoomId || undefined} // Pass the Matrix room ID
    />
  );
}
