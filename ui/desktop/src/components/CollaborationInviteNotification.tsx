import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Users, X, Check, Clock } from 'lucide-react';
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
    acceptCollaborationInvite,
    declineCollaborationInvite 
  } = useMatrix();
  
  const [pendingInvites, setPendingInvites] = useState<GooseChatMessage[]>([]);
  const [dismissedInvites, setDismissedInvites] = useState<Set<string>>(new Set());

  // Listen for collaboration invites
  useEffect(() => {
    if (!isConnected) return;

    const unsubscribe = onGooseMessage((message: GooseChatMessage) => {
      // Only show collaboration invites that are not from self
      if (message.type === 'goose.collaboration.invite' && !message.metadata?.isFromSelf) {
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
    });

    return unsubscribe;
  }, [isConnected, onGooseMessage, dismissedInvites]);

  const handleAcceptInvite = async (invite: GooseChatMessage) => {
    try {
      console.log('🤝 Accepting collaboration invite from notification:', invite);
      
      // Extract session details from the message metadata
      const sessionData = invite.metadata;
      if (!sessionData?.sessionId || !sessionData?.roomId) {
        console.error('❌ Invalid collaboration invite - missing session data');
        return;
      }

      // Accept the collaboration invite via Matrix
      await acceptCollaborationInvite(invite.roomId, invite.messageId, ['ai-chat', 'collaboration']);
      
      // Remove from pending invites
      setPendingInvites(prev => prev.filter(i => i.messageId !== invite.messageId));
      
      console.log('✅ Accepted collaboration invite:', {
        sessionId: sessionData.sessionId,
        sessionTitle: sessionData.sessionTitle,
        roomId: sessionData.roomId,
      });
      
      // Show success notification
      alert(`Joined collaborative session: ${sessionData.sessionTitle}\n\nYou can now collaborate in real-time!`);
      
    } catch (error) {
      console.error('❌ Failed to accept collaboration invite:', error);
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
                    Collaboration Invite
                  </h4>
                  <button
                    onClick={() => handleDismissInvite(invite)}
                    className="text-gray-400 hover:text-gray-600 transition-colors"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
                
                <p className="text-sm text-gray-600 mb-2">
                  <span className="font-medium">{getSenderName(invite)}</span> invited you to collaborate
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
