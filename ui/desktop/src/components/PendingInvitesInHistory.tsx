import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Users, Clock, Check, X, UserPlus, MessageSquare } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useMatrix } from '../contexts/MatrixContext';
import { matrixInviteStateService, InviteState } from '../services/MatrixInviteStateService';
import { matrixService } from '../services/MatrixService';

interface PendingInvitesInHistoryProps {
  className?: string;
  showInChatHistory?: boolean; // If true, shows as chat messages, if false shows as separate section
}

const PendingInvitesInHistory: React.FC<PendingInvitesInHistoryProps> = ({
  className = '',
  showInChatHistory = false,
}) => {
  const { 
    isConnected, 
    joinRoom,
    acceptCollaborationInvite,
    declineCollaborationInvite 
  } = useMatrix();
  
  const navigate = useNavigate();
  const [pendingInvites, setPendingInvites] = useState<InviteState[]>([]);
  const [loading, setLoading] = useState(false);

  // Load pending invites
  useEffect(() => {
    if (!isConnected) return;

    const loadPendingInvites = () => {
      const invites = matrixInviteStateService.getPendingInvites();
      console.log('📋 PendingInvitesInHistory: Loaded pending invites:', invites.length);
      setPendingInvites(invites);
    };

    // Load initially
    loadPendingInvites();

    // Listen for invite state changes
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'goose-matrix-invite-states') {
        loadPendingInvites();
      }
    };

    window.addEventListener('storage', handleStorageChange);

    // Refresh every 30 seconds to catch any changes
    const interval = setInterval(loadPendingInvites, 30000);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      clearInterval(interval);
    };
  }, [isConnected]);

  const handleAcceptInvite = async (invite: InviteState) => {
    setLoading(true);
    try {
      console.log('🤝 Accepting invite from history:', invite);
      
      // Join the room
      await joinRoom(invite.roomId);
      
      // Mark as accepted
      matrixInviteStateService.acceptInvite(invite.roomId);
      
      // Navigate to the collaboration
      const searchParams = new URLSearchParams({
        matrixMode: 'true',
        matrixRoomId: invite.roomId,
        matrixRecipientId: invite.inviter
      });
      navigate(`/pair?${searchParams.toString()}`);
      
      // Remove from local state
      setPendingInvites(prev => prev.filter(i => i.roomId !== invite.roomId));
      
      console.log('✅ Successfully accepted invite from history');
      
    } catch (error) {
      console.error('❌ Failed to accept invite from history:', error);
      alert(`Failed to accept invite: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDeclineInvite = async (invite: InviteState) => {
    setLoading(true);
    try {
      console.log('❌ Declining invite from history:', invite);
      
      // Mark as declined
      matrixInviteStateService.declineInvite(invite.roomId);
      
      // Remove from local state
      setPendingInvites(prev => prev.filter(i => i.roomId !== invite.roomId));
      
      console.log('✅ Successfully declined invite from history');
      
    } catch (error) {
      console.error('❌ Failed to decline invite from history:', error);
    } finally {
      setLoading(false);
    }
  };

  const formatTimestamp = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const getSenderDisplayName = (invite: InviteState) => {
    return invite.inviterName || invite.inviter.split(':')[0].substring(1);
  };

  if (!isConnected || pendingInvites.length === 0) {
    return null;
  }

  // Chat history style (shows as messages in the chat)
  if (showInChatHistory) {
    return (
      <div className={`space-y-4 ${className}`}>
        {pendingInvites.map((invite) => (
          <motion.div
            key={invite.roomId}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex items-start gap-3 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800"
          >
            <div className="flex-shrink-0 w-10 h-10 bg-blue-100 dark:bg-blue-800 rounded-full flex items-center justify-center">
              <UserPlus className="w-5 h-5 text-blue-600 dark:text-blue-400" />
            </div>
            
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-2">
                <h4 className="text-sm font-semibold text-blue-900 dark:text-blue-100">
                  Collaboration Invite
                </h4>
                <span className="text-xs text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-800 px-2 py-1 rounded">
                  Pending
                </span>
              </div>
              
              <p className="text-sm text-blue-800 dark:text-blue-200 mb-2">
                <span className="font-medium">{getSenderDisplayName(invite)}</span> invited you to collaborate
              </p>
              
              <div className="flex items-center gap-2 text-xs text-blue-600 dark:text-blue-400 mb-3">
                <Clock className="w-3 h-3" />
                {formatTimestamp(invite.timestamp)}
              </div>
              
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleAcceptInvite(invite)}
                  disabled={loading}
                  className="flex items-center gap-1 px-3 py-1.5 bg-green-500 text-white text-xs rounded hover:bg-green-600 transition-colors disabled:opacity-50"
                >
                  <Check className="w-3 h-3" />
                  Accept & Join
                </button>
                
                <button
                  onClick={() => handleDeclineInvite(invite)}
                  disabled={loading}
                  className="flex items-center gap-1 px-3 py-1.5 border border-blue-300 dark:border-blue-600 text-blue-700 dark:text-blue-300 text-xs rounded hover:bg-blue-50 dark:hover:bg-blue-800 transition-colors disabled:opacity-50"
                >
                  <X className="w-3 h-3" />
                  Decline
                </button>
              </div>
            </div>
          </motion.div>
        ))}
      </div>
    );
  }

  // Separate section style (shows as a dedicated section)
  return (
    <div className={`bg-background-default rounded-lg border border-border-default p-4 ${className}`}>
      <div className="flex items-center gap-2 mb-4">
        <Users className="w-5 h-5 text-text-prominent" />
        <h3 className="text-lg font-semibold text-text-default">
          Outstanding Invitations ({pendingInvites.length})
        </h3>
      </div>
      
      <div className="space-y-3">
        <AnimatePresence>
          {pendingInvites.map((invite) => (
            <motion.div
              key={invite.roomId}
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="flex items-center justify-between p-3 bg-background-medium rounded-lg border border-border-default"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center">
                  <MessageSquare className="w-4 h-4 text-text-on-accent" />
                </div>
                
                <div>
                  <p className="text-sm font-medium text-text-default">
                    {getSenderDisplayName(invite)}
                  </p>
                  <p className="text-xs text-text-muted">
                    Invited you to collaborate • {formatTimestamp(invite.timestamp)}
                  </p>
                </div>
              </div>
              
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleAcceptInvite(invite)}
                  disabled={loading}
                  className="flex items-center gap-1 px-3 py-1.5 bg-green-500 text-white text-xs rounded hover:bg-green-600 transition-colors disabled:opacity-50"
                >
                  <Check className="w-3 h-3" />
                  Accept
                </button>
                
                <button
                  onClick={() => handleDeclineInvite(invite)}
                  disabled={loading}
                  className="flex items-center gap-1 px-3 py-1.5 border border-border-default text-text-default text-xs rounded hover:bg-background-medium transition-colors disabled:opacity-50"
                >
                  <X className="w-3 h-3" />
                  Decline
                </button>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
      
      {pendingInvites.length === 0 && (
        <p className="text-sm text-text-muted text-center py-4">
          No outstanding invitations
        </p>
      )}
    </div>
  );
};

export default PendingInvitesInHistory;
