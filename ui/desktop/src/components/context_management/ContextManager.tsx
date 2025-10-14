import React, { createContext, useContext, useState, useCallback } from 'react';
import { manageContextFromBackend } from './index';
import { Message } from '../../api';

// Define the context management interface
interface ContextManagerState {
  isCompacting: boolean;
  compactionError: string | null;
}

interface ContextManagerActions {
  handleAutoCompaction: (
    messages: Message[],
    setMessages: (messages: Message[]) => void,
    append: (message: Message) => void,
    sessionId: string
  ) => Promise<void>;
  handleManualCompaction: (
    messages: Message[],
    setMessages: (messages: Message[]) => void,
    append?: (message: Message) => void,
    sessionId?: string
  ) => Promise<void>;
  hasCompactionMarker: (message: Message) => boolean;
}

// Create the context
const ContextManagerContext = createContext<
  (ContextManagerState & ContextManagerActions) | undefined
>(undefined);

// Create the provider component
export const ContextManagerProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [isCompacting, setIsCompacting] = useState<boolean>(false);
  const [compactionError, setCompactionError] = useState<string | null>(null);

  const performCompaction = useCallback(
    async (
      messages: Message[],
      setMessages: (messages: Message[]) => void,
      append: (message: Message) => void,
      sessionId: string,
      isManual: boolean = false
    ) => {
      setIsCompacting(true);
      setCompactionError(null);

      try {
        // Get the summary from the backend
        const summaryResponse = await manageContextFromBackend({
          messages: messages,
          manageAction: 'summarize',
          sessionId: sessionId,
        });

        setMessages(summaryResponse.messages);

        // Only automatically submit the continuation message for auto-compaction (context limit reached)
        // Manual compaction should just compact without continuing the conversation
        if (!isManual) {
          // Automatically submit the continuation message to continue the conversation
          // This should be the third message (index 2) which contains the "I ran into a context length exceeded error..." text
          const continuationMessage = summaryResponse.messages[2];
          if (continuationMessage) {
            setTimeout(() => {
              append(continuationMessage);
            }, 100);
          }
        }

        setIsCompacting(false);
      } catch (err) {
        console.error('Error during compaction:', err);
        setCompactionError(err instanceof Error ? err.message : 'Unknown error during compaction');

        // Create an error marker
        const errorMarker: Message = {
          id: `compaction-error-${Date.now()}`,
          role: 'assistant',
          created: Math.floor(Date.now() / 1000),
          content: [
            {
              type: 'summarizationRequested',
              msg: 'Compaction failed. Please try again or start a new session.',
            },
          ],
        };

        setMessages([...messages, errorMarker]);
        setIsCompacting(false);
      }
    },
    []
  );

  const handleAutoCompaction = useCallback(
    async (
      messages: Message[],
      setMessages: (messages: Message[]) => void,
      append: (message: Message) => void,
      sessionId: string
    ) => {
      await performCompaction(messages, setMessages, append, sessionId, false);
    },
    [performCompaction]
  );

  const handleManualCompaction = useCallback(
    async (
      messages: Message[],
      setMessages: (messages: Message[]) => void,
      append?: (message: Message) => void,
      sessionId?: string
    ) => {
      await performCompaction(messages, setMessages, append || (() => {}), sessionId || '', true);
    },
    [performCompaction]
  );

  const hasCompactionMarker = useCallback((message: Message): boolean => {
    return message.content.some((content) => content.type === 'summarizationRequested');
  }, []);

  const value = {
    // State
    isCompacting,
    compactionError,

    // Actions
    handleAutoCompaction,
    handleManualCompaction,
    hasCompactionMarker,
  };

  return <ContextManagerContext.Provider value={value}>{children}</ContextManagerContext.Provider>;
};

// Create a hook to use the context
export const useContextManager = () => {
  const context = useContext(ContextManagerContext);
  if (context === undefined) {
    throw new Error('useContextManager must be used within a ContextManagerProvider');
  }
  return context;
};
