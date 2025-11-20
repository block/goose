import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { TabBar, Tab } from './TabBar';
import BaseChat2 from './BaseChat2';
import { ChatType } from '../types/chat';
import { generateSessionId } from '../utils/sessionUtils';

interface TabbedChatContainerProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string, tabId: string) => void;
  initialTabs?: Tab[];
  className?: string;
  initialMessage?: string;
  sidebarCollapsed?: boolean; // Add prop to track sidebar state
}

interface TabState {
  tab: Tab;
  chat: ChatType;
  loadingChat: boolean;
}

const createNewTab = (overrides: Partial<Tab> = {}): Tab => {
  // Generate a truly unique session ID with additional entropy
  const timestamp = Date.now();
  const random = Math.random().toString(36).substr(2, 9);
  const sessionId = `new_${timestamp}_${random}`;
  
  return {
    id: `tab-${timestamp}-${random}`,
    title: 'New Chat',
    type: 'chat',
    sessionId,
    isActive: false,
    hasUnsavedChanges: false,
    ...overrides
  };
};

const createNewChat = (sessionId: string): ChatType => ({
  sessionId,
  title: 'New Chat',
  messages: [],
  messageHistoryIndex: 0,
  recipeConfig: null,
  aiEnabled: true,
});

export const TabbedChatContainer: React.FC<TabbedChatContainerProps> = ({
  setIsGoosehintsModalOpen,
  onMessageSubmit,
  initialTabs = [],
  className,
  initialMessage,
  sidebarCollapsed = false
}) => {
  // Initialize with one tab if no initial tabs provided
  const [tabStates, setTabStates] = useState<TabState[]>(() => {
    if (initialTabs.length > 0) {
      return initialTabs.map(tab => ({
        tab: { ...tab, isActive: false },
        chat: createNewChat(tab.sessionId),
        loadingChat: false
      }));
    }
    
    const firstTab = createNewTab({ isActive: true });
    return [{
      tab: firstTab,
      chat: createNewChat(firstTab.sessionId),
      loadingChat: false
    }];
  });

  const [activeTabId, setActiveTabId] = useState(() => {
    const activeTab = tabStates.find(ts => ts.tab.isActive);
    return activeTab?.tab.id || tabStates[0]?.tab.id || '';
  });

  // Update active states when activeTabId changes
  useEffect(() => {
    setTabStates(prev => prev.map(ts => ({
      ...ts,
      tab: { ...ts.tab, isActive: ts.tab.id === activeTabId }
    })));
  }, [activeTabId]);

  const handleTabClick = useCallback((tabId: string) => {
    setActiveTabId(tabId);
  }, []);

  const handleTabClose = useCallback((tabId: string) => {
    setTabStates(prev => {
      const newStates = prev.filter(ts => ts.tab.id !== tabId);
      
      // If we're closing the active tab, activate another one
      if (tabId === activeTabId && newStates.length > 0) {
        const tabIndex = prev.findIndex(ts => ts.tab.id === tabId);
        const nextActiveIndex = Math.min(tabIndex, newStates.length - 1);
        setActiveTabId(newStates[nextActiveIndex].tab.id);
      }
      
      // If this was the last tab, create a new one
      if (newStates.length === 0) {
        const newTab = createNewTab({ isActive: true });
        setActiveTabId(newTab.id);
        return [{
          tab: newTab,
          chat: createNewChat(newTab.sessionId),
          loadingChat: false
        }];
      }
      
      return newStates;
    });
  }, [activeTabId]);

  const handleNewTab = useCallback(() => {
    const newTab = createNewTab();
    const newTabState: TabState = {
      tab: newTab,
      chat: createNewChat(newTab.sessionId),
      loadingChat: false
    };
    
    setTabStates(prev => [...prev, newTabState]);
    setActiveTabId(newTab.id);
  }, []);

  const handleChatUpdate = useCallback((tabId: string, chat: ChatType) => {
    setTabStates(prev => prev.map(ts => 
      ts.tab.id === tabId 
        ? { 
            ...ts, 
            chat,
            tab: {
              ...ts.tab,
              title: chat.title || ts.tab.title,
              hasUnsavedChanges: chat.messages.length > 0,
              recipeTitle: chat.recipeConfig?.title
            }
          }
        : ts
    ));
  }, []);

  const handleMessageSubmit = useCallback((message: string, tabId: string) => {
    // Mark tab as having unsaved changes
    setTabStates(prev => prev.map(ts => 
      ts.tab.id === tabId 
        ? { ...ts, tab: { ...ts.tab, hasUnsavedChanges: true } }
        : ts
    ));
    
    onMessageSubmit?.(message, tabId);
  }, [onMessageSubmit]);

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
            setActiveTabId(tabStates[nextIndex].tab.id);
            break;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleNewTab, handleTabClose, activeTabId, tabStates]);

  const activeTabState = tabStates.find(ts => ts.tab.id === activeTabId);

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
            onMessageSubmit={(message) => handleMessageSubmit(message, activeTabState.tab.id)}
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
