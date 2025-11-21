import { useLocation } from 'react-router-dom';
import { useChatContext } from '../contexts/ChatContext';
import { useTabContext } from '../contexts/TabContext';

/**
 * Hook to detect the currently active session/room to prevent notifications
 * for messages from the room the user is currently viewing
 * 
 * Updated for new tabbed architecture - checks both legacy URL params and new tab state
 */
export const useActiveSession = () => {
  const location = useLocation();
  const chatContext = useChatContext();
  
  // NEW: Access tab context for tabbed architecture
  const tabContext = useTabContext();

  // Get Matrix room information from URL parameters (legacy support)
  const getActiveMatrixRoomFromURL = () => {
    const searchParams = new URLSearchParams(location.search);
    const isMatrixMode = searchParams.get('matrixMode') === 'true';
    const matrixRoomId = searchParams.get('matrixRoomId');
    
    return isMatrixMode && matrixRoomId ? matrixRoomId : null;
  };

  // NEW: Get Matrix room information from active tab state
  const getActiveMatrixRoomFromTab = () => {
    if (!tabContext) return null;
    
    try {
      const activeTabState = tabContext.getActiveTabState();
      // FIXED: Only return Matrix room ID if the CURRENTLY ACTIVE tab is a Matrix tab
      // getActiveTabState() already returns the currently active tab
      if (activeTabState?.tab.type === 'matrix' && activeTabState.tab.matrixRoomId) {
        return activeTabState.tab.matrixRoomId;
      }
    } catch (error) {
      // Tab context might not be available in all routes
      console.debug('Tab context not available for Matrix room detection');
    }
    
    return null;
  };

  // Combined Matrix room detection (tab state takes priority over URL)
  const getActiveMatrixRoom = () => {
    return getActiveMatrixRoomFromTab() || getActiveMatrixRoomFromURL();
  };

  // Get current session ID from chat context (legacy) or tab context (new)
  const getActiveSessionId = () => {
    // Try tab context first (new architecture)
    if (tabContext) {
      try {
        const activeTabState = tabContext.getActiveTabState();
        if (activeTabState?.tab.sessionId) {
          return activeTabState.tab.sessionId;
        }
      } catch (error) {
        console.debug('Tab context not available for session ID detection');
      }
    }
    
    // Fallback to chat context (legacy)
    return chatContext?.chat?.sessionId || null;
  };

  // NEW: Get all active Matrix rooms from all tabs
  const getAllActiveMatrixRooms = () => {
    if (!tabContext) return [];
    
    try {
      return tabContext.tabStates
        .filter(ts => ts.tab.type === 'matrix' && ts.tab.matrixRoomId)
        .map(ts => ts.tab.matrixRoomId!);
    } catch (error) {
      console.debug('Tab context not available for all Matrix rooms detection');
      return [];
    }
  };

  // Get current page/view information (enhanced for tabbed architecture)
  const getCurrentView = () => {
    const path = location.pathname;
    const searchParams = new URLSearchParams(location.search);
    
    // Legacy URL-based detection
    const urlMatrixMode = searchParams.get('matrixMode') === 'true';
    const urlMatrixRoomId = searchParams.get('matrixRoomId');
    const urlMatrixRecipientId = searchParams.get('matrixRecipientId');
    
    // NEW: Tab-based detection
    let tabMatrixRoomId = null;
    let tabMatrixRecipientId = null;
    let isTabMatrixMode = false;
    
    if (tabContext) {
      try {
        const activeTabState = tabContext.getActiveTabState();
        console.log('🔍 getCurrentView tab detection:', {
          activeTabId: tabContext.activeTabId,
          activeTabState: activeTabState ? {
            id: activeTabState.tab.id,
            type: activeTabState.tab.type,
            title: activeTabState.tab.title,
            matrixRoomId: activeTabState.tab.matrixRoomId,
            matrixRecipientId: activeTabState.tab.matrixRecipientId
          } : null,
          // Show all tabs to understand the full state
          allTabs: tabContext.tabStates.map(ts => ({
            id: ts.tab.id,
            type: ts.tab.type,
            title: ts.tab.title,
            isActive: ts.tab.isActive,
            matrixRoomId: ts.tab.matrixRoomId
          })),
          // Debug timing
          timestamp: new Date().toISOString(),
          caller: 'getCurrentView'
        });
        
        if (activeTabState?.tab.type === 'matrix') {
          isTabMatrixMode = true;
          tabMatrixRoomId = activeTabState.tab.matrixRoomId || null;
          tabMatrixRecipientId = activeTabState.tab.matrixRecipientId || null;
          console.log('🔍 Matrix tab detected as active:', {
            tabMatrixRoomId,
            tabMatrixRecipientId
          });
        } else {
          console.log('🔍 Active tab is not Matrix type:', activeTabState?.tab.type);
        }
      } catch (error) {
        console.debug('Tab context not available for current view detection');
      }
    }
    
    const result = {
      path,
      // FIXED: In tabbed mode, ONLY use tab state, ignore URL params completely
      // URL params are legacy and can be stale in tabbed environment
      isMatrixMode: tabContext ? isTabMatrixMode : urlMatrixMode,
      matrixRoomId: tabContext ? tabMatrixRoomId : urlMatrixRoomId,
      matrixRecipientId: tabContext ? tabMatrixRecipientId : urlMatrixRecipientId,
      sessionId: getActiveSessionId(),
      // NEW: Additional tab context info
      isTabbed: !!tabContext,
      allActiveMatrixRooms: getAllActiveMatrixRooms(),
    };
    
    console.log('🔍 getCurrentView result:', result);
    return result;
  };

  // Check if a message should be suppressed (no notification shown)
  const shouldSuppressNotification = (messageRoomId: string, messageSenderId?: string) => {
    const currentView = getCurrentView();
    
    console.log('🔍 shouldSuppressNotification check:', {
      messageRoomId,
      messageSenderId,
      currentView: {
        path: currentView.path,
        isMatrixMode: currentView.isMatrixMode,
        matrixRoomId: currentView.matrixRoomId,
        matrixRecipientId: currentView.matrixRecipientId,
        isTabbed: currentView.isTabbed,
        allActiveMatrixRooms: currentView.allActiveMatrixRooms
      }
    });
    
    // FIXED: Only suppress if the message is from the CURRENTLY ACTIVE Matrix room tab
    // Not just any open tab - only if you're actively viewing that specific room
    if (currentView.isMatrixMode && currentView.matrixRoomId === messageRoomId) {
      console.log('🔕 Suppressing notification: message from currently active Matrix room', {
        messageRoomId,
        currentActiveMatrixRoomId: currentView.matrixRoomId,
        path: currentView.path
      });
      return true;
    }
    
    // DEBUG: Check if we're detecting the Matrix room correctly in tab context
    const activeTabState = tabContext?.getActiveTabState();
    console.log('🔍 Matrix room detection debug:', {
      messageRoomId,
      isMatrixMode: currentView.isMatrixMode,
      currentMatrixRoomId: currentView.matrixRoomId,
      tabMatrixRoomId: getActiveMatrixRoomFromTab(),
      urlMatrixRoomId: getActiveMatrixRoomFromURL(),
      shouldSuppressBasedOnActiveRoom: currentView.isMatrixMode && currentView.matrixRoomId === messageRoomId,
      // Additional debug info
      activeTabId: tabContext?.activeTabId,
      activeTabType: activeTabState?.tab.type,
      activeTabMatrixRoomId: activeTabState?.tab.matrixRoomId,
      activeTabTitle: activeTabState?.tab.title,
      allTabStates: tabContext?.tabStates.map(ts => ({
        id: ts.tab.id,
        type: ts.tab.type,
        title: ts.tab.title,
        matrixRoomId: ts.tab.matrixRoomId,
        isActive: ts.tab.isActive
      }))
    });

    // If we're in a regular chat session and have a session mapping to this Matrix room, suppress it
    if (currentView.sessionId && messageRoomId) {
      // Note: We could add session mapping logic here if needed
      // For now, we rely on the Matrix room check above
    }

    // Legacy: If we're in pair view with a specific recipient and the message is from that recipient, suppress it
    if (currentView.path.startsWith('/pair') && 
        currentView.matrixRecipientId && 
        messageSenderId === currentView.matrixRecipientId) {
      console.log('🔕 Suppressing notification: message from current pair recipient (legacy)', {
        messageSenderId,
        currentRecipientId: currentView.matrixRecipientId,
        path: currentView.path
      });
      return true;
    }

    // Don't suppress - show the notification
    console.log('✅ Not suppressing notification: no active session match');
    return false;
  };

  return {
    getActiveMatrixRoom,
    getActiveSessionId,
    getCurrentView,
    shouldSuppressNotification,
    // NEW: Additional methods for tabbed architecture
    getAllActiveMatrixRooms,
  };
};
