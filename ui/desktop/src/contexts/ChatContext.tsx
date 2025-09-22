import React, { createContext, useContext, ReactNode } from 'react';
import { ChatType } from '../types/chat';
import { generateSessionId } from '../sessions';
import { Recipe } from '../recipe';
import { useDraftContext } from './DraftContext';

// TODO(Douwe): We should not need this anymore
export const DEFAULT_CHAT_TITLE = 'New Chat';

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
    const newSessionId = generateSessionId();
    setChat({
      sessionId: newSessionId,
      title: DEFAULT_CHAT_TITLE,
      messages: [],
      messageHistoryIndex: 0,
      recipeConfig: null, // Clear recipe when resetting chat
      recipeParameters: null, // Clear  when resetting chat
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
    agentWaitingMessage,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

export const useChatContext = (): ChatContextType | null => {
  const context = useContext(ChatContext);
  return context || null;
};
