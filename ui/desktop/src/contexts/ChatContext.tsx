import React, { createContext, useContext, ReactNode } from 'react';
import { ChatType } from '../components/BaseChat';
import { generateSessionId } from '../sessions';
import { Recipe } from '../recipe';
import { useDraftContext } from './DraftContext';

interface ChatContextType {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  resetChat: () => void;
  hasActiveSession: boolean;
  setRecipeConfig: (recipe: Recipe | null) => void;
  clearRecipeConfig: () => void;
  // Draft functionality
  draft: string;
  setDraft: (draft: string) => void;
  clearDraft: () => void;
  // Context identification
  contextKey: string; // 'hub' or 'pair-{sessionId}'
}

const ChatContext = createContext<ChatContextType | undefined>(undefined);

interface ChatProviderProps {
  children: ReactNode;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  contextKey?: string; // Optional context key, defaults to 'hub'
}

export const ChatProvider: React.FC<ChatProviderProps> = ({
  children,
  chat,
  setChat,
  contextKey = 'hub',
}) => {
  const draftContext = useDraftContext();

  // Draft functionality using the app-level DraftContext
  const draft = draftContext.getDraft(contextKey);

  const setDraft = (newDraft: string) => {
    draftContext.setDraft(contextKey, newDraft);
  };

  const clearDraft = () => {
    draftContext.clearDraft(contextKey);
  };

  const resetChat = () => {
    const newSessionId = generateSessionId();
    setChat({
      id: newSessionId,
      title: 'New Chat',
      messages: [],
      messageHistoryIndex: 0,
      recipeConfig: null, // Clear recipe when resetting chat
    });
    // Clear draft when resetting chat
    clearDraft();
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
    draft,
    setDraft,
    clearDraft,
    contextKey,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

export const useChatContext = (): ChatContextType | null => {
  const context = useContext(ChatContext);
  return context || null;
};
