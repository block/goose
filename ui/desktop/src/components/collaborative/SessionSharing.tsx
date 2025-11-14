import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Users, 
  UserPlus, 
  X, 
  Crown, 
  Check,
  Clock,
  ChevronDown,
  ChevronUp
} from 'lucide-react';
import { useSessionSharing } from '../../hooks/useSessionSharing';
import { MatrixUser } from '../../services/MatrixService';
import { Message } from '../../types/message';

interface SessionSharingProps {
  sessionId: string;
  sessionTitle: string;
  messages: Message[];
  onMessageSync?: (message: Message) => void;
  className?: string;
}

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
  roomId?: string;
}

const SessionSharing: React.FC<SessionSharingProps> = ({
  sessionId,
  sessionTitle,
  messages,
  onMessageSync,
  className = '',
}) => {
  const {
    isShared,
    participants,
    isHost,
    pendingInvitations,
    error,
    canInvite,
    inviteToSession,
    joinSession,
    leaveSession,
    declineInvitation,
    getAvailableFriends,
    clearError,
  } = useSessionSharing({
    sessionId,
    sessionTitle,
    messages,
    onMessageSync,
  });

  const [isExpanded, setIsExpanded] = useState(false);
  const [showInviteModal, setShowInviteModal] = useState(false);
  const [isInviting, setIsInviting] = useState(false);

  const availableFriends = getAvailableFriends();

  const handleInviteFriend = async (friendUserId: string) => {
    setIsInviting(true);
    try {
      await inviteToSession(friendUserId);
      setShowInviteModal(false);
    } catch (error) {
      console.error('Failed to invite friend:', error);
    } finally {
      setIsInviting(false);
    }
  };

  const handleJoinSession = async (invitation: SessionInvitation) => {
    try {
      await joinSession(invitation);
    } catch (error) {
      console.error('Failed to join session:', error);
    }
  };

  // Show pending invitations at the top level
  if (pendingInvitations.length > 0 && !isShared) {
    return (
      <div className={`bg-blue-50 border border-blue-200 rounded-lg p-4 ${className}`}>
        <h3 className="font-medium text-blue-900 mb-3">Session Invitations</h3>
        <div className="space-y-3">
          {pendingInvitations.map((invitation) => (
            <div key={invitation.sessionId} className="flex items-center justify-between bg-white rounded-lg p-3 border">
              <div>
                <p className="font-medium text-gray-900">{invitation.sessionTitle}</p>
                <p className="text-sm text-gray-600">
                  Invited by {invitation.inviterName}
                </p>
                <p className="text-xs text-gray-500">
                  {invitation.timestamp.toLocaleTimeString()}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleJoinSession(invitation)}
                  className="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm"
                >
                  Join
                </button>
                <button
                  onClick={() => declineInvitation(invitation)}
                  className="px-3 py-1 border border-gray-300 rounded hover:bg-gray-50 transition-colors text-sm"
                >
                  Decline
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  // Don't show anything if not shared and no invitations
  if (!isShared) {
    return null;
  }

  return (
    <div className={`bg-background-default border border-border-default rounded-lg ${className}`}>
      {/* Header */}
      <div 
        className="flex items-center justify-between p-3 cursor-pointer hover:bg-background-medium transition-colors"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <Users className="w-4 h-4 text-blue-500" />
          <span className="text-sm font-medium">Shared Session</span>
          <span className="text-xs text-text-muted">({participants.length} participants)</span>
          {isHost && <Crown className="w-3 h-3 text-yellow-500" title="You are the host" />}
        </div>
        
        <div className="flex items-center gap-2">
          {!isExpanded && canInvite && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setShowInviteModal(true);
              }}
              className="p-1 rounded hover:bg-background-accent/20 transition-colors"
              title="Invite friend"
            >
              <UserPlus className="w-4 h-4" />
            </button>
          )}
          
          {isExpanded ? (
            <ChevronUp className="w-4 h-4" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="px-3 pb-2">
          <div className="bg-red-50 border border-red-200 rounded p-2 flex items-center justify-between">
            <span className="text-red-600 text-sm">{error}</span>
            <button
              onClick={clearError}
              className="text-red-400 hover:text-red-600"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Expanded Content */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden border-t border-border-default"
          >
            <div className="p-3">
              {/* Participants List */}
              <div className="mb-3">
                <h4 className="text-xs font-medium text-text-muted mb-2">Participants</h4>
                <div className="space-y-2">
                  {participants.map((participant) => (
                    <div key={participant.userId} className="flex items-center gap-2">
                      <div className="w-6 h-6 bg-background-accent rounded-full flex items-center justify-center">
                        <span className="text-xs font-medium text-text-on-accent">
                          {(participant.displayName || participant.userId).charAt(0).toUpperCase()}
                        </span>
                      </div>
                      <div className="flex-1">
                        <p className="text-sm font-medium">
                          {participant.displayName || participant.userId.split(':')[0].substring(1)}
                          {participant.userId === participants[0]?.userId && (
                            <Crown className="w-3 h-3 inline ml-1 text-yellow-500" />
                          )}
                        </p>
                        <p className="text-xs text-text-muted">
                          Joined {participant.joinedAt.toLocaleTimeString()}
                        </p>
                      </div>
                      {participant.isTyping && (
                        <div className="flex items-center gap-1">
                          <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
                          <span className="text-xs text-blue-500">typing...</span>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center gap-2">
                {canInvite && (
                  <button
                    onClick={() => setShowInviteModal(true)}
                    className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm"
                  >
                    <UserPlus className="w-4 h-4" />
                    Invite Friend
                  </button>
                )}
                
                <button
                  onClick={leaveSession}
                  className="px-3 py-2 border border-red-300 text-red-600 rounded hover:bg-red-50 transition-colors text-sm"
                >
                  Leave Session
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Invite Friend Modal */}
      <AnimatePresence>
        {showInviteModal && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
            onClick={() => setShowInviteModal(false)}
          >
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="bg-background-default rounded-xl p-6 max-w-md w-full mx-4"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold">Invite Friend to Session</h2>
                <button
                  onClick={() => setShowInviteModal(false)}
                  className="p-1 rounded hover:bg-background-medium transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="space-y-3 mb-6">
                {availableFriends.length === 0 ? (
                  <p className="text-text-muted text-center py-4">
                    {participants.length > 1 
                      ? 'All your friends are already in this session.'
                      : 'No friends available to invite. Add friends in the Peers page first.'
                    }
                  </p>
                ) : (
                  availableFriends.map((friend) => (
                    <div key={friend.userId} className="flex items-center justify-between p-3 border border-border-default rounded-lg">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center">
                          <span className="text-sm font-medium text-text-on-accent">
                            {(friend.displayName || friend.userId).charAt(0).toUpperCase()}
                          </span>
                        </div>
                        <div>
                          <p className="font-medium">
                            {friend.displayName || friend.userId.split(':')[0].substring(1)}
                          </p>
                          <p className="text-sm text-text-muted">{friend.userId}</p>
                        </div>
                      </div>
                      
                      <button
                        onClick={() => handleInviteFriend(friend.userId)}
                        disabled={isInviting}
                        className="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isInviting ? 'Inviting...' : 'Invite'}
                      </button>
                    </div>
                  ))
                )}
              </div>

              <div className="flex justify-end">
                <button
                  onClick={() => setShowInviteModal(false)}
                  className="px-4 py-2 border border-border-default rounded hover:bg-background-medium transition-colors"
                >
                  Close
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default SessionSharing;
