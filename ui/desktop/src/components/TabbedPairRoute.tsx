import React, { useEffect } from 'react';
import { useLocation, useSearchParams } from 'react-router-dom';
import { TabbedChatContainer } from './TabbedChatContainer';
import { ViewOptions } from '../utils/navigationUtils';
import { ContextManagerProvider } from './context_management/ContextManager';
import { useNavigation } from './Layout/AppLayout';
import { useTabContext } from '../contexts/TabContext';

interface TabbedPairRouteProps {
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export const TabbedPairRoute: React.FC<TabbedPairRouteProps> = ({
  setIsGoosehintsModalOpen
}) => {
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const routeState = location.state as ViewOptions | undefined;
  const initialMessage = routeState?.initialMessage;
  const { isNavExpanded } = useNavigation();
  const { openExistingSession, openMatrixChat, handleNewTab } = useTabContext();

  // Track if we've already handled the initial message to prevent duplicate handling
  const [hasHandledInitialMessage, setHasHandledInitialMessage] = React.useState(false);

  // Handle initial message from Hub - create a new tab with the message
  useEffect(() => {
    if (initialMessage && !hasHandledInitialMessage) {
      console.log('📝 TabbedPairRoute: Handling initial message from Hub:', initialMessage);
      
      // Create a new tab - the initialMessage will be passed to TabbedChatContainer
      // and auto-submitted in the new tab
      handleNewTab();
      setHasHandledInitialMessage(true);
      
      // Clear the location state to prevent re-handling on navigation
      window.history.replaceState({}, '', window.location.href);
    }
  }, [initialMessage, hasHandledInitialMessage, handleNewTab]);

  // Handle resuming existing sessions from URL parameters
  useEffect(() => {
    const resumeSessionId = searchParams.get('resumeSessionId');
    if (resumeSessionId) {
      console.log('📂 Resuming session from URL parameter:', resumeSessionId);
      openExistingSession(resumeSessionId);
      
      // Clear the URL parameter to prevent re-opening on refresh
      const newSearchParams = new URLSearchParams(searchParams);
      newSearchParams.delete('resumeSessionId');
      const newUrl = `${window.location.pathname}${newSearchParams.toString() ? '?' + newSearchParams.toString() : ''}`;
      window.history.replaceState({}, '', newUrl);
    }
  }, [searchParams, openExistingSession]);

  // Handle Matrix tab creation from notifications
  useEffect(() => {
    const handleCreateMatrixTab = (event: CustomEvent) => {
      const { roomId, senderId } = event.detail;
      console.log('📱 TabbedPairRoute: Creating Matrix tab for room:', roomId, 'sender:', senderId);
      
      // Use the new openMatrixChat method - much simpler!
      openMatrixChat(roomId, senderId);
    };

    window.addEventListener('create-matrix-tab', handleCreateMatrixTab as EventListener);
    return () => {
      window.removeEventListener('create-matrix-tab', handleCreateMatrixTab as EventListener);
    };
  }, [openMatrixChat]);

  const handleMessageSubmit = (message: string, tabId: string) => {
    console.log('Message submitted in tab:', tabId, message);
    // Here you could add analytics, logging, or other side effects
  };

  return (
    <ContextManagerProvider>
      <TabbedChatContainer
        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        onMessageSubmit={handleMessageSubmit}
        initialMessage={initialMessage}
        sidebarCollapsed={!isNavExpanded}
      />
    </ContextManagerProvider>
  );
};
