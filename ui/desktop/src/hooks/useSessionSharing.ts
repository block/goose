import { useState, useEffect, useCallback, useRef } from 'react';
import { useMatrix } from '../contexts/MatrixContext';
import { MatrixUser } from '../services/MatrixService';
import { Message } from '../types/message';

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
  sessionId: string;
  sessionTitle: string;
  messages: Message[];
  onMessageSync?: (message: Message) => void;
  onParticipantJoin?: (participant: SessionParticipant) => void;
  onParticipantLeave?: (userId: string) => void;
}

export const useSessionSharing = ({
  sessionId,
  sessionTitle,
  messages,
  onMessageSync,
  onParticipantJoin,
  onParticipantLeave,
}: UseSessionSharingProps) => {
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

  const [state, setState] = useState<SharedSessionState>({
    isShared: false,
    sessionId,
    participants: [],
    isHost: false,
    roomId: null,
    pendingInvitations: [],
    error: null,
  });

  // Listen for session-related Matrix messages
  useEffect(() => {
    if (!isConnected) return;

    const handleSessionMessage = (data: any) => {
      const { content, sender, roomId } = data;
      
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
          
          // In Matrix collaboration, we want to process all session messages from the room
          // regardless of session ID, since different users have different local session IDs
          const isMatrixRoom = sessionId.startsWith('!'); // Matrix room IDs start with !
          const shouldProcessMessage = isMatrixRoom || messageData.sessionId === sessionId;
          
          console.log('🔍 Session message processing check:', {
            messageSessionId: messageData.sessionId,
            currentSessionId: sessionId,
            isMatrixRoom,
            shouldProcessMessage,
            roomId
          });
          
          if (shouldProcessMessage) {
            // Convert to local message format
            const message: Message = {
              id: `shared-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
              role: messageData.role,
              created: Math.floor(Date.now() / 1000),
              content: [{
                type: 'text',
                text: messageData.content,
              }],
            };
            
            console.log('💬 Syncing message to local session:', message);
            onMessageSync?.(message);
          } else {
            console.log('🚫 Skipping session message due to session ID mismatch');
          }
        } catch (error) {
          console.error('Failed to parse session message:', error);
        }
      }
    };

    // Also listen for regular messages that might contain session data
    const handleRegularMessage = (data: any) => {
      const { content, sender, roomId, senderInfo } = data;
      
      // Only log debug info for messages that might be processed (reduce noise)
      if (state.roomId && roomId === state.roomId && sender !== currentUser?.userId) {
        console.log('🔍 Processing message in session room:', { 
          content: content?.substring(0, 50) + '...', 
          sender, 
          roomId
        });
      }
      
      // Only process messages from Matrix rooms that are part of our session
      if (state.roomId && roomId === state.roomId && sender !== currentUser?.userId) {
        console.log('💬 Regular message in session room:', { content, sender, roomId, senderInfo });
        
        // Find sender info from friends or participants
        let senderData = senderInfo;
        if (!senderData) {
          // Try to find sender in friends list
          const friend = friends.find(f => f.userId === sender);
          if (friend) {
            senderData = {
              userId: friend.userId,
              displayName: friend.displayName,
              avatarUrl: friend.avatarUrl,
            };
          } else {
            // Try to find in participants
            const participant = state.participants.find(p => p.userId === sender);
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
        
        // Convert regular Matrix messages to Goose messages with sender info
        const message: Message = {
          id: `matrix-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          role: 'user',
          created: Math.floor(Date.now() / 1000),
          content: [{
            type: 'text',
            text: content,
          }],
          sender: senderData,
        };
        
        console.log('💬 Converting regular Matrix message to Goose message');
        onMessageSync?.(message);
      }
      // Removed debug logging for filtered messages to reduce console noise
    };

    const sessionCleanup = onSessionMessage(handleSessionMessage);
    const messageCleanup = onMessage(handleRegularMessage);
    const gooseSessionCleanup = onMessage('gooseSessionSync', handleRegularMessage);
    
    return () => {
      sessionCleanup();
      messageCleanup();
      gooseSessionCleanup();
    };
  }, [isConnected, sessionId, state.roomId, currentUser?.userId, onSessionMessage, onMessage, onMessageSync, onParticipantJoin]);

  // Invite a friend to the current session
  const inviteToSession = useCallback(async (friendUserId: string) => {
    console.log('🚀 Starting invitation process for:', friendUserId);
    console.log('📊 Current state:', { 
      isConnected, 
      currentUser: currentUser?.userId, 
      roomId: state.roomId,
      friends: friends.length 
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
  }, [currentUser, isConnected, state.roomId, sessionId, sessionTitle, createAISession, sendMessage, inviteToRoom, sendCollaborationInvite, friends.length]);

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
    return friends.filter(friend => !participantIds.has(friend.userId));
  }, [friends, state.participants]);

  return {
    // State
    isShared: state.isShared,
    isSessionActive: state.isShared, // Add this for ChatInput compatibility
    participants: state.participants,
    isHost: state.isHost,
    pendingInvitations: state.pendingInvitations,
    error: state.error,
    canInvite: isConnected && !!currentUser && friends.length > 0,

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
