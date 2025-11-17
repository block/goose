import { useLocation } from 'react-router-dom';
import { useChatContext } from '../contexts/ChatContext';

/**
 * Hook to detect the currently active session/room to prevent notifications
 * for messages from the room the user is currently viewing
 */
export const useActiveSession = () => {
  const location = useLocation();
  const chatContext = useChatContext();

  // Get Matrix room information from URL parameters
  const getActiveMatrixRoom = () => {
    const searchParams = new URLSearchParams(location.search);
    const isMatrixMode = searchParams.get('matrixMode') === 'true';
    const matrixRoomId = searchParams.get('matrixRoomId');
    
    return isMatrixMode && matrixRoomId ? matrixRoomId : null;
  };

  // Get current session ID from chat context
  const getActiveSessionId = () => {
    return chatContext?.chat?.sessionId || null;
  };

  // Get current page/view information
  const getCurrentView = () => {
    const path = location.pathname;
    const searchParams = new URLSearchParams(location.search);
    
    return {
      path,
      isMatrixMode: searchParams.get('matrixMode') === 'true',
      matrixRoomId: searchParams.get('matrixRoomId'),
      matrixRecipientId: searchParams.get('matrixRecipientId'),
      sessionId: getActiveSessionId(),
    };
  };

  // Check if a message should be suppressed (no notification shown)
  const shouldSuppressNotification = (messageRoomId: string, messageSenderId?: string) => {
    const currentView = getCurrentView();
    
    // If we're in Matrix mode and the message is from the current Matrix room, suppress it
    if (currentView.isMatrixMode && currentView.matrixRoomId === messageRoomId) {
      console.log('🔕 Suppressing notification: message from current Matrix room', {
        messageRoomId,
        currentMatrixRoomId: currentView.matrixRoomId,
        path: currentView.path
      });
      return true;
    }

    // If we're in a regular chat session and have a session mapping to this Matrix room, suppress it
    if (currentView.sessionId && messageRoomId) {
      // Note: We could add session mapping logic here if needed
      // For now, we rely on the Matrix room check above
    }

    // If we're in pair view with a specific recipient and the message is from that recipient, suppress it
    if (currentView.path.startsWith('/pair') && 
        currentView.matrixRecipientId && 
        messageSenderId === currentView.matrixRecipientId) {
      console.log('🔕 Suppressing notification: message from current pair recipient', {
        messageSenderId,
        currentRecipientId: currentView.matrixRecipientId,
        path: currentView.path
      });
      return true;
    }

    // Don't suppress - show the notification
    return false;
  };

  return {
    getActiveMatrixRoom,
    getActiveSessionId,
    getCurrentView,
    shouldSuppressNotification,
  };
};
