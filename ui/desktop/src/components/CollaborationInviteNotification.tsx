import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Users, X, Check, Clock } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useMatrix } from '../contexts/MatrixContext';
import { GooseChatMessage } from '../services/MatrixService';

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
  const [pendingInvites, setPendingInvites] = useState<GooseChatMessage[]>([]);
  const [dismissedInvites, setDismissedInvites] = useState<Set<string>>(new Set());

  // Listen for Goose messages that could be collaboration opportunities
  useEffect(() => {
    if (!isConnected) return;

    const unsubscribe = onGooseMessage((message: GooseChatMessage) => {
      // Only show messages that are not from self
      if (message.metadata?.isFromSelf) {
        console.log('💬 Ignoring message from self');
        return;
      }

      // Handle explicit collaboration invites
      if (message.type === 'goose.collaboration.invite') {
        console.log('🔔 Received collaboration invite notification:', message);
        
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

    return unsubscribe;
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
        
        // Show success notification (but don't block navigation)
        setTimeout(() => {
          alert(`Joined collaborative session: ${sessionData.sessionTitle}\n\nYou can now collaborate in real-time!`);
        }, 500);
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
            className="bg-white border border-blue-200 rounded-lg shadow-lg p-4 max-w-sm"
          >
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center">
                <Users className="w-5 h-5 text-blue-600" />
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1">
                  <h4 className="text-sm font-semibold text-gray-900">
                    {invite.type === 'goose.collaboration.invite' ? 'Collaboration Invite' : 'New Message'}
                  </h4>
                  <button
                    onClick={() => handleDismissInvite(invite)}
                    className="text-gray-400 hover:text-gray-600 transition-colors"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
                
                <p className="text-sm text-gray-600 mb-2">
                  <span className="font-medium">{getSenderName(invite)}</span> {
                    invite.type === 'goose.collaboration.invite' 
                      ? 'invited you to collaborate' 
                      : 'sent you a message'
                  }
                </p>
                
                <p className="text-xs text-gray-500 mb-3 line-clamp-2">
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
                    className="flex items-center gap-1 px-3 py-1.5 border border-gray-300 text-gray-700 text-xs rounded hover:bg-gray-50 transition-colors"
                  >
                    <X className="w-3 h-3" />
                    Decline
                  </button>
                </div>
                
                <div className="flex items-center gap-1 mt-2 text-xs text-gray-400">
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
