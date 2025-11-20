import React, { useCallback, useEffect } from 'react';
import { TabBar } from './TabBar';
import BaseChat2 from './BaseChat2';
import { useTabContext } from '../contexts/TabContext';

interface TabbedChatContainerProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string, tabId: string) => void;
  className?: string;
  initialMessage?: string;
  sidebarCollapsed?: boolean; // Add prop to track sidebar state
}

export const TabbedChatContainer: React.FC<TabbedChatContainerProps> = ({
  setIsGoosehintsModalOpen,
  onMessageSubmit,
  className,
  initialMessage,
  sidebarCollapsed = false
}) => {
  const {
    tabStates,
    activeTabId,
    handleTabClick,
    handleTabClose,
    handleNewTab,
    handleChatUpdate,
    handleMessageSubmit: contextHandleMessageSubmit,
    getActiveTabState,
    syncTabTitleWithBackend,
    updateTabTitleFromMessage
  } = useTabContext();

  const handleMessageSubmitWrapper = useCallback(async (message: string, tabId: string) => {
    // Find the tab state to check if this is the first message
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    const isFirstMessage = tabState && tabState.chat.messages.length === 0 && tabState.tab.title === 'New Chat';
    
    // Handle the message submission
    contextHandleMessageSubmit(message, tabId);
    onMessageSubmit?.(message, tabId);
    
    // Update tab title from first message
    if (isFirstMessage && message.trim()) {
      console.log('🏷️ First message detected, updating tab title');
      try {
        await updateTabTitleFromMessage(tabId, message);
      } catch (error) {
        console.warn('Failed to update tab title from message:', error);
      }
    }
  }, [contextHandleMessageSubmit, onMessageSubmit, tabStates, updateTabTitleFromMessage]);

  // Sync tab titles with backend when component mounts or tabs change
  useEffect(() => {
    const syncAllTabTitles = async () => {
      for (const tabState of tabStates) {
        // Sync titles for any tab that shows "New Chat" and has a session ID
        // This includes both existing sessions and new sessions that might have been created
        if (tabState.tab.sessionId && tabState.tab.title === 'New Chat') {
          try {
            console.log('🏷️ Attempting to sync title for session:', tabState.tab.sessionId);
            await syncTabTitleWithBackend(tabState.tab.id);
          } catch (error) {
            console.warn('Failed to sync tab title for session:', tabState.tab.sessionId, error);
          }
        }
      }
    };

    // Only run on mount or when we have tabs to sync
    if (tabStates.length > 0) {
      syncAllTabTitles();
    }
  }, [tabStates, syncTabTitleWithBackend]); // Run when tabStates change

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey)) {
        switch (e.key) {
          case 't':
            e.preventDefault();
            handleNewTab();
            break;
          case 'w':
            if (tabStates.length > 1) {
              e.preventDefault();
              handleTabClose(activeTabId);
            }
            break;
          case 'Tab':
            e.preventDefault();
            const currentIndex = tabStates.findIndex(ts => ts.tab.id === activeTabId);
            const nextIndex = e.shiftKey 
              ? (currentIndex - 1 + tabStates.length) % tabStates.length
              : (currentIndex + 1) % tabStates.length;
            handleTabClick(tabStates[nextIndex].tab.id);
            break;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleNewTab, handleTabClose, handleTabClick, activeTabId, tabStates]);

  const activeTabState = getActiveTabState();

  return (
    <div className={`flex flex-col h-full bg-background-default ${className || ''}`}>
      {/* Tab Bar - Fixed at top */}
      <div className="flex-shrink-0 relative z-10">
        <TabBar
          tabs={tabStates.map(ts => ts.tab)}
          activeTabId={activeTabId}
          onTabClick={handleTabClick}
          onTabClose={handleTabClose}
          onNewTab={handleNewTab}
          sidebarCollapsed={sidebarCollapsed}
        />
      </div>

      {/* Active Chat - Takes remaining space with rounded top corners */}
      <div className="flex-1 min-h-0 relative overflow-hidden rounded-t-lg bg-background-default">
        {activeTabState && (
          <BaseChat2
            key={activeTabState.tab.sessionId} // Force React to create new instance for each session
            sessionId={activeTabState.tab.sessionId}
            setChat={(chat) => handleChatUpdate(activeTabState.tab.id, chat)}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            onMessageSubmit={(message) => handleMessageSubmitWrapper(message, activeTabState.tab.id)}
            suppressEmptyState={false}
            showPopularTopics={true}
            loadingChat={activeTabState.loadingChat}
            initialMessage={initialMessage}
            // Matrix props (if this tab is a Matrix session)
            showParticipantsBar={activeTabState.tab.type === 'matrix'}
            matrixRoomId={activeTabState.tab.matrixRoomId}
            showPendingInvites={activeTabState.tab.type === 'matrix'}
          />
        )}
      </div>
    </div>
  );
};
