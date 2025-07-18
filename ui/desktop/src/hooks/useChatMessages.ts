import { createContext, useContext } from 'react';
import { Message } from '../types/message';

interface ChatContextType {
  messages: Message[];
  sendMessage: (message: string) => Promise<void>;
}

export const ChatContext = createContext<ChatContextType | undefined>(undefined);

export function useChatMessages() {
  const context = useContext(ChatContext);
  if (!context) {
    throw new Error('useChatMessages must be used within a ChatContextProvider');
  }
  return context;
}
