import { useState, useEffect, useCallback, useRef } from 'react';
import { useMatrix } from '../contexts/MatrixContext';
import { MatrixUser } from '../services/MatrixService';
import { Message } from '../types/message';

// Force rebuild timestamp: 2025-01-15T01:00:00Z - Fixed friends.length and useEffect dependency array

interface SessionParticipant extends MatrixUser {
  joinedAt: Date;
  isTyping?: boolean;
  lastActive?: Date;
}

interface SessionInvitation {
  sessionId: string;
  sessionTitle: string;
  inviterUserId: string;
  inviterName: string;
  timestamp: Date;
  roomId?: string; // Matrix room for this shared session
}

interface SharedSessionState {
  isShared: boolean;
  sessionId: string;
  participants: SessionParticipant[];
  isHost: boolean;
  roomId: string | null;
  pendingInvitations: SessionInvitation[];
  error: string | null;
}

interface UseSessionSharingProps {
  sessionId: string | null; // Allow null to disable session sharing
  sessionTitle: string;
  messages: Message[];
  onMessageSync?: (message: Message) => void;
  onParticipantJoin?: (participant: SessionParticipant) => void;
  onParticipantLeave?: (userId: string) => void;
  initialRoomId?: string; // Matrix room ID to listen to for real-time messages
}

export const useSessionSharing = ({
  sessionId,
  sessionTitle,
  messages,
  onMessageSync,
  onParticipantJoin,
  onParticipantLeave,
  initialRoomId,
}: UseSessionSharingProps) => {
  // Always call hooks first - no conditional returns before hooks!
  const { 
    currentUser, 
    friends, 
    createAISession, 
    sendMessage,
    inviteToRoom,
    onMessage,
    onAIMessage,
    onSessionMessage,
    sendCollaborationInvite,
    isConnected 
  } = useMatrix();

  // Check if this is a Matrix room (before using in useState)
  const isMatrixRoom = sessionId && sessionId.startsWith('!');
  
  const [state, setState] = useState<SharedSessionState>({
    isShared: !!initialRoomId || isMatrixRoom, // If we have an initial room ID or it's a Matrix room, we're in a shared session
    sessionId,
    participants: [],
    isHost: false,
    roomId: initialRoomId || (isMatrixRoom ? sessionId : null), // Use sessionId as roomId for Matrix rooms
    pendingInvitations: [],
    error: null,
  });
  
  // Update state when initialRoomId changes (for Matrix mode)
  useEffect(() => {
    if (initialRoomId) {
      console.log('🏠 useSessionSharing: Updating room ID from initialRoomId:', initialRoomId);
      setState(prev => ({
        ...prev,
        roomId: initialRoomId,
        isShared: true, // Mark as shared when we have a room ID
        isHost: false, // In Matrix mode, we're joining an existing session
      }));
    }
  }, [initialRoomId]); // Only depend on initialRoomId
  
  // Log initial state setup for debugging
  useEffect(() => {
    console.log('🔧 useSessionSharing: Initial state setup:', {
      sessionId,
      initialRoomId,
      isShared: !!initialRoomId,
      roomId: initialRoomId || null
    });
  }, [sessionId, initialRoomId]);

  // Use refs to avoid stale closures in event handlers
  const stateRef = useRef(state);
  const friendsRef = useRef(friends);
  const currentUserRef = useRef(currentUser);
  
  // Update refs when values change
  useEffect(() => {
    stateRef.current = state;
  }, [state]);
  
  useEffect(() => {
    friendsRef.current = friends;
  }, [friends]);
  
  useEffect(() => {
    currentUserRef.current = currentUser;
  }, [currentUser]);

  // Track processed messages to prevent duplicates - ALWAYS call this hook
  const processedMessages = useRef<Set<string>>(new Set());

  // Helper function to create a deduplication key based on content and timestamp
  const createMessageKey = (content: string, sender: string, timestamp?: number) => {
    const time = timestamp || Date.now();
    // Round timestamp to nearest second to catch near-simultaneous duplicates
    const roundedTime = Math.floor(time / 1000);
    // Use first 50 chars of content + sender + rounded timestamp
    return `${sender}-${roundedTime}-${content.substring(0, 50)}`;
  };

  // Listen for session-related Matrix messages
  useEffect(() => {
    if (!isConnected) {
      console.log('🔌 useSessionSharing: Not connected, skipping listener setup');
      return;
    }

    console.log('🔧 useSessionSharing: Setting up Matrix message listeners for session:', sessionId);
    console.log('🔧 useSessionSharing: Current state when setting up listeners:', {
      sessionId,
      roomId: stateRef.current.roomId,
      isShared: stateRef.current.isShared,
      participantsCount: stateRef.current.participants.length,
      onMessageSyncAvailable: !!onMessageSync
    });
    
    // Debug: Test the onMessageSync callback immediately
    if (onMessageSync) {
      console.log('🔧 useSessionSharing: Testing onMessageSync callback...');
      // Don't actually call it, just confirm it exists
      console.log('🔧 useSessionSharing: onMessageSync callback is available and callable');
    } else {
      console.warn('⚠️ useSessionSharing: onMessageSync callback is NOT available!');
    }

    const handleSessionMessage = (data: any) => {
      const { content, sender, roomId, senderInfo } = data;
      
      // Only log session messages that aren't the repetitive goose-session-message ones
      if (!content.includes('goose-session-message:')) {
        console.log('📨 Received session message:', { content: content?.substring(0, 50) + '...', sender, roomId });
      }
      
      // Handle session invitation messages
      if (content.includes('goose-session-invite:')) {
        try {
          const inviteData = JSON.parse(content.split('goose-session-invite:')[1]);
          const invitation: SessionInvitation = {
            sessionId: inviteData.sessionId,
            sessionTitle: inviteData.sessionTitle,
            inviterUserId: sender,
            inviterName: inviteData.inviterName,
            timestamp: new Date(),
            roomId: inviteData.roomId,
          };
          
          console.log('📧 Parsed invitation:', invitation);
          
          setState(prev => ({
            ...prev,
            pendingInvitations: [...prev.pendingInvitations, invitation],
          }));
        } catch (error) {
          console.error('Failed to parse session invitation:', error);
        }
      }
      
      // Handle session join confirmations
      if (content.includes('goose-session-joined:')) {
        try {
          const joinData = JSON.parse(content.split('goose-session-joined:')[1]);
          if (joinData.sessionId === sessionId) {
            const participant: SessionParticipant = {
              userId: sender,
              displayName: joinData.participantName,
              joinedAt: new Date(),
            };
            
            console.log('👥 Participant joined:', participant);
            
            setState(prev => ({
              ...prev,
              participants: [...prev.participants, participant],
            }));
            
            onParticipantJoin?.(participant);
          }
        } catch (error) {
          console.error('Failed to parse session join:', error);
        }
      }
      
      // Handle session messages (AI prompts/responses)
      if (content.includes('goose-session-message:')) {
        try {
          const messageData = JSON.parse(content.split('goose-session-message:')[1]);
          
          // In Matrix collaboration, we want to process session messages from the current room only
          // We check both session ID match AND room ID match to avoid cross-contamination
          const isMatrixRoom = sessionId && sessionId.startsWith('!'); // Matrix room IDs start with !
          const isFromCurrentRoom = !roomId || roomId === sessionId; // Either no roomId filter or matches current room
          const isSessionMatch = messageData.sessionId === sessionId;
          
          // For Matrix rooms, prioritize room ID matching over session ID matching
          // This handles cases where messages have regular session IDs but are sent in Matrix rooms
          const shouldProcessMessage = isMatrixRoom ? isFromCurrentRoom : (isSessionMatch || isFromCurrentRoom);
          
          console.log('🔍 Session message processing check:', {
            messageSessionId: messageData.sessionId,
            currentSessionId: sessionId,
            messageRoomId: roomId,
            isMatrixRoom,
            isFromCurrentRoom,
            isSessionMatch,
            shouldProcessMessage,
            sender,
            messageRole: messageData.role,
            messageContent: messageData.content?.substring(0, 50) + '...'
          });
          
          if (shouldProcessMessage) {
            // Get sender information for proper attribution
            let senderData = senderInfo;
            if (!senderData && sender) {
              // Try to find sender in friends list
              const friend = friendsRef.current.find(f => f.userId === sender);
              if (friend) {
                senderData = {
                  userId: friend.userId,
                  displayName: friend.displayName,
                  avatarUrl: friend.avatarUrl,
                };
              } else {
                // Fallback to basic sender info from Matrix ID
                senderData = {
                  userId: sender,
                  displayName: sender.split(':')[0].substring(1), // Extract username from Matrix ID
                };
              }
            }
            
            // Enhanced role detection for session messages
            let finalRole = messageData.role as 'user' | 'assistant';
            
            // If the role is 'assistant', double-check that it's actually from a Goose instance
            if (finalRole === 'assistant') {
              const isFromGoose = senderData?.displayName?.toLowerCase().includes('goose') ||
                                senderData?.userId?.toLowerCase().includes('goose') ||
                                messageData.content?.includes('🦆') ||
                                messageData.content?.includes('🤖');
              
              if (!isFromGoose) {
                console.log('🔍 Role correction: Message marked as assistant but not from Goose, changing to user');
                finalRole = 'user';
              }
            }
            
            // If the role is 'user' but content looks like a Goose response, correct it
            if (finalRole === 'user') {
              const looksLikeGooseResponse = messageData.content && (
                messageData.content.includes('🦆') ||
                messageData.content.includes('🤖') ||
                messageData.content.startsWith('I\'m') ||
                messageData.content.includes('I can help') ||
                messageData.content.includes('Let me') ||
                (messageData.content.length > 100 && messageData.content.includes('\n\n')) ||
                /```[\s\S]*```/.test(messageData.content) // Contains code blocks
              );
              
              const isFromGoose = senderData?.displayName?.toLowerCase().includes('goose') ||
                                senderData?.userId?.toLowerCase().includes('goose');
              
              if (looksLikeGooseResponse || isFromGoose) {
                console.log('🔍 Role correction: Message marked as user but looks like Goose response, changing to assistant');
                finalRole = 'assistant';
              }
            }
            
            // Convert to local message format with proper sender attribution
            const message: Message = {
              id: `shared-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              role: finalRole,
              created: Math.floor(Date.now() / 1000),
              content: [{
                type: 'text',
                text: messageData.content,
              }],
              sender: senderData, // Include sender information
              metadata: {
                originalRole: messageData.role,
                correctedRole: finalRole,
                isFromMatrix: true,
                skipLocalResponse: true, // Prevent triggering local AI response
                preventAutoResponse: true,
                isFromCollaborator: true,
                sessionMessageId: messageData.sessionId
              }
            };
            
            console.log('💬 Syncing session message to local session:', {
              messageId: message.id,
              originalRole: messageData.role,
              finalRole: finalRole,
              sender: senderData?.displayName || senderData?.userId,
              content: messageData.content?.substring(0, 50) + '...'
            });
            
            onMessageSync?.(message);
          } else {
            console.log('🚫 Skipping session message - not from current room/session');
          }
        } catch (error) {
          console.error('Failed to parse session message:', error);
          console.error('Raw content that failed to parse:', content);
        }
      }
    };

    // Also listen for regular messages that might contain session data
    const handleRegularMessage = (data: any) => {
      const { content, sender, roomId, senderInfo, timestamp, event } = data;
      
      const currentState = stateRef.current;
      const currentUserFromRef = currentUserRef.current;
      
      // Simple deduplication: check if we've seen this exact message content at this time
      const messageKey = createMessageKey(content || '', sender, timestamp?.getTime?.());
      if (processedMessages.current.has(messageKey)) {
        console.log('🚫 Skipping duplicate regular message - same content and time:', messageKey);
        return;
      }
      
      // Debug logging for all incoming messages to understand the flow
      console.log('🔍 handleRegularMessage called:', { 
        messageKey,
        content: content?.substring(0, 50) + '...', 
        sender, 
        roomId,
        currentRoomId: currentState.roomId,
        isFromSelf: sender === currentUserFromRef?.userId,
        sessionId
      });
      
      // Only process messages from Matrix rooms that are part of our session
      if (currentState.roomId && roomId === currentState.roomId && sender !== currentUserFromRef?.userId) {
        console.log('💬 Processing message in session room:', { messageKey, content, sender, roomId, senderInfo });
        
        // Skip if this is a goose-session-message (should be handled by handleGooseSessionSync)
        if (content && content.includes('goose-session-message:')) {
          console.log('🚫 Skipping handleRegularMessage - this is a session message, will be handled by handleGooseSessionSync');
          return;
        }
        
        // Mark this message as processed
        processedMessages.current.add(messageKey);
        
        // Find sender info from friends or participants
        let senderData = senderInfo;
        if (!senderData) {
          // Try to find sender in friends list
          const friend = friendsRef.current.find(f => f.userId === sender);
          if (friend) {
            senderData = {
              userId: friend.userId,
              displayName: friend.displayName,
              avatarUrl: friend.avatarUrl,
            };
          } else {
            // Try to find in participants
            const participant = currentState.participants.find(p => p.userId === sender);
            if (participant) {
              senderData = {
                userId: participant.userId,
                displayName: participant.displayName,
                avatarUrl: participant.avatarUrl,
              };
            } else {
              // Fallback to basic sender info
              senderData = {
                userId: sender,
                displayName: sender.split(':')[0].substring(1), // Extract username from Matrix ID
              };
            }
          }
        }
        
        // Determine the correct role based on message content and sender
        let messageRole: 'user' | 'assistant' = 'user'; // Default to user
        
        // Use heuristic detection for non-session messages
        const isGooseResponse = content && (
            // Direct Goose markers - these are the most reliable indicators
            content.includes('🦆 Goose:') ||
            content.startsWith('🦆 Goose:') ||
            content.includes('🤖') ||
            // AI assistant self-identification patterns
            /I'm\s+goose,?\s+an?\s+AI\s+(agent|assistant)/i.test(content) ||
            /created\s+by\s+Block/i.test(content) ||
            /I'm\s+an?\s+AI\s+(agent|assistant)/i.test(content) ||
            // Tool and capability mentions (very specific to AI assistants)
            /I\s+have\s+access\s+to\s+(several\s+)?tools/i.test(content) ||
            /I\s+can\s+(use|access)\s+(tools|extensions)/i.test(content) ||
            /using\s+the\s+tools\s+(and\s+extensions\s+)?available/i.test(content) ||
            // AI assistant help patterns (be more specific)
            /I'm\s+(here|ready|available)\s+(and\s+ready\s+)?to\s+help/i.test(content) ||
            /What\s+(would\s+you\s+like|can\s+I\s+help)\s+(me\s+to\s+)?(work\s+on|do|with)/i.test(content) ||
            /I\s+can\s+assist\s+you\s+with/i.test(content) ||
            // Markdown formatting patterns (common in AI responses)
            /^-\s+\*\*[^*]+\*\*/.test(content) || // Starts with "- **Something**"
            /\*\*[^*]+\*\*.*\*\*[^*]+\*\*/.test(content) || // Multiple bold sections
            // Code block patterns
            /```[\s\S]*```/.test(content) ||
            // Long structured responses (likely AI)
            (content.length > 200 && /\n\n/.test(content) && /^(I|Let|Here|To|The)/i.test(content)) ||
            // Check if sender info indicates it's a Goose instance
            (senderData?.displayName && senderData.displayName.toLowerCase().includes('goose')) ||
            (sender && sender.toLowerCase().includes('goose'))
          );
          
        if (isGooseResponse) {
          messageRole = 'assistant';
          console.log('🤖 Detected AI response from Matrix based on content patterns');
        } else {
          console.log('👤 Treating as user message from Matrix');
        }
        
        // Convert regular Matrix messages to Goose messages with sender info
        const message: Message = {
          id: `matrix-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          role: messageRole,
          created: Math.floor(Date.now() / 1000),
          content: [{
            type: 'text',
            text: content,
          }],
          sender: senderData,
        };
        
        console.log('💬 Converting Goose message to local message and syncing:', message);
        console.log('🔍 onMessageSync callback available?', !!onMessageSync);
        
        if (onMessageSync) {
          console.log('📤 Calling onMessageSync with message:', {
            messageId: message.id,
            role: message.role,
            sender: message.sender?.displayName || message.sender?.userId,
            contentPreview: message.content[0]?.text?.substring(0, 50) + '...'
          });
          onMessageSync(message);
        } else {
          console.warn('⚠️ onMessageSync callback is not available!');
        }
      } else {
        console.log('🚫 Skipping message - not from current session room or from self');
      }
    };

    const sessionCleanup = onSessionMessage(handleSessionMessage);
    
    // Re-enable regular message handler with proper filtering
    const messageCleanup = onMessage(handleRegularMessage);
    console.log('✅ ENABLED regular message handler with goose-session-message filtering');
    
    // Handle gooseSessionSync events separately to avoid duplication
    const handleGooseSessionSync = (data: any) => {
      const { content, sender, roomId, senderInfo } = data;
      
      const currentState = stateRef.current;
      const currentUserFromRef = currentUserRef.current;
      
      // Debug logging
      console.log('🔄 handleGooseSessionSync called:', { 
        content: content?.substring(0, 50) + '...', 
        sender, 
        roomId,
        currentRoomId: currentState.roomId,
        isFromSelf: sender === currentUserFromRef?.userId,
        sessionId
      });
      
      // Only process messages from Matrix rooms that are part of our session and not from self
      if (currentState.roomId && roomId === currentState.roomId && sender !== currentUserFromRef?.userId) {
        console.log('🔄 Processing gooseSessionSync message in session room:', { content, sender, roomId, senderInfo });
        
        // If this is a goose-session-message, process it here since handleSessionMessage isn't being called
        if (content && content.includes('goose-session-message:')) {
          console.log('🔄 Processing goose-session-message in gooseSessionSync handler');
          
          // Call the same logic as handleSessionMessage for session messages
          try {
            const messageData = JSON.parse(content.split('goose-session-message:')[1]);
            
            // Simple deduplication: check if we've seen this exact message content at this time
            const messageKey = createMessageKey(messageData.content || '', sender, messageData.timestamp);
            if (processedMessages.current.has(messageKey)) {
              console.log('🚫 Skipping duplicate message - same content and time:', messageKey);
              return;
            }
            
            // Mark this message as processed
            processedMessages.current.add(messageKey);
            
            // In Matrix collaboration, we want to process session messages from the current room only
            const isMatrixRoom = sessionId && sessionId.startsWith('!');
            const isFromCurrentRoom = !roomId || roomId === sessionId;
            const isSessionMatch = messageData.sessionId === sessionId;
            
            // For Matrix rooms, prioritize room ID matching over session ID matching
            const shouldProcessMessage = isMatrixRoom ? isFromCurrentRoom : (isSessionMatch || isFromCurrentRoom);
            
            console.log('🔍 Session message processing check (gooseSessionSync):', {
              messageSessionId: messageData.sessionId,
              currentSessionId: sessionId,
              messageRoomId: roomId,
              isMatrixRoom,
              isFromCurrentRoom,
              isSessionMatch,
              shouldProcessMessage,
              sender,
              messageRole: messageData.role,
              messageContent: messageData.content?.substring(0, 50) + '...'
            });
            
            if (shouldProcessMessage) {
              // Get sender information for proper attribution
              let senderData = senderInfo;
              if (!senderData && sender) {
                // Try to find sender in friends list
                const friend = friendsRef.current.find(f => f.userId === sender);
                if (friend) {
                  senderData = {
                    userId: friend.userId,
                    displayName: friend.displayName,
                    avatarUrl: friend.avatarUrl,
                  };
                } else {
                  // Fallback to basic sender info from Matrix ID
                  senderData = {
                    userId: sender,
                    displayName: sender.split(':')[0].substring(1), // Extract username from Matrix ID
                  };
                }
              }
              
              // Enhanced role detection for session messages
              let finalRole = messageData.role as 'user' | 'assistant';
              
              // If the role is 'assistant', double-check that it's actually from a Goose instance
              if (finalRole === 'assistant') {
                const isFromGoose = senderData?.displayName?.toLowerCase().includes('goose') ||
                                  senderData?.userId?.toLowerCase().includes('goose') ||
                                  messageData.content?.includes('🦆') ||
                                  messageData.content?.includes('🤖');
                
                if (!isFromGoose) {
                  console.log('🔍 Role correction: Message marked as assistant but not from Goose, changing to user');
                  finalRole = 'user';
                }
              }
              
              // If the role is 'user' but content looks like a Goose response, correct it
              if (finalRole === 'user') {
                const looksLikeGooseResponse = messageData.content && (
                  messageData.content.includes('🦆') ||
                  messageData.content.includes('🤖') ||
                  messageData.content.startsWith('I\'m') ||
                  messageData.content.includes('I can help') ||
                  messageData.content.includes('Let me') ||
                  (messageData.content.length > 100 && messageData.content.includes('\n\n')) ||
                  /```[\s\S]*```/.test(messageData.content) // Contains code blocks
                );
                
                const isFromGoose = senderData?.displayName?.toLowerCase().includes('goose') ||
                                  senderData?.userId?.toLowerCase().includes('goose');
                
                if (looksLikeGooseResponse || isFromGoose) {
                  console.log('🔍 Role correction: Message marked as user but looks like Goose response, changing to assistant');
                  finalRole = 'assistant';
                }
              }
              
              // Convert to local message format with proper sender attribution
              const message: Message = {
                id: `shared-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
                role: finalRole,
                created: Math.floor(Date.now() / 1000),
                content: [{
                  type: 'text',
                  text: messageData.content,
                }],
                sender: senderData, // Include sender information
                metadata: {
                  originalRole: messageData.role,
                  correctedRole: finalRole,
                  isFromMatrix: true,
                  skipLocalResponse: true, // Prevent triggering local AI response
                  preventAutoResponse: true,
                  isFromCollaborator: true,
                  sessionMessageId: messageData.sessionId
                }
              };
              
              console.log('💬 *** PROCESSING SESSION MESSAGE IN GOOSE SESSION SYNC ***:', {
                messageId: message.id,
                originalRole: messageData.role,
                finalRole: finalRole,
                sender: senderData?.displayName || senderData?.userId,
                content: messageData.content?.substring(0, 50) + '...'
              });
              
              console.log('💬 *** CALLING onMessageSync FROM GOOSE SESSION SYNC ***');
              onMessageSync?.(message);
            } else {
              console.log('🚫 Skipping session message - not from current room/session (gooseSessionSync)');
            }
          } catch (error) {
            console.error('Failed to parse session message in gooseSessionSync:', error);
            console.error('Raw content that failed to parse:', content);
          }
          
          return; // Exit early after processing session message
        }
        
        // For non-session messages, let the regular message handler process them
        console.log('🔄 Non-session message in gooseSessionSync - letting regular handler process');
      } else {
        console.log('🚫 Skipping gooseSessionSync - not from current session room or from self');
      }
    };
    
    const gooseSessionCleanup = onMessage('gooseSessionSync', handleGooseSessionSync);
    
    return () => {
      console.log('🔧 useSessionSharing: Cleaning up Matrix message listeners for session:', sessionId);
      sessionCleanup();
      messageCleanup();
      gooseSessionCleanup();
    };
  }, [isConnected, sessionId, currentUser?.userId || null]);
  
  // Separate effect to log room ID changes without recreating listeners
  useEffect(() => {
    console.log('🏠 useSessionSharing: Room ID changed to:', state.roomId);
  }, [state.roomId]);

  // Invite a friend to the current session
  const inviteToSession = useCallback(async (friendUserId: string) => {
    console.log('🚀 Starting invitation process for:', friendUserId);
    console.log('📊 Current state:', { 
      isConnected, 
      currentUser: currentUser?.userId, 
      roomId: state.roomId,
      friends: friends?.length || 0 
    });

    if (!currentUser || !isConnected) {
      const errorMsg = 'Not connected to Matrix or no current user';
      console.error('❌', errorMsg);
      setState(prev => ({ ...prev, error: errorMsg }));
      throw new Error(errorMsg);
    }

    try {
      // Clear any previous errors
      setState(prev => ({ ...prev, error: null }));

      // Create or get the Matrix room for this session if not exists
      let roomId = state.roomId;
      if (!roomId) {
        console.log('🏠 Creating new AI session room and inviting friend directly...');
        // Create the session room and invite the friend immediately
        roomId = await createAISession(`Shared Session: ${sessionTitle}`, [friendUserId]);
        console.log('✅ Created session room with friend invited:', roomId);
        
        setState(prev => ({ 
          ...prev, 
          roomId,
          isShared: true,
          isHost: true,
          participants: [{
            ...currentUser,
            joinedAt: new Date(),
          }],
        }));
      } else {
        console.log('🏠 Using existing room, inviting friend to session room:', roomId);
        // Invite the friend to the existing session room
        await inviteToRoom(roomId, friendUserId);
        console.log('✅ Invited friend to existing session room');
      }

      // Send a Goose collaboration invite instead of a simple welcome message
      console.log('📤 Sending Goose collaboration invite...');
      
      // Use the sendCollaborationInvite from the Matrix context
      if (sendCollaborationInvite) {
        await sendCollaborationInvite(
          roomId, 
          `Collaborative AI Session: ${sessionTitle}`,
          ['ai-chat', 'collaboration', 'real-time-sync'],
          {
            sessionId,
            sessionTitle,
            roomId,
            inviterName: currentUser.displayName || currentUser.userId,
            timestamp: new Date().toISOString(),
          }
        );
        console.log('✅ Sent structured Goose collaboration invite');
      } else {
        // Fallback to regular message if Goose communication not available
        const welcomeMessage = `🎉 ${currentUser.displayName || currentUser.userId} invited you to collaborate on: ${sessionTitle}`;
        await sendMessage(roomId, welcomeMessage);
        console.log('✅ Sent fallback welcome message');
      }

      console.log(`✅ Successfully invited ${friendUserId} to session room and sent invite`);
      
      // Show success feedback
      setState(prev => ({ 
        ...prev, 
        error: null // Clear any errors on success
      }));
      
    } catch (error) {
      console.error('❌ Failed to invite to session:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to send invitation';
      setState(prev => ({ 
        ...prev, 
        error: errorMessage
      }));
      throw error;
    }
  }, [currentUser, isConnected, state.roomId, sessionId, sessionTitle, createAISession, sendMessage, inviteToRoom, sendCollaborationInvite, friends?.length]);

  // Join a shared session
  const joinSession = useCallback(async (invitation: SessionInvitation) => {
    if (!currentUser || !invitation.roomId) {
      throw new Error('Invalid invitation or not connected');
    }

    try {
      // Join the Matrix room
      // Note: In a real implementation, you'd need to handle room joining
      // For now, we'll simulate joining
      
      setState(prev => ({
        ...prev,
        isShared: true,
        sessionId: invitation.sessionId,
        roomId: invitation.roomId,
        isHost: false,
        participants: [{
          ...currentUser,
          joinedAt: new Date(),
        }],
        pendingInvitations: prev.pendingInvitations.filter(inv => inv.sessionId !== invitation.sessionId),
      }));

      // Notify the session host that we joined
      const joinData = {
        sessionId: invitation.sessionId,
        participantName: currentUser.displayName || currentUser.userId,
      };

      await sendMessage(invitation.roomId, `goose-session-joined:${JSON.stringify(joinData)}`);
      
      console.log(`Joined session ${invitation.sessionId}`);
    } catch (error) {
      console.error('Failed to join session:', error);
      setState(prev => ({ 
        ...prev, 
        error: error instanceof Error ? error.message : 'Failed to join session' 
      }));
      throw error;
    }
  }, [currentUser, sendMessage]);

  // Leave the shared session
  const leaveSession = useCallback(() => {
    setState(prev => ({
      ...prev,
      isShared: false,
      participants: [],
      roomId: null,
      isHost: false,
    }));
  }, []);

  // Use refs to avoid infinite loops in useCallback dependencies
  const syncTimeoutsRef = useRef<Map<string, NodeJS.Timeout>>(new Map());
  const lastSyncedContentRef = useRef<Map<string, string>>(new Map());

  // Sync a message to all session participants (debounced to prevent streaming spam)
  const syncMessage = useCallback(async (message: Message | { id: string; role: string; content: string; timestamp: string }) => {
    if (!state.isShared || !state.roomId) return;

    try {
      let messageContent: string;
      let messageId: string;
      
      // Handle both Message type and simple message object
      if ('content' in message && Array.isArray(message.content)) {
        // Standard Message type
        messageContent = message.content.map(c => c.type === 'text' ? c.text : '').join('');
        messageId = message.id;
      } else if ('content' in message && typeof message.content === 'string') {
        // Simple message object from ChatInput
        messageContent = message.content;
        messageId = message.id;
      } else {
        console.error('Invalid message format for sync:', message);
        return;
      }

      // Skip if content hasn't changed (prevents duplicate syncing)
      const lastContent = lastSyncedContentRef.current.get(messageId);
      if (lastContent === messageContent) {
        return;
      }

      // Clear any existing timeout for this message
      const existingTimeout = syncTimeoutsRef.current.get(messageId);
      if (existingTimeout) {
        clearTimeout(existingTimeout);
      }

      // Set up debounced sync (wait 1 second after last update before syncing)
      const timeout = setTimeout(async () => {
        try {
          const messageData = {
            sessionId,
            role: message.role,
            content: messageContent,
            timestamp: Date.now(),
          };

          await sendMessage(state.roomId!, `goose-session-message:${JSON.stringify(messageData)}`);
          
          // Update last synced content
          lastSyncedContentRef.current.set(messageId, messageContent);
          
          // Clean up timeout
          syncTimeoutsRef.current.delete(messageId);
          
          console.log('✅ Message synced to collaborative session (final):', messageContent.substring(0, 50) + '...');
        } catch (error) {
          console.error('❌ Failed to sync message:', error);
        }
      }, 1000); // Wait 1 second after last update

      // Store the timeout
      syncTimeoutsRef.current.set(messageId, timeout);

    } catch (error) {
      console.error('❌ Failed to setup message sync:', error);
      setState(prev => ({ 
        ...prev, 
        error: error instanceof Error ? error.message : 'Failed to sync message' 
      }));
    }
  }, [state.isShared, state.roomId, sessionId, sendMessage]);

  // Decline a session invitation
  const declineInvitation = useCallback((invitation: SessionInvitation) => {
    setState(prev => ({
      ...prev,
      pendingInvitations: prev.pendingInvitations.filter(inv => inv.sessionId !== invitation.sessionId),
    }));
  }, []);

  // Get available friends to invite (excluding current participants)
  const getAvailableFriends = useCallback(() => {
    const participantIds = new Set(state.participants.map(p => p.userId));
    return (friends || []).filter(friend => !participantIds.has(friend.userId));
  }, [friends, state.participants]);

  // Check if session sharing should be disabled AFTER all hooks are called
  // For Matrix rooms, we still want useSessionSharing to work for message processing,
  // but we disable the collaborative features (invitations, etc.)
  if (!sessionId) {
    console.log('🚫 useSessionSharing: Disabled (sessionId is null)');
    return {
      // State
      isShared: false,
      isSessionActive: false,
      participants: [],
      isHost: false,
      pendingInvitations: [],
      error: null,
      canInvite: false,

      // Actions (no-op functions)
      inviteToSession: async () => { throw new Error('Session sharing is disabled'); },
      joinSession: async () => { throw new Error('Session sharing is disabled'); },
      leaveSession: () => {},
      syncMessage: async () => {},
      declineInvitation: () => {},
      getAvailableFriends: () => [],
      
      // Utilities
      clearError: () => {},
    };
  }
  
  if (isMatrixRoom) {
    console.log('🔄 useSessionSharing: Matrix room mode - message processing enabled, collaboration features disabled');
  }

  return {
    // State
    isShared: state.isShared,
    isSessionActive: state.isShared, // Add this for ChatInput compatibility
    participants: state.participants,
    isHost: state.isHost,
    pendingInvitations: state.pendingInvitations,
    error: state.error,
    canInvite: isConnected && !!currentUser && (friends?.length || 0) > 0,

    // Actions
    inviteToSession,
    joinSession,
    leaveSession,
    syncMessage,
    declineInvitation,
    getAvailableFriends,
    
    // Utilities
    clearError: () => setState(prev => ({ ...prev, error: null })),
  };
};
