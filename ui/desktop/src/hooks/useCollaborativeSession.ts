import { useState, useEffect, useCallback } from 'react';
import { useMatrix } from '../contexts/MatrixContext';
import { CollaborativeSessionData } from '../components/collaborative/CollaborativeSession';
import { Message } from '../types/message';

interface UseCollaborativeSessionProps {
  sessionId: string;
  messages: Message[];
  onMessageSync?: (message: Message) => void;
}

interface CollaborativeSessionState {
  isCollaborative: boolean;
  roomId: string | null;
  sessionData: CollaborativeSessionData | null;
  isCreatingSession: boolean;
  error: string | null;
}

export const useCollaborativeSession = ({
  sessionId,
  messages,
  onMessageSync,
}: UseCollaborativeSessionProps) => {
  const { 
    createAISession, 
    sendAIPrompt, 
    sendAIResponse, 
    onAIMessage, 
    isConnected,
    currentUser 
  } = useMatrix();

  const [state, setState] = useState<CollaborativeSessionState>({
    isCollaborative: false,
    roomId: null,
    sessionData: null,
    isCreatingSession: false,
    error: null,
  });

  // Listen for AI messages from Matrix
  useEffect(() => {
    if (!state.roomId) return;

    const handleAIMessage = (message: any) => {
      if (message.sessionId === state.roomId) {
        console.log('Received collaborative AI message:', message);
        
        // Convert Matrix AI message to local message format if needed
        if (onMessageSync && message.type === 'ai.response') {
          // Create a message object that matches our local format
          const localMessage: Message = {
            id: `matrix-${Date.now()}`,
            role: 'assistant',
            created: Math.floor(Date.now() / 1000),
            content: [{
              type: 'text',
              text: message.content,
            }],
          };
          
          onMessageSync(localMessage);
        }
      }
    };

    const cleanup = onAIMessage(handleAIMessage);
    return cleanup;
  }, [state.roomId, onAIMessage, onMessageSync]);

  // Create a new collaborative session
  const startCollaborativeSession = useCallback(async (sessionName?: string) => {
    if (!currentUser || state.isCreatingSession) return;

    setState(prev => ({ ...prev, isCreatingSession: true, error: null }));

    try {
      const name = sessionName || `AI Session ${sessionId.substring(0, 8)}`;
      const roomId = await createAISession(name);

      const sessionData: CollaborativeSessionData = {
        roomId,
        sessionName: name,
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

      setState(prev => ({
        ...prev,
        isCollaborative: true,
        roomId,
        sessionData,
        isCreatingSession: false,
      }));

      return roomId;
    } catch (error) {
      console.error('Failed to create collaborative session:', error);
      setState(prev => ({
        ...prev,
        isCreatingSession: false,
        error: error instanceof Error ? error.message : 'Failed to create session',
      }));
      throw error;
    }
  }, [currentUser, sessionId, createAISession, state.isCreatingSession]);

  // End collaborative session
  const endCollaborativeSession = useCallback(() => {
    setState({
      isCollaborative: false,
      roomId: null,
      sessionData: null,
      isCreatingSession: false,
      error: null,
    });
  }, []);

  // Sync a prompt to the collaborative session
  const syncPrompt = useCallback(async (prompt: string, model?: string) => {
    if (!state.roomId || !state.sessionData?.settings.shareAllMessages) return;

    try {
      await sendAIPrompt(state.roomId, prompt, sessionId, model);
    } catch (error) {
      console.error('Failed to sync prompt:', error);
    }
  }, [state.roomId, state.sessionData, sessionId, sendAIPrompt]);

  // Sync an AI response to the collaborative session
  const syncResponse = useCallback(async (response: string, model?: string) => {
    if (!state.roomId || !state.sessionData?.settings.shareAllMessages) return;

    try {
      await sendAIResponse(state.roomId, response, sessionId, model);
    } catch (error) {
      console.error('Failed to sync response:', error);
    }
  }, [state.roomId, state.sessionData, sessionId, sendAIResponse]);

  // Update session data
  const updateSessionData = useCallback((newSessionData: CollaborativeSessionData) => {
    setState(prev => ({
      ...prev,
      sessionData: newSessionData,
    }));
  }, []);

  // Join an existing collaborative session (when invited)
  const joinCollaborativeSession = useCallback(async (roomId: string) => {
    if (!currentUser) return;

    try {
      // In a real implementation, you'd fetch session data from the Matrix room
      const sessionData: CollaborativeSessionData = {
        roomId,
        sessionName: `AI Session ${roomId.substring(0, 8)}`,
        participants: [currentUser], // Would be populated from room members
        isHost: false,
        permissions: {
          canSendPrompts: true, // Would be determined by room permissions
          canInviteUsers: false,
          canManageSession: false,
        },
        settings: {
          allowSpectators: true,
          requireApprovalForPrompts: false,
          shareAllMessages: true,
        },
      };

      setState({
        isCollaborative: true,
        roomId,
        sessionData,
        isCreatingSession: false,
        error: null,
      });

      return sessionData;
    } catch (error) {
      console.error('Failed to join collaborative session:', error);
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to join session',
      }));
      throw error;
    }
  }, [currentUser]);

  return {
    // State
    isCollaborative: state.isCollaborative,
    roomId: state.roomId,
    sessionData: state.sessionData,
    isCreatingSession: state.isCreatingSession,
    error: state.error,
    canCreateSession: isConnected && !!currentUser,

    // Actions
    startCollaborativeSession,
    endCollaborativeSession,
    joinCollaborativeSession,
    syncPrompt,
    syncResponse,
    updateSessionData,
    
    // Clear error
    clearError: () => setState(prev => ({ ...prev, error: null })),
  };
};
