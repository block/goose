import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react';
import { Tab, TabSidecarState, TabSidecarView } from '../components/TabBar';
import { ChatType } from '../types/chat';
import { generateSessionId } from '../utils/sessionUtils';
import { getSession, updateSessionDescription } from '../api';

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
  syncTabTitleWithBackend: (tabId: string) => Promise<void>;
  updateTabTitleFromMessage: (tabId: string, message: string | any) => Promise<void>;
  openExistingSession: (sessionId: string, title?: string) => void;
  updateSessionId: (tabId: string, newSessionId: string) => void;
  // Sidecar management functions
  showSidecarView: (tabId: string, view: TabSidecarView) => void;
  hideSidecarView: (tabId: string, viewId: string) => void;
  hideAllSidecarViews: (tabId: string) => void;
  getSidecarState: (tabId: string) => TabSidecarState | undefined;
  showDiffViewer: (tabId: string, diffContent: string, fileName?: string, instanceId?: string) => void;
  showLocalhostViewer: (tabId: string, url?: string, title?: string, instanceId?: string) => void;
  showFileViewer: (tabId: string, filePath: string, instanceId?: string) => void;
  showDocumentEditor: (tabId: string, filePath?: string, initialContent?: string, instanceId?: string) => void;
}

const TabContext = createContext<TabContextType | undefined>(undefined);

const TAB_STATE_STORAGE_KEY = 'goose-tab-state';

const createNewTab = (overrides: Partial<Tab> = {}): Tab => {
  // Generate a truly unique session ID with additional entropy
  const timestamp = Date.now();
  const random = Math.random().toString(36).substr(2, 9);
  const sessionId = overrides.sessionId || `new_${timestamp}_${random}`;
  
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

    // If the chat has a title and the tab doesn't, update the tab title
    if (chat.title && chat.title !== 'New Chat') {
      const tabState = tabStates.find(ts => ts.tab.id === tabId);
      if (tabState && tabState.tab.title === 'New Chat') {
        console.log('🏷️ Updating tab title from chat update:', chat.title);
        setTabStates(prev => prev.map(ts => 
          ts.tab.id === tabId 
            ? { 
                ...ts, 
                tab: { ...ts.tab, title: chat.title }
              }
            : ts
        ));
      }
    }
  }, [tabStates]);

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

  // Sync tab title with backend session description
  const syncTabTitleWithBackend = useCallback(async (tabId: string) => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (!tabState) return;

    // Don't try to sync new sessions that don't exist on the backend yet
    if (tabState.tab.sessionId.startsWith('new_')) {
      console.log('🏷️ Skipping backend sync for new session:', tabState.tab.sessionId);
      return;
    }

    try {
      console.log('🏷️ Syncing tab title with backend for session:', tabState.tab.sessionId);
      const response = await getSession({
        path: { session_id: tabState.tab.sessionId }
      });

      if (response.data) {
        const backendTitle = response.data.description;
        console.log('🏷️ Backend session data:', {
          sessionId: tabState.tab.sessionId,
          description: backendTitle,
          messageCount: response.data.message_count
        });
        
        // Only update if we got a meaningful title from the backend
        if (backendTitle && backendTitle.trim() && backendTitle !== 'New Chat') {
          console.log('🏷️ Updating tab title from backend:', backendTitle);
          setTabStates(prev => prev.map(ts => 
            ts.tab.id === tabId 
              ? { 
                  ...ts, 
                  tab: { ...ts.tab, title: backendTitle },
                  chat: { ...ts.chat, title: backendTitle }
                }
              : ts
          ));
        } else {
          console.log('🏷️ No meaningful title found in backend, keeping current title');
        }
      }
    } catch (error) {
      console.warn('Failed to sync tab title with backend for session:', tabState.tab.sessionId, error);
      // Don't throw - this is a nice-to-have feature
    }
  }, [tabStates]);

  // Update tab title from first message and sync with backend
  const updateTabTitleFromMessage = useCallback(async (tabId: string, message: string | any) => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (!tabState) return;

    // Ensure message is a string and handle different input types
    let messageText: string;
    if (typeof message === 'string') {
      messageText = message;
    } else if (message && typeof message === 'object') {
      // Handle Matrix message objects or other structured messages
      messageText = message.text || message.content || message.body || String(message);
    } else {
      messageText = String(message || '');
    }

    // Generate a meaningful title from the message
    let newTitle = messageText.trim();
    
    // Truncate long messages
    if (newTitle.length > 50) {
      newTitle = newTitle.substring(0, 47) + '...';
    }
    
    // Fallback for empty or very short messages
    if (newTitle.length < 3) {
      newTitle = `Chat ${new Date().toLocaleTimeString()}`;
    }

    console.log('🏷️ Updating tab title from message:', newTitle);

    // Update local state immediately for responsive UI
    setTabStates(prev => prev.map(ts => 
      ts.tab.id === tabId 
        ? { 
            ...ts, 
            tab: { ...ts.tab, title: newTitle },
            chat: { ...ts.chat, title: newTitle }
          }
        : ts
    ));

    // Sync with backend (async, don't wait)
    try {
      await updateSessionDescription({
        path: { session_id: tabState.tab.sessionId },
        body: { description: newTitle }
      });
      console.log('🏷️ Successfully updated backend session description');
    } catch (error) {
      console.warn('Failed to update backend session description:', error);
      // Don't throw - local title update is more important
    }
  }, [tabStates]);

  // Open an existing session in a new tab or switch to it if already open
  const openExistingSession = useCallback((sessionId: string, title?: string) => {
    console.log('📂 Opening existing session:', { sessionId, title });

    // Check if session is already open in a tab
    const existingTab = tabStates.find(ts => ts.tab.sessionId === sessionId);
    if (existingTab) {
      console.log('📂 Session already open, switching to existing tab:', existingTab.tab.id);
      setActiveTabId(existingTab.tab.id);
      return;
    }

    // Create new tab with existing session ID
    const newTab = createNewTab({
      sessionId,
      title: title || 'Loading...',
      isActive: true
    });
    
    const newTabState: TabState = {
      tab: newTab,
      chat: createNewChat(sessionId),
      loadingChat: false
    };
    
    console.log('📂 Creating new tab for existing session:', {
      tabId: newTab.id,
      sessionId,
      title: newTab.title
    });
    
    setTabStates(prev => [...prev, newTabState]);
    setActiveTabId(newTab.id);

    // Try to sync title from backend after tab is created (only for existing sessions)
    if (!sessionId.startsWith('new_')) {
      setTimeout(() => {
        syncTabTitleWithBackend(newTab.id).catch(error => {
          console.warn('Failed to sync title for opened session:', error);
        });
      }, 100);
    }
  }, [tabStates, syncTabTitleWithBackend]);

  // Update session ID for a tab (used when a new session gets a real backend ID)
  const updateSessionId = useCallback((tabId: string, newSessionId: string) => {
    console.log('🔄 Updating session ID for tab:', { tabId, newSessionId });
    
    setTabStates(prev => prev.map(ts => 
      ts.tab.id === tabId 
        ? { 
            ...ts, 
            tab: { ...ts.tab, sessionId: newSessionId },
            chat: { ...ts.chat, sessionId: newSessionId }
          }
        : ts
    ));
  }, []);

  // Sidecar management functions
  const showSidecarView = useCallback((tabId: string, view: TabSidecarView) => {
    console.log('🔧 TabContext: Showing sidecar view for tab:', tabId, 'view:', view.id);
    
    setTabStates(prev => prev.map(ts => {
      if (ts.tab.id !== tabId) return ts;
      
      const currentSidecarState = ts.tab.sidecarState || { activeViews: [], views: [] };
      
      // Add or update the view
      const existingViewIndex = currentSidecarState.views.findIndex(v => v.id === view.id);
      let updatedViews;
      if (existingViewIndex >= 0) {
        updatedViews = [...currentSidecarState.views];
        updatedViews[existingViewIndex] = view;
      } else {
        updatedViews = [...currentSidecarState.views, view];
      }
      
      // Add to active views if not already active
      const updatedActiveViews = currentSidecarState.activeViews.includes(view.id)
        ? currentSidecarState.activeViews
        : [...currentSidecarState.activeViews, view.id];
      
      return {
        ...ts,
        tab: {
          ...ts.tab,
          sidecarState: {
            activeViews: updatedActiveViews,
            views: updatedViews
          }
        }
      };
    }));
  }, []);

  const hideSidecarView = useCallback((tabId: string, viewId: string) => {
    console.log('🔧 TabContext: Hiding sidecar view for tab:', tabId, 'view:', viewId);
    
    setTabStates(prev => prev.map(ts => {
      if (ts.tab.id !== tabId || !ts.tab.sidecarState) return ts;
      
      return {
        ...ts,
        tab: {
          ...ts.tab,
          sidecarState: {
            ...ts.tab.sidecarState,
            activeViews: ts.tab.sidecarState.activeViews.filter(id => id !== viewId)
          }
        }
      };
    }));
  }, []);

  const hideAllSidecarViews = useCallback((tabId: string) => {
    console.log('🔧 TabContext: Hiding all sidecar views for tab:', tabId);
    
    setTabStates(prev => prev.map(ts => {
      if (ts.tab.id !== tabId || !ts.tab.sidecarState) return ts;
      
      return {
        ...ts,
        tab: {
          ...ts.tab,
          sidecarState: {
            ...ts.tab.sidecarState,
            activeViews: []
          }
        }
      };
    }));
  }, []);

  const getSidecarState = useCallback((tabId: string): TabSidecarState | undefined => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    return tabState?.tab.sidecarState;
  }, [tabStates]);

  // Helper function to create sidecar views for specific types
  const showDiffViewer = useCallback((tabId: string, diffContent: string, fileName = 'File', instanceId?: string) => {
    const id = instanceId ? `diff-${instanceId}` : 'diff';
    
    const diffView: TabSidecarView = {
      id,
      title: 'Diff Viewer',
      iconType: 'diff',
      contentType: 'diff',
      contentProps: { diffContent },
      fileName,
      instanceId,
    };
    
    showSidecarView(tabId, diffView);
  }, [showSidecarView]);

  const showLocalhostViewer = useCallback((tabId: string, url = 'http://localhost:3000', title = 'Localhost Viewer', instanceId?: string) => {
    const id = instanceId ? `localhost-${instanceId}` : 'localhost';
    
    const localhostView: TabSidecarView = {
      id,
      title,
      iconType: 'localhost',
      contentType: 'localhost',
      contentProps: { url, title },
      fileName: url,
      instanceId,
    };
    
    showSidecarView(tabId, localhostView);
  }, [showSidecarView]);

  const showFileViewer = useCallback((tabId: string, filePath: string, instanceId?: string) => {
    const fileName = filePath.split('/').pop() || filePath;
    const id = instanceId ? `file-${instanceId}` : 'file';
    
    const fileView: TabSidecarView = {
      id,
      title: 'File Viewer',
      iconType: 'file',
      contentType: 'file',
      contentProps: { path: filePath },
      fileName,
      instanceId,
    };
    
    showSidecarView(tabId, fileView);
  }, [showSidecarView]);

  const showDocumentEditor = useCallback((tabId: string, filePath?: string, initialContent?: string, instanceId?: string) => {
    const fileName = filePath ? filePath.split('/').pop() || filePath : 'Untitled Document';
    const id = instanceId ? `editor-${instanceId}` : 'editor';
    
    const editorView: TabSidecarView = {
      id,
      title: 'Document Editor',
      iconType: 'editor',
      contentType: 'editor',
      contentProps: { path: filePath, content: initialContent },
      fileName,
      instanceId,
    };
    
    showSidecarView(tabId, editorView);
  }, [showSidecarView]);

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
    clearTabState,
    syncTabTitleWithBackend,
    updateTabTitleFromMessage,
    openExistingSession,
    updateSessionId,
    // Sidecar functions
    showSidecarView,
    hideSidecarView,
    hideAllSidecarViews,
    getSidecarState,
    showDiffViewer,
    showLocalhostViewer,
    showFileViewer,
    showDocumentEditor
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
