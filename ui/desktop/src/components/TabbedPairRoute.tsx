import React from 'react';
import { useLocation } from 'react-router-dom';
import { TabbedChatContainer } from './TabbedChatContainer';
import { ViewOptions } from '../utils/navigationUtils';
import { ContextManagerProvider } from './context_management/ContextManager';
import { useNavigation } from './Layout/AppLayout';

interface TabbedPairRouteProps {
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}

export const TabbedPairRoute: React.FC<TabbedPairRouteProps> = ({
  setIsGoosehintsModalOpen
}) => {
  const location = useLocation();
  const routeState = location.state as ViewOptions | undefined;
  const initialMessage = routeState?.initialMessage;
  const { isNavExpanded } = useNavigation();

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
