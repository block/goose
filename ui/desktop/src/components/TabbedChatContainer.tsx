import React, { useCallback, useEffect, useState } from 'react';
import { TabBar } from './TabBar';
import BaseChat2 from './BaseChat2';
import { TabSidecar } from './TabSidecar';
import { useTabContext } from '../contexts/TabContext';
import { ResizableSplitter } from './Layout/ResizableSplitter';

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
    updateTabTitleFromMessage,
    updateSessionId,
    // Sidecar functions
    hideSidecarView,
    getSidecarState
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
        // Only sync titles for existing sessions (not new sessions that start with 'new_')
        // and only if the tab still shows "New Chat" or "Loading..."
        const isExistingSession = tabState.tab.sessionId && !tabState.tab.sessionId.startsWith('new_');
        const needsTitleSync = tabState.tab.title === 'New Chat' || tabState.tab.title === 'Loading...';
        
        if (isExistingSession && needsTitleSync) {
          try {
            console.log('🏷️ Attempting to sync title for existing session:', tabState.tab.sessionId);
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

  // Handle session updates from BaseChat2 (when a new session gets a real backend ID)
  const handleSessionUpdate = useCallback((newSessionId: string, tabId: string) => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (tabState && tabState.tab.sessionId !== newSessionId) {
      console.log('🔄 Session ID changed for tab:', {
        tabId,
        oldSessionId: tabState.tab.sessionId,
        newSessionId
      });
      updateSessionId(tabId, newSessionId);
    }
  }, [tabStates, updateSessionId]);

  // Handle chat updates from BaseChat2 - only update when session ID or title changes
  const handleSetChat = useCallback((chat: any) => {
    const currentActiveTabState = getActiveTabState();
    if (!currentActiveTabState) return;

    let shouldUpdate = false;
    const updates: any = {};

    // Check if session ID changed (new session got a real backend ID)
    if (chat.sessionId && chat.sessionId !== currentActiveTabState.tab.sessionId) {
      console.log('🔄 Session ID changed:', {
        tabId: currentActiveTabState.tab.id,
        oldSessionId: currentActiveTabState.tab.sessionId,
        newSessionId: chat.sessionId
      });
      updates.sessionId = chat.sessionId;
      shouldUpdate = true;
    }

    // Check if title changed (from backend session description or first message)
    if (chat.title && chat.title !== currentActiveTabState.tab.title && chat.title !== 'New Chat') {
      console.log('🏷️ Tab title changed:', {
        tabId: currentActiveTabState.tab.id,
        oldTitle: currentActiveTabState.tab.title,
        newTitle: chat.title
      });
      updates.title = chat.title;
      shouldUpdate = true;
    }

    // Only update if something actually changed
    if (shouldUpdate) {
      if (updates.sessionId) {
        updateSessionId(currentActiveTabState.tab.id, updates.sessionId);
      }
      if (updates.title) {
        // Update the chat object with new title and session ID
        const updatedChat = {
          ...currentActiveTabState.chat,
          title: updates.title,
          sessionId: updates.sessionId || currentActiveTabState.chat.sessionId
        };
        handleChatUpdate(currentActiveTabState.tab.id, updatedChat);
      }
    }
  }, [getActiveTabState, updateSessionId, handleChatUpdate]);

  // Sidecar resizing state
  const [chatWidth, setChatWidth] = useState(60); // Default 60% for chat, 40% for sidecar

  const activeTabState = getActiveTabState();
  const sidecarState = activeTabState ? getSidecarState(activeTabState.tab.id) : undefined;
  const hasSidecar = sidecarState && sidecarState.activeViews.length > 0;

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

      {/* Main Content Area - Chat and Sidecar */}
      <div className="flex-1 min-h-0 relative overflow-hidden rounded-t-lg bg-background-default">
        {activeTabState && (
          <>
            {hasSidecar ? (
              /* Resizable Split Layout: Chat + Sidecar */
              <ResizableSplitter
                leftContent={
                  <BaseChat2
                    key={activeTabState.tab.id}
                    sessionId={activeTabState.tab.sessionId}
                    setChat={handleSetChat}
                    setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                    onMessageSubmit={(message) => handleMessageSubmitWrapper(message, activeTabState.tab.id)}
                    onSessionIdChange={(newSessionId) => updateSessionId(activeTabState.tab.id, newSessionId)}
                    suppressEmptyState={false}
                    showPopularTopics={true}
                    loadingChat={activeTabState.loadingChat}
                    initialMessage={initialMessage}
                    showParticipantsBar={activeTabState.tab.type === 'matrix'}
                    matrixRoomId={activeTabState.tab.matrixRoomId}
                    showPendingInvites={activeTabState.tab.type === 'matrix'}
                    tabId={activeTabState.tab.id}
                  />
                }
                rightContent={
                  <TabSidecar
                    sidecarState={sidecarState}
                    onHideView={(viewId) => hideSidecarView(activeTabState.tab.id, viewId)}
                  />
                }
                initialLeftWidth={chatWidth}
                minLeftWidth={30}
                maxLeftWidth={80}
                onResize={setChatWidth}
                className="h-full"
                floatingRight={true}
              />
            ) : (
              /* Full Width Chat */
              <BaseChat2
                key={activeTabState.tab.id}
                sessionId={activeTabState.tab.sessionId}
                setChat={handleSetChat}
                setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                onMessageSubmit={(message) => handleMessageSubmitWrapper(message, activeTabState.tab.id)}
                onSessionIdChange={(newSessionId) => updateSessionId(activeTabState.tab.id, newSessionId)}
                suppressEmptyState={false}
                showPopularTopics={true}
                loadingChat={activeTabState.loadingChat}
                initialMessage={initialMessage}
                showParticipantsBar={activeTabState.tab.type === 'matrix'}
                matrixRoomId={activeTabState.tab.matrixRoomId}
                showPendingInvites={activeTabState.tab.type === 'matrix'}
                tabId={activeTabState.tab.id}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
};
