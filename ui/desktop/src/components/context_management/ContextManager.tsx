import React, { createContext, useContext, useState } from 'react';
import { Message } from '../../types/message';
import { manageContextFromBackend, convertApiMessageToFrontendMessage } from './index';

// Define the context management interface
interface ChatContextManagerState {
  summaryContent: string;
  summarizedThread: Message[];
  isSummaryModalOpen: boolean;
  isLoadingSummary: boolean;
  errorLoadingSummary: boolean;
}

interface ChatContextManagerActions {
  fetchSummary: (messages: Message[]) => Promise<void>;
  updateSummary: (newSummaryContent: string) => void;
  resetMessagesWithSummary: (
    messages: Message[],
    setMessages: (messages: Message[]) => void,
    ancestorMessages: Message[],
    setAncestorMessages: (messages: Message[]) => void,
    summaryContent: string
  ) => void;
  openSummaryModal: () => void;
  closeSummaryModal: () => void;
  hasContextLengthExceededContent: (message: Message) => boolean;
  handleContextLengthExceeded: (
    messages: Message[],
    chatId: string,
    workingDir: string
  ) => Promise<void>;
}

// Create the context
const ChatContextManagerContext = createContext<
  (ChatContextManagerState & ChatContextManagerActions) | undefined
>(undefined);

// Create the provider component
export const ChatContextManagerProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [summaryContent, setSummaryContent] = useState<string>('');
  const [summarizedThread, setSummarizedThread] = useState<Message[]>([]);
  const [isSummaryModalOpen, setIsSummaryModalOpen] = useState<boolean>(false);
  const [isLoadingSummary, setIsLoadingSummary] = useState<boolean>(false);
  const [errorLoadingSummary, setErrorLoadingSummary] = useState<boolean>(false);

  const handleContextLengthExceeded = async (messages: Message[]): Promise<void> => {
    setIsLoadingSummary(true);
    setErrorLoadingSummary(false);

    try {
      // 2. Now get the summary from the backend
      const summaryResponse = await manageContextFromBackend({
        messages: messages,
        manageAction: 'summarize',
      });

      // Convert API messages to frontend messages
      const convertedMessages = summaryResponse.messages.map(
        (apiMessage) => convertApiMessageToFrontendMessage(apiMessage, false, true) // do not show to user but send to llm
      );

      // Extract summary from the first message
      const summaryMessage = convertedMessages[0].content[0];
      if (summaryMessage.type === 'text') {
        const summary = summaryMessage.text;
        setSummaryContent(summary);
        setSummarizedThread(convertedMessages);
      }

      setIsLoadingSummary(false);
    } catch (err) {
      console.error('Error handling context length exceeded:', err);
      setErrorLoadingSummary(true);
      setIsLoadingSummary(false);
    }
  };

  const fetchSummary = async (messages: Message[]) => {
    setIsLoadingSummary(true);
    setErrorLoadingSummary(false);

    try {
      const response = await manageContextFromBackend({
        messages: messages,
        manageAction: 'summarize',
      });

      // Convert API messages to frontend messages
      const convertedMessages = response.messages.map(
        (apiMessage) => convertApiMessageToFrontendMessage(apiMessage, false, true) // do not show to user but send to llm
      );

      // Extract the summary text from the first message
      const summaryMessage = convertedMessages[0].content[0];
      if (summaryMessage.type === 'text') {
        const summary = summaryMessage.text;
        setSummaryContent(summary);
        setSummarizedThread(convertedMessages);
      }

      setIsLoadingSummary(false);
    } catch (err) {
      console.error('Error fetching summary:', err);
      setErrorLoadingSummary(true);
      setIsLoadingSummary(false);
    }
  };

  const updateSummary = (newSummaryContent: string) => {
    // Update the summary content
    setSummaryContent(newSummaryContent);

    // Update the thread if it exists
    if (summarizedThread.length > 0) {
      // Create a deep copy of the thread
      const updatedThread = [...summarizedThread];

      // Create a copy of the first message
      const firstMessage = { ...updatedThread[0] };

      // Create a copy of the content array
      const updatedContent = [...firstMessage.content];

      // Update the summary text in the first content item
      if (updatedContent[0] && updatedContent[0].type === 'text') {
        updatedContent[0] = {
          ...updatedContent[0],
          text: newSummaryContent,
        };
      }

      // Update the message with the new content
      firstMessage.content = updatedContent;
      updatedThread[0] = firstMessage;

      // Update the thread
      setSummarizedThread(updatedThread);
    }
  };

  const resetMessagesWithSummary = (
    messages: Message[],
    setMessages: (messages: Message[]) => void,
    ancestorMessages: Message[],
    setAncestorMessages: (messages: Message[]) => void,
    summaryContent: string
  ) => {
    // Create a copy of the summarized thread
    const updatedSummarizedThread = [...summarizedThread];

    // Make sure there's at least one message in the summarized thread
    if (updatedSummarizedThread.length > 0) {
      // Get the first message
      const firstMessage = { ...updatedSummarizedThread[0] };

      // Make a copy of the content array
      const contentCopy = [...firstMessage.content];

      // Assuming the first content item is of type TextContent
      if (contentCopy.length > 0 && 'text' in contentCopy[0]) {
        // Update the text with the new summary content
        contentCopy[0] = {
          ...contentCopy[0],
          text: summaryContent,
        };

        // Update the first message with the new content
        firstMessage.content = contentCopy;

        // Update the first message in the thread
        updatedSummarizedThread[0] = firstMessage;
      }
    }

    // Update metadata for the summarized thread
    const finalUpdatedThread = updatedSummarizedThread.map((msg, index) => ({
      ...msg,
      display: index === 0, // First message has display: true, others false
      sendToLLM: true, // All messages have sendToLLM: true
    }));

    // Update the messages state
    setMessages(finalUpdatedThread);

    // If ancestorMessages already has items, extend it instead of replacing it
    if (ancestorMessages.length > 0) {
      // Convert current messages to ancestor format
      const newAncestorMessages = messages.map((msg) => ({
        ...msg,
        display: true,
        sendToLLM: false,
      }));

      // Append new ancestor messages to existing ones
      setAncestorMessages([...ancestorMessages, ...newAncestorMessages]);
    } else {
      // Initial set of ancestor messages
      const newAncestorMessages = messages.map((msg) => ({
        ...msg,
        display: true,
        sendToLLM: false,
      }));

      setAncestorMessages(newAncestorMessages);
    }

    // Clear the summarized thread and content
    setSummarizedThread([]);
    setSummaryContent('');
  };

  const hasContextLengthExceededContent = (message: Message): boolean => {
    return message.content.some((content) => content.type === 'contextLengthExceeded');
  };

  const openSummaryModal = () => {
    setIsSummaryModalOpen(true);
  };

  const closeSummaryModal = () => {
    setIsSummaryModalOpen(false);
  };

  const value = {
    // State
    summaryContent,
    summarizedThread,
    isSummaryModalOpen,
    isLoadingSummary,
    errorLoadingSummary,

    // Actions
    fetchSummary,
    updateSummary,
    resetMessagesWithSummary,
    openSummaryModal,
    closeSummaryModal,
    hasContextLengthExceededContent,
    handleContextLengthExceeded,
  };

  return (
    <ChatContextManagerContext.Provider value={value}>
      {children}
    </ChatContextManagerContext.Provider>
  );
};

// Create a hook to use the context
export const useChatContextManager = () => {
  const context = useContext(ChatContextManagerContext);
  if (context === undefined) {
    throw new Error('useContextManager must be used within a ContextManagerProvider');
  }
  return context;
};
