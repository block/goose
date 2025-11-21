import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react';
import { Tab, TabSidecarState, TabSidecarView } from '../components/TabBar';
import { ChatType } from '../types/chat';
import { generateSessionId } from '../utils/sessionUtils';
import { getSession, updateSessionDescription, startAgent, deleteSession } from '../api';
import { sessionMappingService } from '../services/SessionMappingService';

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
  // Matrix-specific methods
  openMatrixChat: (roomId: string, senderId: string) => void;
  morphTabToMatrix: (tabId: string, roomId: string, recipientId: string, roomTitle?: string) => Promise<void>;
  createBackendSession: (tabId: string) => Promise<string>;
  // Sidecar management functions
  showSidecarView: (tabId: string, view: TabSidecarView) => void;
  hideSidecarView: (tabId: string, viewId: string) => void;
  hideAllSidecarViews: (tabId: string) => void;
  getSidecarState: (tabId: string) => TabSidecarState | undefined;
  showDiffViewer: (tabId: string, diffContent: string, fileName?: string, instanceId?: string) => void;
  showLocalhostViewer: (tabId: string, url?: string, title?: string, instanceId?: string) => void;
  showWebViewer: (tabId: string, url?: string, title?: string, instanceId?: string) => void;
  showFileViewer: (tabId: string, filePath: string, instanceId?: string) => void;
  showDocumentEditor: (tabId: string, filePath?: string, initialContent?: string, instanceId?: string) => void;
}

const TabContext = createContext<TabContextType | undefined>(undefined);

const TAB_STATE_STORAGE_KEY = 'goose-tab-state';

const createNewTab = (overrides: Partial<Tab> = {}): Tab => {
  // Generate a truly unique tab ID
  const timestamp = Date.now();
  const random = Math.random().toString(36).substr(2, 9);
  
  // For new tabs, we'll create the backend session immediately
  // The sessionId will be set after the backend session is created
  return {
    id: `tab-${timestamp}-${random}`,
    title: 'New Chat',
    type: 'chat',
    sessionId: '', // Will be set immediately after creation
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
  // Start with a temporary session - we'll create the backend session after component mounts
  const firstTab = createNewTab({ sessionId: `temp_initial_${Date.now()}`, isActive: true });
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
          // CRITICAL FIX: Validate and sanitize restored tab state
          const sanitizedTabs = parsed.map((tabState: any) => {
            const tab = tabState.tab;
            
            // Validate Matrix tab properties (HYBRID APPROACH)
            if (tab.type === 'matrix') {
              // Matrix tabs must have matrixRoomId - sessionId can be any backend session ID
              if (!tab.matrixRoomId) {
                console.warn('🚨 Invalid Matrix tab detected during restore (missing matrixRoomId), converting to regular chat:', tab);
                return {
                  ...tabState,
                  tab: {
                    ...tab,
                    type: 'chat',
                    matrixRoomId: undefined,
                    matrixRecipientId: undefined,
                    // Keep the sessionId as-is since it's a valid backend session ID
                  }
                };
              }
            } else {
              // Regular chat tabs must NOT have Matrix properties
              if (tab.matrixRoomId || tab.matrixRecipientId) {
                console.warn('🚨 Regular chat tab with Matrix properties detected during restore, sanitizing:', tab);
                return {
                  ...tabState,
                  tab: {
                    ...tab,
                    type: 'chat',
                    matrixRoomId: undefined,
                    matrixRecipientId: undefined,
                    // Keep the sessionId as-is - it might be a valid backend session ID
                  }
                };
              }
            }
            
            return tabState;
          });
          
          console.log('🔄 Restored and sanitized tab states:', sanitizedTabs.map(ts => ({
            id: ts.tab.id,
            type: ts.tab.type,
            sessionId: ts.tab.sessionId,
            matrixRoomId: ts.tab.matrixRoomId,
            title: ts.tab.title
          })));
          
          return sanitizedTabs;
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

  // Create backend sessions for any temporary sessions after component mounts
  useEffect(() => {
    const createBackendSessionsForTempTabs = async () => {
      const tempTabs = tabStates.filter(ts => ts.tab.sessionId.startsWith('temp_'));
      
      for (const tabState of tempTabs) {
        try {
          console.log('🔄 Converting temporary session to backend session:', tabState.tab.sessionId);
          await createBackendSession(tabState.tab.id);
        } catch (error) {
          console.error('❌ Failed to create backend session for temp tab:', tabState.tab.id, error);
        }
      }
    };

    // Only run this once after initial mount, with a small delay to ensure everything is initialized
    const timeoutId = setTimeout(createBackendSessionsForTempTabs, 1000);
    
    return () => clearTimeout(timeoutId);
  }, []); // Empty dependency array - only run once on mount

  const handleTabClick = useCallback((tabId: string) => {
    console.log('🖱️ TabContext: handleTabClick called:', { 
      clickedTabId: tabId, 
      currentActiveTabId: activeTabId,
      willChange: tabId !== activeTabId
    });
    console.log('🖱️ TabContext: About to call setActiveTabId with:', tabId);
    setActiveTabId(tabId);
    console.log('🖱️ TabContext: setActiveTabId called');
  }, [activeTabId]);

  const handleNewTab = useCallback(async () => {
    try {
      console.log('🆕 Creating new tab with immediate backend session');
      
      // Create a new backend session immediately using startAgent
      const response = await startAgent({
        body: {
          working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
        }
      });

      if (!response.data?.id) {
        throw new Error('Failed to create backend session - no session ID returned');
      }

      const sessionId = response.data.id;
      console.log('✅ Created backend session for new tab:', sessionId);

      // Create the tab with the real backend session ID
      const newTab = createNewTab({ sessionId });
      const newTabState: TabState = {
        tab: newTab,
        chat: createNewChat(sessionId),
        loadingChat: false
      };
      
      setTabStates(prev => [...prev, newTabState]);
      setActiveTabId(newTab.id);
      
      console.log('✅ New tab created successfully:', { tabId: newTab.id, sessionId });
    } catch (error) {
      console.error('❌ Failed to create new tab with backend session:', error);
      
      // Fallback: create tab with temporary session ID and try to create backend session later
      const newTab = createNewTab({ sessionId: `temp_${Date.now()}` });
      const newTabState: TabState = {
        tab: newTab,
        chat: createNewChat(newTab.sessionId),
        loadingChat: false
      };
      
      setTabStates(prev => [...prev, newTabState]);
      setActiveTabId(newTab.id);
      
      // Try to create backend session in the background
      setTimeout(async () => {
        try {
          const backendSessionId = await createBackendSession(newTab.id);
          console.log('✅ Successfully created backend session for fallback tab:', backendSessionId);
        } catch (bgError) {
          console.error('❌ Failed to create backend session in background:', bgError);
        }
      }, 1000);
    }
  }, []);

  const handleTabClose = useCallback(async (tabId: string) => {
    // Get the tab being closed for cleanup logic
    const closingTab = tabStates.find(ts => ts.tab.id === tabId);
    
    // Perform session cleanup if needed
    if (closingTab) {
      console.log('🗑️ Closing tab:', { tabId, sessionId: closingTab.tab.sessionId, hasMessages: closingTab.chat.messages.length > 0 });
      
      // Check if the session should be cleaned up (empty sessions only)
      const shouldCleanupSession = closingTab.chat.messages.length === 0 && 
                                  closingTab.tab.sessionId && 
                                  !closingTab.tab.sessionId.startsWith('temp_') &&
                                  !closingTab.tab.sessionId.startsWith('new_');
      
      if (shouldCleanupSession) {
        try {
          console.log('🧹 Cleaning up empty session:', closingTab.tab.sessionId);
          await deleteSession({
            path: { session_id: closingTab.tab.sessionId }
          });
          console.log('✅ Successfully deleted empty session:', closingTab.tab.sessionId);
        } catch (error) {
          console.warn('⚠️ Failed to delete empty session (may not exist):', closingTab.tab.sessionId, error);
          // Don't block tab closing if session deletion fails
        }
      } else if (closingTab.chat.messages.length > 0) {
        console.log('💾 Preserving session with messages:', closingTab.tab.sessionId);
      } else {
        console.log('⏭️ Skipping cleanup for temporary/new session:', closingTab.tab.sessionId);
      }
    }

    // Update tab states
    setTabStates(prev => {
      const newStates = prev.filter(ts => ts.tab.id !== tabId);
      
      // If we're closing the active tab, activate another one
      if (tabId === activeTabId && newStates.length > 0) {
        const tabIndex = prev.findIndex(ts => ts.tab.id === tabId);
        const nextActiveIndex = Math.min(tabIndex, newStates.length - 1);
        setActiveTabId(newStates[nextActiveIndex].tab.id);
      }
      
      // If this was the last tab, create a new one with immediate backend session
      if (newStates.length === 0) {
        // Create a new tab immediately - this will trigger handleNewTab logic
        handleNewTab();
        return prev; // Return current state, handleNewTab will update it
      }
      
      return newStates;
    });
  }, [activeTabId, tabStates, handleNewTab]);

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
          // Apply the same sanitization logic as in initial load
          const sanitizedTabs = parsed.map((tabState: any) => {
            const tab = tabState.tab;
            
            // Validate Matrix tab properties (HYBRID APPROACH)
            if (tab.type === 'matrix') {
              // Matrix tabs must have matrixRoomId - sessionId can be any backend session ID
              if (!tab.matrixRoomId) {
                console.warn('🚨 Invalid Matrix tab detected during manual restore (missing matrixRoomId), converting to regular chat:', tab);
                return {
                  ...tabState,
                  tab: {
                    ...tab,
                    type: 'chat',
                    matrixRoomId: undefined,
                    matrixRecipientId: undefined,
                    // Keep the sessionId as-is since it's a valid backend session ID
                  }
                };
              }
            } else {
              // Regular chat tabs must NOT have Matrix properties
              if (tab.matrixRoomId || tab.matrixRecipientId) {
                console.warn('🚨 Regular chat tab with Matrix properties detected during manual restore, sanitizing:', tab);
                return {
                  ...tabState,
                  tab: {
                    ...tab,
                    type: 'chat',
                    matrixRoomId: undefined,
                    matrixRecipientId: undefined,
                    // Keep the sessionId as-is - it might be a valid backend session ID
                  }
                };
              }
            }
            
            return tabState;
          });
          
          setTabStates(sanitizedTabs);
          const activeTab = sanitizedTabs.find((ts: TabState) => ts.tab.isActive);
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
  const openExistingSession = useCallback(async (sessionId: string, title?: string) => {
    console.log('📂 Opening existing session:', { sessionId, title });

    // Check if session is already open in a tab
    const existingTab = tabStates.find(ts => ts.tab.sessionId === sessionId);
    if (existingTab) {
      console.log('📂 Session already open, switching to existing tab:', existingTab.tab.id);
      setActiveTabId(existingTab.tab.id);
      return;
    }

    // CRITICAL: Check if this is a Matrix session by looking up Matrix metadata
    // BUT ONLY if the user explicitly requested a Matrix session
    let isMatrixSession = false;
    let matrixMetadata: any = null;
    
    // SECURITY: Only check for Matrix metadata if this is NOT a solo session creation
    // Solo sessions should NEVER be treated as Matrix sessions unless explicitly requested
    const isExplicitMatrixRequest = title && title.includes('Matrix');
    
    if (!sessionId.startsWith('new_') && isExplicitMatrixRequest) {
      try {
        console.log('🔍 Checking if session is Matrix session (explicit Matrix request):', sessionId);
        matrixMetadata = await sessionMappingService.getMatrixMetadataForBackendSession(sessionId);
        if (matrixMetadata) {
          isMatrixSession = true;
          console.log('✅ Session is Matrix session, metadata:', matrixMetadata);
        } else {
          console.log('ℹ️ Session is regular session, no Matrix metadata found');
        }
      } catch (error) {
        console.warn('⚠️ Failed to check Matrix metadata for session:', error);
      }
    } else {
      console.log('🚫 Skipping Matrix metadata check for solo session:', {
        sessionId,
        title,
        isExplicitMatrixRequest,
        startsWithNew: sessionId.startsWith('new_')
      });
    }

    // Create new tab with appropriate properties based on session type
    let newTab: Tab;
    if (isMatrixSession && matrixMetadata) {
      // CRITICAL: Use the actual backend session ID, not matrix_ format
      // The Matrix context comes from the tab properties, not the sessionId
      const matrixTitle = matrixMetadata.roomName || title || `Matrix Chat ${matrixMetadata.roomId.substring(1, 8)}`;
      
      console.log('📱 Creating Matrix tab from backend session:', {
        backendSessionId: sessionId, // Use actual backend session ID
        matrixRoomId: matrixMetadata.roomId,
        matrixRecipientId: matrixMetadata.recipientId,
        title: matrixTitle
      });
      
      newTab = createNewTab({
        sessionId: sessionId, // Use actual backend session ID for API calls
        title: matrixTitle,
        type: 'matrix',
        matrixRoomId: matrixMetadata.roomId,
        matrixRecipientId: matrixMetadata.recipientId,
        isActive: true
      });
    } else {
      // Create regular chat tab
      newTab = createNewTab({
        sessionId,
        title: title || 'Loading...',
        isActive: true
      });
    }
    
    const newTabState: TabState = {
      tab: newTab,
      chat: createNewChat(newTab.sessionId),
      loadingChat: false
    };
    
    console.log('📂 Creating new tab for existing session:', {
      tabId: newTab.id,
      sessionId: newTab.sessionId,
      type: newTab.type,
      matrixRoomId: newTab.matrixRoomId,
      title: newTab.title,
      isMatrixSession
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
    
    // If this is a web viewer, trigger explicit cleanup
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (tabState?.tab.sidecarState) {
      const view = tabState.tab.sidecarState.views.find(v => v.id === viewId);
      if (view && view.contentType === 'web') {
        console.log('🔧 TabContext: Triggering cleanup for web viewer:', viewId);
        // Dispatch a custom event to trigger WebBrowser cleanup
        window.dispatchEvent(new CustomEvent('sidecar-web-view-closing', { 
          detail: { tabId, viewId } 
        }));
      }
    }
    
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
  }, [tabStates]);

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

  const showWebViewer = useCallback((tabId: string, url = 'https://google.com', title = 'Web Browser', instanceId?: string) => {
    const id = instanceId ? `web-${instanceId}` : 'web';
    
    const webView: TabSidecarView = {
      id,
      title,
      iconType: 'web',
      contentType: 'web',
      contentProps: { url, title },
      fileName: url,
      instanceId,
    };
    
    showSidecarView(tabId, webView);
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

  // Open a Matrix chat in a new tab or switch to it if already open
  const openMatrixChat = useCallback(async (roomId: string, senderId: string) => {
    console.log('📱 TabContext: Opening Matrix chat for room:', roomId, 'sender:', senderId);

    // Check if we already have a tab for this Matrix room
    const existingTab = tabStates.find(ts => 
      ts.tab.type === 'matrix' && ts.tab.matrixRoomId === roomId
    );
    
    if (existingTab) {
      console.log('📱 Matrix room already open in tab, switching to it:', existingTab.tab.id);
      setActiveTabId(existingTab.tab.id);
      return;
    }

    // Create a temporary tab with loading state first for immediate feedback
    const senderName = senderId.split(':')[0].substring(1);
    const tabTitle = `Chat with ${senderName}`;
    
    const tempTab = createNewTab({
      sessionId: `temp_matrix_${Date.now()}`, // Temporary ID until we get the real one
      title: tabTitle,
      type: 'matrix',
      matrixRoomId: roomId,
      matrixRecipientId: senderId,
      isActive: true
    });
    
    const tempTabState: TabState = {
      tab: tempTab,
      chat: {
        sessionId: tempTab.sessionId,
        title: tabTitle,
        messages: [],
        messageHistoryIndex: 0,
        recipeConfig: null,
        aiEnabled: false,
      },
      loadingChat: true // Show loading state
    };
    
    // Add the loading tab immediately
    setTabStates(prev => [...prev, tempTabState]);
    setActiveTabId(tempTab.id);
    
    console.log('📱 Created temporary loading tab:', tempTab.id);

    // Get or create the backend session for this Matrix room (async)
    let backendSessionId = sessionMappingService.getGooseSessionId(roomId);
    
    if (!backendSessionId) {
      console.log('📱 No existing mapping found, creating new Matrix session mapping');
      try {
        const roomTitle = `DM with ${senderName}`;
        
        // Create a backend session for this Matrix room
        const mapping = await sessionMappingService.createMappingWithBackendSession(
          roomId, 
          [], 
          roomTitle, 
          senderId
        );
        backendSessionId = mapping.gooseSessionId;
        console.log('✅ Created new backend session for Matrix room:', backendSessionId);
      } catch (error) {
        console.error('❌ Failed to create backend session for Matrix room:', error);
        // Keep the temporary session ID - this won't have backend persistence
        backendSessionId = tempTab.sessionId;
      }
    } else {
      console.log('📱 Found existing backend session for Matrix room:', backendSessionId);
    }

    // Update the tab with the real backend session ID and remove loading state
    console.log('📱 Updating tab with backend session:', {
      tabId: tempTab.id,
      backendSessionId,
      roomId,
      senderId
    });
    
    setTabStates(prev => prev.map(ts => 
      ts.tab.id === tempTab.id
        ? {
            ...ts,
            tab: { ...ts.tab, sessionId: backendSessionId },
            chat: { ...ts.chat, sessionId: backendSessionId },
            loadingChat: false // Remove loading state
          }
        : ts
    ));
  }, [tabStates]);

  // Create a backend session for a tab (converts new_ session to real backend session)
  const createBackendSession = useCallback(async (tabId: string): Promise<string> => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (!tabState) {
      throw new Error(`Tab not found: ${tabId}`);
    }

    // If already has a backend session (not temporary), return it
    if (!tabState.tab.sessionId.startsWith('temp_') && !tabState.tab.sessionId.startsWith('new_')) {
      console.log('🏗️ Tab already has backend session:', tabState.tab.sessionId);
      return tabState.tab.sessionId;
    }

    try {
      console.log('🏗️ Creating backend session for tab:', tabId);
      
      // Create a new backend session using startAgent
      const response = await startAgent({
        body: {
          working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
        }
      });

      if (!response.data?.id) {
        throw new Error('Failed to create backend session - no session ID returned');
      }

      const newSessionId = response.data.id;
      console.log('✅ Created backend session:', newSessionId);

      // Update the tab with the new session ID
      setTabStates(prev => prev.map(ts => 
        ts.tab.id === tabId 
          ? { 
              ...ts, 
              tab: { ...ts.tab, sessionId: newSessionId },
              chat: { ...ts.chat, sessionId: newSessionId }
            }
          : ts
      ));

      return newSessionId;
    } catch (error) {
      console.error('❌ Failed to create backend session:', error);
      throw error;
    }
  }, [tabStates]);

  // Morph a regular chat tab into a Matrix session
  const morphTabToMatrix = useCallback(async (
    tabId: string, 
    roomId: string, 
    recipientId: string, 
    roomTitle?: string
  ): Promise<void> => {
    const tabState = tabStates.find(ts => ts.tab.id === tabId);
    if (!tabState) {
      throw new Error(`Tab not found: ${tabId}`);
    }

    console.log('🔄 Morphing tab to Matrix:', {
      tabId,
      currentSessionId: tabState.tab.sessionId,
      roomId,
      recipientId,
      roomTitle
    });

    try {
      // Ensure we have a backend session first
      let backendSessionId = tabState.tab.sessionId;
      if (backendSessionId.startsWith('temp_') || backendSessionId.startsWith('new_')) {
        console.log('🏗️ Creating backend session before morphing to Matrix');
        backendSessionId = await createBackendSession(tabId);
      }

      // Create the Matrix mapping in the session mapping service
      await sessionMappingService.createMapping(roomId, backendSessionId, roomTitle || `Matrix Chat`, recipientId);
      console.log('✅ Created Matrix mapping for backend session:', backendSessionId);

      // Update the tab to Matrix type with Matrix properties
      const matrixTitle = roomTitle || `Matrix Chat ${roomId.substring(1, 8)}`;
      
      setTabStates(prev => prev.map(ts => 
        ts.tab.id === tabId 
          ? { 
              ...ts, 
              tab: { 
                ...ts.tab, 
                type: 'matrix',
                matrixRoomId: roomId,
                matrixRecipientId: recipientId,
                title: matrixTitle,
                sessionId: backendSessionId // Keep the backend session ID
              },
              chat: { 
                ...ts.chat, 
                title: matrixTitle,
                sessionId: backendSessionId,
                aiEnabled: false // Matrix chats typically have AI disabled
              }
            }
          : ts
      ));

      console.log('✅ Successfully morphed tab to Matrix:', {
        tabId,
        backendSessionId,
        roomId,
        recipientId,
        title: matrixTitle
      });

    } catch (error) {
      console.error('❌ Failed to morph tab to Matrix:', error);
      throw error;
    }
  }, [tabStates, createBackendSession]);

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
    // Matrix-specific methods
    openMatrixChat,
    morphTabToMatrix,
    createBackendSession,
    // Sidecar functions
    showSidecarView,
    hideSidecarView,
    hideAllSidecarViews,
    getSidecarState,
    showDiffViewer,
    showLocalhostViewer,
    showWebViewer,
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
