import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react';
import { Tab } from '../components/TabBar';
import { ChatType } from '../types/chat';
import { generateSessionId } from '../utils/sessionUtils';

interface TabState {
  tab: Tab;
  chat: ChatType;
  loadingChat: boolean;
}

interface TabContextType {
  tabStates: TabState[];
  activeTabId: string;
  setActiveTabId: (tabId: string) => void;
  handleTabClick: (tabId: string) => void;
  handleTabClose: (tabId: string) => void;
  handleNewTab: () => void;
  handleChatUpdate: (tabId: string, chat: ChatType) => void;
  handleMessageSubmit: (message: string, tabId: string) => void;
  getActiveTabState: () => TabState | undefined;
  restoreTabState: () => void;
  clearTabState: () => void;
}

const TabContext = createContext<TabContextType | undefined>(undefined);

const TAB_STATE_STORAGE_KEY = 'goose-tab-state';

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

const createInitialTabState = (): TabState[] => {
  const firstTab = createNewTab({ isActive: true });
  return [{
    tab: firstTab,
    chat: createNewChat(firstTab.sessionId),
    loadingChat: false
  }];
};

interface TabProviderProps {
  children: ReactNode;
}

export const TabProvider: React.FC<TabProviderProps> = ({ children }) => {
  const [tabStates, setTabStates] = useState<TabState[]>(() => {
    // Try to restore from localStorage on initial load
    try {
      const saved = localStorage.getItem(TAB_STATE_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed;
        }
      }
    } catch (error) {
      console.warn('Failed to restore tab state from localStorage:', error);
    }
    return createInitialTabState();
  });

  const [activeTabId, setActiveTabId] = useState(() => {
    const activeTab = tabStates.find(ts => ts.tab.isActive);
    return activeTab?.tab.id || tabStates[0]?.tab.id || '';
  });

  // Save tab state to localStorage whenever it changes
  useEffect(() => {
    try {
      localStorage.setItem(TAB_STATE_STORAGE_KEY, JSON.stringify(tabStates));
    } catch (error) {
      console.warn('Failed to save tab state to localStorage:', error);
    }
  }, [tabStates]);

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
  }, []);

  const getActiveTabState = useCallback(() => {
    return tabStates.find(ts => ts.tab.id === activeTabId);
  }, [tabStates, activeTabId]);

  const restoreTabState = useCallback(() => {
    try {
      const saved = localStorage.getItem(TAB_STATE_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          setTabStates(parsed);
          const activeTab = parsed.find((ts: TabState) => ts.tab.isActive);
          if (activeTab) {
            setActiveTabId(activeTab.tab.id);
          }
          return;
        }
      }
    } catch (error) {
      console.warn('Failed to restore tab state:', error);
    }
    
    // Fallback to initial state if restore fails
    const initialState = createInitialTabState();
    setTabStates(initialState);
    setActiveTabId(initialState[0].tab.id);
  }, []);

  const clearTabState = useCallback(() => {
    try {
      localStorage.removeItem(TAB_STATE_STORAGE_KEY);
    } catch (error) {
      console.warn('Failed to clear tab state from localStorage:', error);
    }
    
    const initialState = createInitialTabState();
    setTabStates(initialState);
    setActiveTabId(initialState[0].tab.id);
  }, []);

  const contextValue: TabContextType = {
    tabStates,
    activeTabId,
    setActiveTabId,
    handleTabClick,
    handleTabClose,
    handleNewTab,
    handleChatUpdate,
    handleMessageSubmit,
    getActiveTabState,
    restoreTabState,
    clearTabState
  };

  return (
    <TabContext.Provider value={contextValue}>
      {children}
    </TabContext.Provider>
  );
};

export const useTabContext = (): TabContextType => {
  const context = useContext(TabContext);
  if (!context) {
    throw new Error('useTabContext must be used within a TabProvider');
  }
  return context;
};
