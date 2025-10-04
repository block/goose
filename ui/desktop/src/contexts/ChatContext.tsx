import React, { createContext, useContext, ReactNode } from 'react';
import { ChatType } from '../types/chat';
import { Recipe } from '../recipe';
import { useDraftContext } from './DraftContext';

// TODO(Douwe): We should not need this anymore
export const DEFAULT_CHAT_TITLE = 'New Chat';

interface ChatContextType {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  resetChat: () => void;
  hasActiveSession: boolean;
  setRecipe: (recipe: Recipe | null) => void;
  clearRecipe: () => void;
  // Draft functionality
  draft: string;
  setDraft: (draft: string) => void;
  clearDraft: () => void;
  // Context identification
  contextKey: string; // 'hub' or 'pair-{sessionId}'
  agentWaitingMessage: string | null;
}

const ChatContext = createContext<ChatContextType | undefined>(undefined);

interface ChatProviderProps {
  children: ReactNode;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  contextKey?: string; // Optional context key, defaults to 'hub'
  agentWaitingMessage: string | null;
}

export const ChatProvider: React.FC<ChatProviderProps> = ({
  children,
  chat,
  setChat,
  agentWaitingMessage,
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
    setChat({
      sessionId: '',
      title: DEFAULT_CHAT_TITLE,
      messages: [],
      messageHistoryIndex: 0,
      recipe: null,
      recipeParameters: null,
    });
    clearDraft();
  };

  const setRecipe = (recipe: Recipe | null) => {
    setChat({
      ...chat,
      recipe: recipe,
      recipeParameters: null,
    });
  };

  const clearRecipe = () => {
    setChat({
      ...chat,
      recipe: null,
    });
  };

  const hasActiveSession = chat.messages.length > 0;

  const value: ChatContextType = {
    chat,
    setChat,
    resetChat,
    hasActiveSession,
    setRecipe,
    clearRecipe,
    draft,
    setDraft,
    clearDraft,
    contextKey,
    agentWaitingMessage,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

export const useChatContext = (): ChatContextType | null => {
  const context = useContext(ChatContext);
  return context || null;
};
