import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react';
import { ChatType, CachedSession } from '../types/chat';

interface ChatStateContextType {
  // Session cache - global lookup for all cached sessions
  sessionCache: Map<string, CachedSession>;

  // Active chats - keyed by route or session identifier
  activeChats: Map<string, ChatType>;

  // Last active session ID for the pair route
  lastActiveSessionId: string | null;

  // Get a chat by key (route/session)
  getChat: (key: string) => ChatType | undefined;

  // Set/update a chat
  setChat: (key: string, chat: ChatType | ((prev: ChatType) => ChatType)) => void;

  // Update session cache
  updateSessionCache: (sessionId: string, cachedSession: CachedSession) => void;

  // Get cached session
  getCachedSession: (sessionId: string) => CachedSession | undefined;

  // Set the last active session ID
  setLastActiveSessionId: (sessionId: string | null) => void;

  // Clear a specific chat
  clearChat: (key: string) => void;

  // Clear all chats
  clearAllChats: () => void;
}

const ChatStateContext = createContext<ChatStateContextType | undefined>(undefined);

export const ChatStateProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [sessionCache, setSessionCache] = useState<Map<string, CachedSession>>(() => {
    console.log('🚀 [CHAT STATE CONTEXT] Creating global session cache');
    return new Map();
  });

  const [activeChats, setActiveChats] = useState<Map<string, ChatType>>(() => {
    console.log('🚀 [CHAT STATE CONTEXT] Creating active chats map');
    return new Map();
  });

  const [lastActiveSessionId, setLastActiveSessionId] = useState<string | null>(null);

  const getChat = useCallback(
    (key: string): ChatType | undefined => {
      return activeChats.get(key);
    },
    [activeChats]
  );

  const setChat = useCallback(
    (key: string, chatOrUpdater: ChatType | ((prev: ChatType) => ChatType)) => {
      setActiveChats((prev) => {
        const newMap = new Map(prev);
        const existingChat = newMap.get(key);

        const newChat =
          typeof chatOrUpdater === 'function'
            ? chatOrUpdater(existingChat || createDefaultChat())
            : chatOrUpdater;

        newMap.set(key, newChat);

        console.log(`💬 [CHAT STATE] Updated chat for key: ${key}`, {
          sessionId: newChat.sessionId?.slice(0, 8),
          messageCount: newChat.messages?.length || 0,
          totalActiveChats: newMap.size,
        });

        return newMap;
      });
    },
    []
  );

  const updateSessionCache = useCallback((sessionId: string, cachedSession: CachedSession) => {
    setSessionCache((prev) => {
      const newCache = new Map(prev);
      const wasInCache = newCache.has(sessionId);
      newCache.set(sessionId, cachedSession);

      console.group('💾 [CACHE UPDATE] Storing session data');
      console.log('Session ID:', sessionId.slice(0, 8));
      console.log('Action:', wasInCache ? 'Updated existing' : 'Added new');
      console.log('Message count:', cachedSession.messages.length);
      console.log('Cache size:', newCache.size);
      console.log(
        'Cache keys:',
        Array.from(newCache.keys()).map((k) => k.slice(0, 8))
      );
      console.log('Session description:', cachedSession.session.description);
      console.groupEnd();

      return newCache;
    });
  }, []);

  // Use a ref to avoid recreating the getter function on every render
  const sessionCacheRef = React.useRef(sessionCache);
  sessionCacheRef.current = sessionCache;

  const getCachedSession = useCallback((sessionId: string): CachedSession | undefined => {
    return sessionCacheRef.current.get(sessionId);
  }, []); // Empty deps - function never changes identity

  const clearChat = useCallback((key: string) => {
    setActiveChats((prev) => {
      const newMap = new Map(prev);
      newMap.delete(key);
      console.log(`🗑️ [CHAT STATE] Cleared chat for key: ${key}, remaining: ${newMap.size}`);
      return newMap;
    });
  }, []);

  const clearAllChats = useCallback(() => {
    setActiveChats(new Map());
    console.log('🗑️ [CHAT STATE] Cleared all active chats');
  }, []);

  const value: ChatStateContextType = {
    sessionCache,
    activeChats,
    lastActiveSessionId,
    getChat,
    setChat,
    updateSessionCache,
    getCachedSession,
    setLastActiveSessionId,
    clearChat,
    clearAllChats,
  };

  return <ChatStateContext.Provider value={value}>{children}</ChatStateContext.Provider>;
};

export const useChatState = () => {
  const context = useContext(ChatStateContext);
  if (context === undefined) {
    throw new Error('useChatState must be used within a ChatStateProvider');
  }
  return context;
};

// Helper function to create a default chat
function createDefaultChat(): ChatType {
  return {
    sessionId: '',
    title: 'New Chat',
    messages: [],
    messageHistoryIndex: 0,
    recipe: null,
    recipeParameters: null,
  };
}
