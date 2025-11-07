import { useState, useEffect, useCallback } from 'react';
import { Session, getSession, startAgent } from '../../api';
import { ChatState } from '../../types/chatState';

export type SessionStatus = 'waiting' | 'working' | 'done' | 'error';

interface OpenSession {
  sessionId: string;
  session: Session | null;
  isLoading: boolean;
  chatState?: ChatState;
}

interface UseMultiChatReturn {
  openSessions: OpenSession[];
  activeSessionId: string | null;
  setActiveSessionId: (sessionId: string) => void;
  openSession: (sessionId: string) => void;
  closeSession: (sessionId: string) => void;
  createNewSession: () => void;
  reorderSessions: (fromIndex: number, toIndex: number) => void;
  updateSessionChatState: (sessionId: string, chatState: ChatState) => void;
}

const STORAGE_KEY = 'goose_multi_chat_sessions';
const MAX_OPEN_SESSIONS = 10;

export const useMultiChat = (): UseMultiChatReturn => {
  const [openSessions, setOpenSessions] = useState<OpenSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  // Load sessions from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        const { sessionIds, activeId } = JSON.parse(stored);
        if (Array.isArray(sessionIds) && sessionIds.length > 0) {
          // Filter out any invalid session IDs (null, undefined, empty string)
          const validSessionIds = sessionIds.filter((id: string) => id && typeof id === 'string' && id.trim().length > 0);
          
          if (validSessionIds.length === 0) {
            console.log('No valid session IDs found in localStorage, clearing');
            localStorage.removeItem(STORAGE_KEY);
            return;
          }
          
          // Initialize with session IDs, will load details async
          setOpenSessions(
            validSessionIds.map((id: string) => ({
              sessionId: id,
              session: null,
              isLoading: true,
            }))
          );
          
          // Set active ID, ensuring it's valid
          const validActiveId = activeId && validSessionIds.includes(activeId) ? activeId : validSessionIds[0];
          setActiveSessionId(validActiveId);
          
          // Load session details
          validSessionIds.forEach((id: string) => loadSessionDetails(id));
        }
      } catch (error) {
        console.error('Failed to parse stored sessions:', error);
        localStorage.removeItem(STORAGE_KEY);
      }
    }
  }, []);

  // Persist sessions to localStorage whenever they change
  useEffect(() => {
    if (openSessions.length > 0) {
      const sessionIds = openSessions.map(s => s.sessionId);
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ sessionIds, activeId: activeSessionId })
      );
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [openSessions, activeSessionId]);

  // Load session details
  const loadSessionDetails = useCallback(async (sessionId: string) => {
    try {
      const response = await getSession<true>({
        path: { session_id: sessionId },
        throwOnError: true,
      });
      
      setOpenSessions(prev =>
        prev.map(s =>
          s.sessionId === sessionId
            ? { ...s, session: response.data, isLoading: false }
            : s
        )
      );
    } catch (error) {
      console.error(`Failed to load session ${sessionId}:`, error);
      // Keep the session but mark as failed
      setOpenSessions(prev =>
        prev.map(s =>
          s.sessionId === sessionId
            ? { ...s, isLoading: false }
            : s
        )
      );
    }
  }, []);

  // Open a session (add to tabs if not already open)
  const openSession = useCallback((sessionId: string) => {
    setOpenSessions(prev => {
      // Check if already open
      if (prev.some(s => s.sessionId === sessionId)) {
        return prev;
      }

      // Limit number of open sessions
      if (prev.length >= MAX_OPEN_SESSIONS) {
        console.warn(`Maximum ${MAX_OPEN_SESSIONS} sessions can be open at once`);
        return prev;
      }

      // Add new session
      const newSession: OpenSession = {
        sessionId,
        session: null,
        isLoading: true,
      };

      // Load details async
      loadSessionDetails(sessionId);

      return [...prev, newSession];
    });

    setActiveSessionId(sessionId);
  }, [loadSessionDetails]);

  // Close a session
  const closeSession = useCallback((sessionId: string) => {
    setOpenSessions(prev => {
      const filtered = prev.filter(s => s.sessionId !== sessionId);
      
      // If closing the active session, switch to another
      if (sessionId === activeSessionId) {
        const currentIndex = prev.findIndex(s => s.sessionId === sessionId);
        const newActiveIndex = currentIndex > 0 ? currentIndex - 1 : 0;
        const newActive = filtered[newActiveIndex];
        
        if (newActive) {
          setActiveSessionId(newActive.sessionId);
        } else {
          setActiveSessionId(null);
        }
      }

      return filtered;
    });
  }, [activeSessionId]);

  // Create a new session
  const createNewSession = useCallback(async () => {
    console.log('createNewSession called, current sessions:', openSessions.length);
    
    // Check if we've hit the limit
    if (openSessions.length >= MAX_OPEN_SESSIONS) {
      console.warn(`Maximum ${MAX_OPEN_SESSIONS} sessions can be open at once`);
      return;
    }

    try {
      console.log('Calling startAgent...');
      // Get the working directory from config
      const workingDir = window.appConfig?.get('GOOSE_WORKING_DIR') as string || process.cwd();
      
      // Create a new session on the backend
      const response = await startAgent<true>({
        body: {
          messages: [],
          working_dir: workingDir,
        },
        throwOnError: true,
      });

      console.log('startAgent response:', response);
      console.log('startAgent response.data:', response.data);
      console.log('startAgent response keys:', Object.keys(response));
      console.log('startAgent response.data keys:', response.data ? Object.keys(response.data) : 'no data');
      
      const newSessionId = response.data?.session_id || response.data?.id;
      console.log('New session ID:', newSessionId);

      if (!newSessionId) {
        console.error('No session ID returned from startAgent');
        console.error('Full response:', JSON.stringify(response, null, 2));
        return;
      }

      // Use React 18's automatic batching by wrapping in a single function
      // Both state updates will be batched together
      const newSession: OpenSession = {
        sessionId: newSessionId,
        session: null,
        isLoading: true,
      };

      // Add the new session to the list
      setOpenSessions(prev => {
        const updated = [...prev, newSession];
        console.log('Updated open sessions:', updated);
        return updated;
      });

      // Set as active session
      console.log('Setting active session ID to:', newSessionId);
      setActiveSessionId(newSessionId);

      // Load session details
      loadSessionDetails(newSessionId);
    } catch (error) {
      console.error('Failed to create new session:', error);
    }
  }, [openSessions.length, loadSessionDetails]);

  // Reorder sessions (for drag and drop)
  const reorderSessions = useCallback((fromIndex: number, toIndex: number) => {
    setOpenSessions(prev => {
      const newSessions = [...prev];
      const [removed] = newSessions.splice(fromIndex, 1);
      newSessions.splice(toIndex, 0, removed);
      return newSessions;
    });
  }, []);

  // Update the chat state for a specific session
  const updateSessionChatState = useCallback((sessionId: string, chatState: ChatState) => {
    setOpenSessions(prev =>
      prev.map(s =>
        s.sessionId === sessionId
          ? { ...s, chatState }
          : s
      )
    );
  }, []);

  return {
    openSessions,
    activeSessionId,
    setActiveSessionId,
    openSession,
    closeSession,
    createNewSession,
    reorderSessions,
    updateSessionChatState,
  };
};
