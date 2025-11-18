import { useState, useEffect, useCallback } from 'react';
import { matrixInviteStateService, InviteState } from '../services/MatrixInviteStateService';
import { useMatrix } from '../contexts/MatrixContext';
import { Message } from '../types/message';

interface PendingInviteMessage extends Message {
  inviteData: InviteState;
  isInvite: true;
}

export const usePendingInvites = () => {
  const { isConnected, joinRoom } = useMatrix();
  const [pendingInvites, setPendingInvites] = useState<InviteState[]>([]);
  const [loading, setLoading] = useState(false);

  // Load pending invites
  const loadPendingInvites = useCallback(() => {
    if (!isConnected) {
      setPendingInvites([]);
      return;
    }

    const invites = matrixInviteStateService.getPendingInvites();
    console.log('🔗 usePendingInvites: Loaded', invites.length, 'pending invites');
    setPendingInvites(invites);
  }, [isConnected]);

  useEffect(() => {
    loadPendingInvites();

    // Listen for storage changes (invite state updates)
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'goose-matrix-invite-states') {
        loadPendingInvites();
      }
    };

    window.addEventListener('storage', handleStorageChange);

    // Refresh every 30 seconds
    const interval = setInterval(loadPendingInvites, 30000);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      clearInterval(interval);
    };
  }, [loadPendingInvites]);

  // Convert pending invites to chat messages
  const getPendingInvitesAsMessages = useCallback((): PendingInviteMessage[] => {
    return pendingInvites.map((invite) => ({
      id: `invite-${invite.roomId}`,
      role: 'system' as const,
      created: Math.floor(invite.timestamp / 1000),
      content: [
        {
          type: 'text' as const,
          text: `${invite.inviterName || invite.inviter.split(':')[0].substring(1)} invited you to collaborate`,
        }
      ],
      sender: {
        userId: invite.inviter,
        displayName: invite.inviterName,
      },
      metadata: {
        isInvite: true,
        inviteType: 'collaboration',
        roomId: invite.roomId,
        inviter: invite.inviter,
        inviterName: invite.inviterName,
        timestamp: invite.timestamp,
        status: invite.status,
      },
      inviteData: invite,
      isInvite: true as const,
    }));
  }, [pendingInvites]);

  // Accept an invite
  const acceptInvite = useCallback(async (invite: InviteState) => {
    setLoading(true);
    try {
      console.log('🤝 usePendingInvites: Accepting invite:', invite.roomId);
      
      // Join the room
      await joinRoom(invite.roomId);
      
      // Mark as accepted
      matrixInviteStateService.acceptInvite(invite.roomId);
      
      // Refresh the list
      loadPendingInvites();
      
      console.log('✅ usePendingInvites: Successfully accepted invite');
      return true;
      
    } catch (error) {
      console.error('❌ usePendingInvites: Failed to accept invite:', error);
      throw error;
    } finally {
      setLoading(false);
    }
  }, [joinRoom, loadPendingInvites]);

  // Decline an invite
  const declineInvite = useCallback(async (invite: InviteState) => {
    setLoading(true);
    try {
      console.log('❌ usePendingInvites: Declining invite:', invite.roomId);
      
      // Mark as declined
      matrixInviteStateService.declineInvite(invite.roomId);
      
      // Refresh the list
      loadPendingInvites();
      
      console.log('✅ usePendingInvites: Successfully declined invite');
      return true;
      
    } catch (error) {
      console.error('❌ usePendingInvites: Failed to decline invite:', error);
      throw error;
    } finally {
      setLoading(false);
    }
  }, [loadPendingInvites]);

  // Dismiss an invite (hide without accepting/declining)
  const dismissInvite = useCallback(async (invite: InviteState) => {
    console.log('🙈 usePendingInvites: Dismissing invite:', invite.roomId);
    
    // Mark as dismissed
    matrixInviteStateService.dismissInvite(invite.roomId);
    
    // Refresh the list
    loadPendingInvites();
    
    console.log('✅ usePendingInvites: Successfully dismissed invite');
  }, [loadPendingInvites]);

  // Get invite statistics
  const getInviteStats = useCallback(() => {
    return matrixInviteStateService.getInviteStats();
  }, []);

  // Refresh invites manually
  const refreshInvites = useCallback(() => {
    loadPendingInvites();
  }, [loadPendingInvites]);

  return {
    // Data
    pendingInvites,
    pendingInvitesAsMessages: getPendingInvitesAsMessages(),
    loading,
    
    // Actions
    acceptInvite,
    declineInvite,
    dismissInvite,
    refreshInvites,
    
    // Utils
    getInviteStats,
    hasPendingInvites: pendingInvites.length > 0,
    pendingInvitesCount: pendingInvites.length,
  };
};

export default usePendingInvites;
