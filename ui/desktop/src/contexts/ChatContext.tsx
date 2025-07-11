import React, { createContext, useContext, ReactNode } from 'react';
import { ChatType } from '../components/BaseChat';
import { generateSessionId } from '../sessions';
import { Recipe } from '../recipe';

interface ChatContextType {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  resetChat: () => void;
  hasActiveSession: boolean;
  setRecipeConfig: (recipe: Recipe | null) => void;
  clearRecipeConfig: () => void;
}

const ChatContext = createContext<ChatContextType | undefined>(undefined);

interface ChatProviderProps {
  children: ReactNode;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
}

export const ChatProvider: React.FC<ChatProviderProps> = ({ children, chat, setChat }) => {
  const resetChat = () => {
    const newSessionId = generateSessionId();
    setChat({
      id: newSessionId,
      title: 'New Chat',
      messages: [],
      messageHistoryIndex: 0,
      recipeConfig: null, // Clear recipe when resetting chat
    });
  };

  const setRecipeConfig = (recipe: Recipe | null) => {
    setChat({
      ...chat,
      recipeConfig: recipe,
    });
  };

  const clearRecipeConfig = () => {
    setChat({
      ...chat,
      recipeConfig: null,
    });
  };

  const hasActiveSession = chat.messages.length > 0;

  const value: ChatContextType = {
    chat,
    setChat,
    resetChat,
    hasActiveSession,
    setRecipeConfig,
    clearRecipeConfig,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

export const useChatContext = (): ChatContextType => {
  const context = useContext(ChatContext);
  if (context === undefined) {
    throw new Error('useChatContext must be used within a ChatProvider');
  }
  return context;
};
