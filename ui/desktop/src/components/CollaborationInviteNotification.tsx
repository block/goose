import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Users, X, Check, Clock } from 'lucide-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useMatrix } from '../contexts/MatrixContext';
import { GooseChatMessage, matrixService } from '../services/MatrixService';
import { matrixInviteStateService } from '../services/MatrixInviteStateService';

interface CollaborationInviteNotificationProps {
  className?: string;
}

const CollaborationInviteNotification: React.FC<CollaborationInviteNotificationProps> = ({
  className = '',
}) => {
  const { 
    isConnected, 
    onGooseMessage,
    joinRoom,
    acceptCollaborationInvite,
    declineCollaborationInvite 
  } = useMatrix();
  
  const navigate = useNavigate();
  const location = useLocation();
  const [pendingInvites, setPendingInvites] = useState<GooseChatMessage[]>([]);
  const [dismissedInvites, setDismissedInvites] = useState<Set<string>>(new Set());

  // Helper function to get current active Matrix room ID if in shared session mode
  const getCurrentActiveMatrixRoom = () => {
    const searchParams = new URLSearchParams(location.search);
    const isMatrixMode = searchParams.get('matrixMode') === 'true';
    const matrixRoomId = searchParams.get('matrixRoomId');
    
    return isMatrixMode && matrixRoomId ? matrixRoomId : null;
  };

  // Listen for Matrix room invitations and Goose messages
  useEffect(() => {
    if (!isConnected) return;

    // Listen for clear notifications event
    const handleClearNotifications = () => {
      console.log('🧹 CollaborationInviteNotification: Clearing all pending invites due to clear event');
      setPendingInvites([]);
      setDismissedInvites(new Set());
    };

    window.addEventListener('clearNotifications', handleClearNotifications);

    // Handler for Matrix room invitations
    const handleMatrixRoomInvitation = (invitationData: any) => {
      console.log('🚨 FRONTEND: CollaborationInviteNotification received Matrix room invitation:', invitationData);
      
      // Skip notifications for the currently active Matrix room (shared session)
      const activeMatrixRoom = getCurrentActiveMatrixRoom();
      if (activeMatrixRoom && invitationData.roomId === activeMatrixRoom) {
        console.log('🔔 FRONTEND: Skipping Matrix invitation notification for active room:', invitationData.roomId);
        return;
      }

      // CRITICAL: Check if this invite should actually be shown according to MatrixInviteStateService
      const shouldShow = matrixInviteStateService.shouldShowInvite(invitationData.roomId, invitationData.inviter);
      if (!shouldShow) {
        console.log('🔔 FRONTEND: Skipping Matrix invitation notification - MatrixInviteStateService says not to show:', {
          roomId: invitationData.roomId,
          inviter: invitationData.inviter,
          shouldShow
        });
        return;
      }

      console.log('🚨 FRONTEND: Matrix invitation passed shouldShow check - ADDING TO UI:', invitationData.roomId);

      // Convert Matrix invitation to GooseChatMessage format for UI compatibility
      const collaborationInvite: GooseChatMessage = {
        type: 'goose.collaboration.invite',
        messageId: `matrix-invite-${invitationData.roomId}-${Date.now()}`,
        content: `${invitationData.inviterName} invited you to collaborate in a Matrix room`,
        sender: invitationData.inviter,
        timestamp: invitationData.timestamp,
        roomId: invitationData.roomId,
        metadata: {
          isFromSelf: false,
          invitationType: 'matrix_room',
          sessionId: `matrix-${invitationData.roomId}`,
          sessionTitle: `Matrix Collaboration with ${invitationData.inviterName}`,
          roomId: invitationData.roomId,
          inviterName: invitationData.inviterName,
        },
      };

      // Add to pending invites if not already dismissed
      setPendingInvites(prev => {
        const exists = prev.some(invite => invite.messageId === collaborationInvite.messageId);
        if (!exists && !dismissedInvites.has(collaborationInvite.messageId)) {
          return [...prev, collaborationInvite];
        }
        return prev;
      });
    };

    // Listen for direct Matrix room invitations (no duplicates)
    matrixService.on('matrixRoomInvitation', handleMatrixRoomInvitation);

    // Listen for Goose messages that could be collaboration opportunities
    const unsubscribeGooseMessages = onGooseMessage((message: GooseChatMessage) => {
      console.log('🚨 FRONTEND: CollaborationInviteNotification received Goose message:', message);
      
      // CRITICAL FIX: Skip collaboration response messages (accept/decline) - these are not new invitations!
      if (message.type === 'goose.collaboration.accept' || message.type === 'goose.collaboration.decline') {
        console.log('💬 FRONTEND: Ignoring collaboration response message (not a new invitation):', message.type);
        return;
      }
      
      // CRITICAL FIX: Ignore old messages from session history (older than 2 minutes)
      // This prevents spam from historical messages when the app loads
      const messageAge = Date.now() - message.timestamp.getTime();
      const twoMinutesInMs = 2 * 60 * 1000;
      
      if (messageAge > twoMinutesInMs) {
        console.log('💬 FRONTEND: Ignoring old message from session history:', {
          messageId: message.messageId,
          type: message.type,
          ageMinutes: Math.round(messageAge / 60000),
          timestamp: message.timestamp.toISOString()
        });
        return;
      }
      
      // Only show messages that are not from self
      const currentUserId = matrixService.client?.getUserId();
      const isFromSelf = message.metadata?.isFromSelf || (currentUserId && message.sender === currentUserId);
      
      if (isFromSelf) {
        console.log('💬 FRONTEND: Ignoring message from self:', {
          sender: message.sender,
          currentUserId,
          metadataIsFromSelf: message.metadata?.isFromSelf,
          calculatedIsFromSelf: isFromSelf
        });
        return;
      }

      // Skip notifications for messages from the currently active Matrix room (shared session)
      const activeMatrixRoom = getCurrentActiveMatrixRoom();
      if (activeMatrixRoom && message.roomId === activeMatrixRoom) {
        console.log('🔔 FRONTEND: Skipping collaboration notification for message from active Matrix room:', message.roomId);
        return;
      }

      // Handle explicit collaboration invites (but skip Matrix room invitations since they're handled above)
      if (message.type === 'goose.collaboration.invite' && message.metadata?.invitationType !== 'matrix_room') {
        console.log('🚨 FRONTEND: Received collaboration invite notification:', message);
        
        // CRITICAL: For goose.collaboration.invite messages, check if we're already joined to the Matrix room
        if (message.roomId && matrixService.client) {
          const room = matrixService.client.getRoom(message.roomId);
          const membership = room?.getMyMembership();
          
          if (membership === 'join') {
            console.log('🔔 FRONTEND: Skipping collaboration invite notification - already joined to room:', {
              roomId: message.roomId,
              membership,
              messageId: message.messageId
            });
            return;
          }
          
          console.log('🔔 FRONTEND: Collaboration invite for room we are not joined to:', {
            roomId: message.roomId,
            membership: membership || 'not found',
            messageId: message.messageId
          });
        }
        
        console.log('🚨 FRONTEND: Collaboration invite passed membership check - ADDING TO UI:', message);
        
        // Add to pending invites if not already dismissed
        setPendingInvites(prev => {
          const exists = prev.some(invite => invite.messageId === message.messageId);
          if (!exists && !dismissedInvites.has(message.messageId)) {
            return [...prev, message];
          }
          return prev;
        });
      }
      
      // Handle regular Goose chat messages as collaboration opportunities
      else if (message.type === 'goose.chat') {
        // Skip messages that contain session data (these are ongoing collaborations, not new invites)
        if (message.content.includes('goose-session-message:') || 
            message.content.includes('goose-session-invite:') ||
            message.content.includes('goose-session-joined:')) {
          console.log('💬 Ignoring session-related message (not a new collaboration invite)');
          return;
        }
        
        console.log('💬 Received Goose chat message - offering collaboration:', message);
        
        // Only show notifications for recent messages (within the last 5 minutes)
        // This prevents spam from old messages when first connecting
        const messageAge = Date.now() - message.timestamp.getTime();
        const fiveMinutesInMs = 5 * 60 * 1000;
        
        if (messageAge > fiveMinutesInMs) {
          console.log('💬 Ignoring old chat message for collaboration notification');
          return;
        }
        
        // Convert chat message to collaboration opportunity
        const collaborationOpportunity: GooseChatMessage = {
          ...message,
          type: 'goose.collaboration.chat', // New type for chat-based collaboration
          content: message.content, // Keep original message content
        };
        
        // Add to pending invites if not already dismissed
        setPendingInvites(prev => {
          const exists = prev.some(invite => invite.messageId === message.messageId);
          if (!exists && !dismissedInvites.has(message.messageId)) {
            return [...prev, collaborationOpportunity];
          }
          return prev;
        });
      }
    });

    return () => {
      // Remove Matrix room invitation listener
      matrixService.off('matrixRoomInvitation', handleMatrixRoomInvitation);
      // Remove Goose message listener
      unsubscribeGooseMessages();
      // Remove clear notifications listener
      window.removeEventListener('clearNotifications', handleClearNotifications);
    };
  }, [isConnected, onGooseMessage, dismissedInvites]);

  const handleAcceptInvite = async (invite: GooseChatMessage) => {
    try {
      console.log('🤝 Accepting collaboration from notification:', invite);
      
      // First, try to join the room
      console.log('🚪 Joining room:', invite.roomId);
      await joinRoom(invite.roomId);
      console.log('✅ Successfully joined room:', invite.roomId);
      
      // Handle explicit collaboration invites with session metadata
      if (invite.type === 'goose.collaboration.invite') {
        const sessionData = invite.metadata;
        if (!sessionData?.sessionId || !sessionData?.roomId) {
          console.error('❌ Invalid collaboration invite - missing session data');
          return;
        }

        // Accept the collaboration invite via Matrix
        await acceptCollaborationInvite(invite.roomId, invite.messageId, ['ai-chat', 'collaboration']);
        
        console.log('✅ Accepted collaboration invite:', {
          sessionId: sessionData.sessionId,
          sessionTitle: sessionData.sessionTitle,
          roomId: sessionData.roomId,
        });
        
        // Navigate to the pair view (regular chat session) with Matrix integration
        console.log('🧭 Navigating to pair view for Matrix collaboration:', invite.roomId);
        const searchParams = new URLSearchParams({
          matrixMode: 'true',
          matrixRoomId: invite.roomId,
          matrixRecipientId: invite.sender
        });
        navigate(`/pair?${searchParams.toString()}`);
        
        // Add a system message to show the collaboration was accepted
        // This will be visible in the chat interface
        console.log('✅ Collaboration invite accepted - system message will be visible in chat');
      }
      
      // Handle regular chat messages as collaboration opportunities
      else if (invite.type === 'goose.collaboration.chat') {
        console.log('💬 Joining chat room for collaboration:', invite.roomId);
        
        // For regular chat messages, we join the room and start collaborating
        // The room ID is where the original message came from
        const senderName = getSenderName(invite);
        
        // Send a collaboration acceptance message to the room
        await acceptCollaborationInvite(invite.roomId, invite.messageId, ['ai-chat', 'collaboration']);
        
        console.log('✅ Joined chat room for collaboration:', invite.roomId);
        
        // Navigate to the pair view (regular chat session) with Matrix integration
        console.log('🧭 Navigating to pair view for Matrix collaboration:', invite.roomId);
        const searchParams2 = new URLSearchParams({
          matrixMode: 'true',
          matrixRoomId: invite.roomId,
          matrixRecipientId: invite.sender
        });
        navigate(`/pair?${searchParams2.toString()}`);
        
        // Show success notification (but don't block navigation)
        setTimeout(() => {
          alert(`Started collaborating with ${senderName}!\n\nYou're now in their chat room and can work together.`);
        }, 500);
      }
      
      // Remove from pending invites
      setPendingInvites(prev => prev.filter(i => i.messageId !== invite.messageId));
      
    } catch (error) {
      console.error('❌ Failed to accept collaboration:', error);
      
      // Provide more helpful error messages
      let errorMessage = 'Failed to join collaboration. Please try again.';
      
      if (error instanceof Error) {
        if (error.message.includes('not invited') || error.message.includes('private')) {
          errorMessage = 'You are not invited to this room or it is private. Please ask the sender to invite you again.';
        } else if (error.message.includes('not found')) {
          errorMessage = 'The room could not be found. It may have been deleted.';
        } else if (error.message.includes('Network error')) {
          errorMessage = 'Network error. Please check your connection and try again.';
        } else {
          errorMessage = `Failed to join: ${error.message}`;
        }
      }
      
      alert(errorMessage);
    }
  };

  const handleDeclineInvite = async (invite: GooseChatMessage) => {
    try {
      console.log('❌ Declining collaboration invite:', invite);
      
      // Send decline message
      await declineCollaborationInvite(invite.roomId, invite.messageId);
      
      // Remove from pending invites
      setPendingInvites(prev => prev.filter(i => i.messageId !== invite.messageId));
      
      console.log('✅ Declined collaboration invite');
      
    } catch (error) {
      console.error('❌ Failed to decline collaboration invite:', error);
    }
  };

  const handleDismissInvite = (invite: GooseChatMessage) => {
    // Add to dismissed set and remove from pending
    setDismissedInvites(prev => new Set([...prev, invite.messageId]));
    setPendingInvites(prev => prev.filter(i => i.messageId !== invite.messageId));
  };

  const getSenderName = (invite: GooseChatMessage) => {
    // Extract sender name from user ID
    return invite.sender.split(':')[0].substring(1);
  };

  if (!isConnected || pendingInvites.length === 0) {
    return null;
  }

  return (
    <div className={`fixed top-4 right-4 z-50 space-y-2 ${className}`}>
      <AnimatePresence>
        {pendingInvites.map((invite) => (
          <motion.div
            key={invite.messageId}
            initial={{ opacity: 0, x: 300, scale: 0.8 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 300, scale: 0.8 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="bg-background-default border border-borderStandard rounded-lg shadow-lg p-4 max-w-sm"
          >
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-10 h-10 bg-bgSubtle rounded-full flex items-center justify-center">
                <Users className="w-5 h-5 text-accent" />
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1">
                  <h4 className="text-sm font-semibold text-textStandard">
                    {invite.type === 'goose.collaboration.invite' ? 'Collaboration Invite' : 'New Message'}
                  </h4>
                  <button
                    onClick={() => handleDismissInvite(invite)}
                    className="text-textSubtle hover:text-textStandard transition-colors"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
                
                <p className="text-sm text-textStandard mb-2">
                  <span className="font-medium">{getSenderName(invite)}</span> {
                    invite.type === 'goose.collaboration.invite' 
                      ? 'invited you to collaborate' 
                      : 'sent you a message'
                  }
                </p>
                
                <p className="text-xs text-textSubtle mb-3 line-clamp-2">
                  {invite.content}
                </p>
                
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleAcceptInvite(invite)}
                    className="flex items-center gap-1 px-3 py-1.5 bg-green-500 text-white text-xs rounded hover:bg-green-600 transition-colors"
                  >
                    <Check className="w-3 h-3" />
                    Join Session
                  </button>
                  
                  <button
                    onClick={() => handleDeclineInvite(invite)}
                    className="flex items-center gap-1 px-3 py-1.5 border border-borderStandard text-textStandard text-xs rounded hover:bg-bgSubtle transition-colors"
                  >
                    <X className="w-3 h-3" />
                    Decline
                  </button>
                </div>
                
                <div className="flex items-center gap-1 mt-2 text-xs text-textSubtle">
                  <Clock className="w-3 h-3" />
                  {invite.timestamp.toLocaleTimeString()}
                </div>
              </div>
            </div>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
};

export default CollaborationInviteNotification;
