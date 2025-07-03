import React, { createContext, useContext, useState } from 'react';

interface ChatInputContextType {
  inputValue: string;
  setInputValue: (value: string) => void;
}

const ChatInputContext = createContext<ChatInputContextType | undefined>(undefined);

export function ChatInputProvider({ children }: { children: React.ReactNode }) {
  const [inputValue, setInputValue] = useState('');

  return (
    <ChatInputContext.Provider value={{ inputValue, setInputValue }}>
      {children}
    </ChatInputContext.Provider>
  );
}

export function useChatInput() {
  const context = useContext(ChatInputContext);
  if (context === undefined) {
    throw new Error('useChatInput must be used within a ChatInputProvider');
  }
  return context;
}
