import React, { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Users, 
  UserPlus, 
  X, 
  Crown, 
  Mic, 
  MicOff, 
  Video, 
  VideoOff,
  Settings,
  LogOut,
  MessageCircle,
  Eye,
  EyeOff
} from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import { MatrixUser, GooseAIMessage } from '../../services/MatrixService';
import { Message } from '../../types/message';

interface CollaborativeSessionProps {
  sessionId: string;
  roomId?: string;
  onInviteFriend?: () => void;
  onLeaveSession?: () => void;
  onSessionUpdate?: (sessionData: CollaborativeSessionData) => void;
  messages: Message[];
  className?: string;
}

export interface CollaborativeSessionData {
  roomId: string;
  sessionName: string;
  participants: MatrixUser[];
  isHost: boolean;
  permissions: {
    canSendPrompts: boolean;
    canInviteUsers: boolean;
    canManageSession: boolean;
  };
  settings: {
    allowSpectators: boolean;
    requireApprovalForPrompts: boolean;
    shareAllMessages: boolean;
  };
}

interface SessionParticipant extends MatrixUser {
  role: 'host' | 'collaborator' | 'spectator';
  isTyping?: boolean;
  lastActive?: Date;
  permissions: {
    canSendPrompts: boolean;
    canInviteUsers: boolean;
  };
}

const CollaborativeSession: React.FC<CollaborativeSessionProps> = ({
  sessionId,
  roomId,
  onInviteFriend,
  onLeaveSession,
  onSessionUpdate,
  messages,
  className = '',
}) => {
  const { 
    currentUser, 
    friends, 
    sendAIPrompt, 
    sendAIResponse, 
    inviteToRoom,
    onAIMessage,
    isConnected 
  } = useMatrix();

  const [sessionData, setSessionData] = useState<CollaborativeSessionData | null>(null);
  const [participants, setParticipants] = useState<SessionParticipant[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [showInviteModal, setShowInviteModal] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  // Initialize session data
  useEffect(() => {
    if (roomId && currentUser) {
      const initialSessionData: CollaborativeSessionData = {
        roomId,
        sessionName: `AI Session ${sessionId.substring(0, 8)}`,
        participants: [currentUser],
        isHost: true,
        permissions: {
          canSendPrompts: true,
          canInviteUsers: true,
          canManageSession: true,
        },
        settings: {
          allowSpectators: true,
          requireApprovalForPrompts: false,
          shareAllMessages: true,
        },
      };

      setSessionData(initialSessionData);
      setParticipants([{
        ...currentUser,
        role: 'host',
        permissions: {
          canSendPrompts: true,
          canInviteUsers: true,
        },
      }]);

      onSessionUpdate?.(initialSessionData);
    }
  }, [roomId, currentUser, sessionId, onSessionUpdate]);

  // Listen for AI messages in the collaborative session
  useEffect(() => {
    if (!roomId) return;

    const handleAIMessage = (message: GooseAIMessage) => {
      if (message.sessionId === roomId) {
        console.log('Received collaborative AI message:', message);
        // Handle different types of AI messages
        switch (message.type) {
          case 'ai.prompt':
            // Someone sent a prompt - could trigger UI updates
            break;
          case 'ai.response':
            // AI responded - sync the response
            break;
          case 'ai.session.join':
            // Someone joined the session
            break;
          case 'ai.session.leave':
            // Someone left the session
            break;
        }
      }
    };

    const cleanup = onAIMessage(handleAIMessage);
    return cleanup;
  }, [roomId, onAIMessage]);

  // Sync AI prompts to Matrix room
  const syncPromptToRoom = useCallback(async (prompt: string, model?: string) => {
    if (!roomId || !sessionData) return;

    try {
      await sendAIPrompt(roomId, prompt, sessionId, model);
    } catch (error) {
      console.error('Failed to sync prompt to collaborative session:', error);
    }
  }, [roomId, sessionId, sessionData, sendAIPrompt]);

  // Sync AI responses to Matrix room
  const syncResponseToRoom = useCallback(async (response: string, model?: string) => {
    if (!roomId || !sessionData) return;

    try {
      await sendAIResponse(roomId, response, sessionId, model);
    } catch (error) {
      console.error('Failed to sync response to collaborative session:', error);
    }
  }, [roomId, sessionId, sessionData, sendAIResponse]);

  // Invite friend to session
  const handleInviteFriend = async (friendId: string) => {
    if (!roomId) return;

    try {
      await inviteToRoom(roomId, friendId);
      setShowInviteModal(false);
    } catch (error) {
      console.error('Failed to invite friend to session:', error);
    }
  };

  // Leave session
  const handleLeaveSession = () => {
    if (onLeaveSession) {
      onLeaveSession();
    }
  };

  // Update participant permissions
  const updateParticipantPermissions = (userId: string, permissions: Partial<SessionParticipant['permissions']>) => {
    setParticipants(prev => 
      prev.map(p => 
        p.userId === userId 
          ? { ...p, permissions: { ...p.permissions, ...permissions } }
          : p
      )
    );
  };

  if (!isConnected || !sessionData) {
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
          <span className="text-sm font-medium">Collaborative Session</span>
          <span className="text-xs text-text-muted">({participants.length} participants)</span>
        </div>
        
        <div className="flex items-center gap-2">
          {/* Quick actions when collapsed */}
          {!isExpanded && (
            <>
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
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setShowSettings(true);
                }}
                className="p-1 rounded hover:bg-background-accent/20 transition-colors"
                title="Session settings"
              >
                <Settings className="w-4 h-4" />
              </button>
            </>
          )}
          
          <motion.div
            animate={{ rotate: isExpanded ? 180 : 0 }}
            transition={{ duration: 0.2 }}
          >
            <X className="w-4 h-4" />
          </motion.div>
        </div>
      </div>

      {/* Expanded Content */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="p-3 pt-0 border-t border-border-default">
              {/* Session Info */}
              <div className="mb-3">
                <h3 className="font-medium text-sm mb-1">{sessionData.sessionName}</h3>
                <p className="text-xs text-text-muted">
                  Room ID: {roomId?.substring(0, 20)}...
                </p>
              </div>

              {/* Participants List */}
              <div className="mb-3">
                <h4 className="text-xs font-medium text-text-muted mb-2">Participants</h4>
                <div className="space-y-2">
                  {participants.map((participant) => (
                    <div key={participant.userId} className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <div className="w-6 h-6 bg-background-accent rounded-full flex items-center justify-center">
                          <span className="text-xs font-medium text-text-on-accent">
                            {(participant.displayName || participant.userId).charAt(0).toUpperCase()}
                          </span>
                        </div>
                        <div>
                          <p className="text-sm font-medium">
                            {participant.displayName || participant.userId.split(':')[0].substring(1)}
                            {participant.role === 'host' && (
                              <Crown className="w-3 h-3 inline ml-1 text-yellow-500" />
                            )}
                          </p>
                          <p className="text-xs text-text-muted">{participant.role}</p>
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-1">
                        {participant.permissions.canSendPrompts ? (
                          <MessageCircle className="w-3 h-3 text-green-500" title="Can send prompts" />
                        ) : (
                          <Eye className="w-3 h-3 text-gray-500" title="Spectator only" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setShowInviteModal(true)}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm"
                >
                  <UserPlus className="w-4 h-4" />
                  Invite Friend
                </button>
                
                <button
                  onClick={() => setShowSettings(true)}
                  className="px-3 py-2 border border-border-default rounded hover:bg-background-medium transition-colors"
                  title="Session settings"
                >
                  <Settings className="w-4 h-4" />
                </button>
                
                <button
                  onClick={handleLeaveSession}
                  className="px-3 py-2 border border-red-300 text-red-600 rounded hover:bg-red-50 transition-colors"
                  title="Leave session"
                >
                  <LogOut className="w-4 h-4" />
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
                {friends.length === 0 ? (
                  <p className="text-text-muted text-center py-4">
                    No friends available to invite. Add friends in the Peers page first.
                  </p>
                ) : (
                  friends.map((friend) => (
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
                        className="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm"
                      >
                        Invite
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

      {/* Session Settings Modal */}
      <AnimatePresence>
        {showSettings && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
            onClick={() => setShowSettings(false)}
          >
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="bg-background-default rounded-xl p-6 max-w-md w-full mx-4"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold">Session Settings</h2>
                <button
                  onClick={() => setShowSettings(false)}
                  className="p-1 rounded hover:bg-background-medium transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-2">Session Name</label>
                  <input
                    type="text"
                    value={sessionData.sessionName}
                    onChange={(e) => setSessionData(prev => prev ? { ...prev, sessionName: e.target.value } : null)}
                    className="w-full px-3 py-2 border border-border-default rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-medium">Permissions</h3>
                  
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={sessionData.settings.allowSpectators}
                      onChange={(e) => setSessionData(prev => prev ? {
                        ...prev,
                        settings: { ...prev.settings, allowSpectators: e.target.checked }
                      } : null)}
                      className="rounded"
                    />
                    <span className="text-sm">Allow spectators (view-only access)</span>
                  </label>

                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={sessionData.settings.requireApprovalForPrompts}
                      onChange={(e) => setSessionData(prev => prev ? {
                        ...prev,
                        settings: { ...prev.settings, requireApprovalForPrompts: e.target.checked }
                      } : null)}
                      className="rounded"
                    />
                    <span className="text-sm">Require approval for AI prompts</span>
                  </label>

                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={sessionData.settings.shareAllMessages}
                      onChange={(e) => setSessionData(prev => prev ? {
                        ...prev,
                        settings: { ...prev.settings, shareAllMessages: e.target.checked }
                      } : null)}
                      className="rounded"
                    />
                    <span className="text-sm">Share all messages with participants</span>
                  </label>
                </div>
              </div>

              <div className="flex justify-end gap-2 mt-6">
                <button
                  onClick={() => setShowSettings(false)}
                  className="px-4 py-2 border border-border-default rounded hover:bg-background-medium transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={() => {
                    if (sessionData) {
                      onSessionUpdate?.(sessionData);
                    }
                    setShowSettings(false);
                  }}
                  className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
                >
                  Save Changes
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default CollaborativeSession;
