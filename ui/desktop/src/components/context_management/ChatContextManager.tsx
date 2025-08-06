import React, { createContext, useContext, useState } from 'react';
import { Message } from '../../types/message';
import {
  manageContextFromBackend,
  convertApiMessageToFrontendMessage,
  createSummarizationRequestMessage,
} from './index';

// Define the context management interface
interface ChatContextManagerState {
  summaryContent: string;
  summarizedThread: Message[];
  isSummaryModalOpen: boolean;
  isLoadingCompaction: boolean;
  errorLoadingSummary: boolean;
  preparingManualSummary: boolean;
}

interface ChatContextManagerActions {
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
  hasContextHandlerContent: (message: Message) => boolean;
  hasContextLengthExceededContent: (message: Message) => boolean;
  hasSummarizationRequestedContent: (message: Message) => boolean;
  getContextHandlerType: (message: Message) => 'contextLengthExceeded' | 'summarizationRequested';
  handleContextLengthExceeded: (messages: Message[]) => Promise<void>;
  handleManualCompaction: (messages: Message[], setMessages: (messages: Message[]) => void) => void;
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
  const [isLoadingCompaction, setIsLoadingCompaction] = useState<boolean>(false);
  const [errorLoadingSummary, setErrorLoadingSummary] = useState<boolean>(false);
  const [preparingManualSummary, setPreparingManualSummary] = useState<boolean>(false);

  const handleContextLengthExceeded = async (messages: Message[]): Promise<void> => {
    setIsLoadingCompaction(true);
    setErrorLoadingSummary(false);
    setPreparingManualSummary(true);

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

      setIsLoadingCompaction(false);
    } catch (err) {
      console.error('Error handling context length exceeded:', err);
      setErrorLoadingSummary(true);
      setIsLoadingCompaction(false);
    } finally {
      setPreparingManualSummary(false);
    }
  };

  const handleManualCompaction = (
    messages: Message[],
    setMessages: (messages: Message[]) => void
  ): void => {
    // add some messages to the message thread
    // these messages will be filtered out in chat view
    // but they will also be what allows us to render some text in the chatview itself, similar to CLE events
    const summarizationRequest = createSummarizationRequestMessage(
      messages,
      'Summarize the session and begin a new one'
    );

    // add the message to the message thread
    setMessages([...messages, summarizationRequest]);
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

  const hasContextHandlerContent = (message: Message): boolean => {
    return hasContextLengthExceededContent(message) || hasSummarizationRequestedContent(message);
  };

  const hasContextLengthExceededContent = (message: Message): boolean => {
    return message.content.some((content) => content.type === 'contextLengthExceeded');
  };

  const hasSummarizationRequestedContent = (message: Message): boolean => {
    return message.content.some((content) => content.type === 'summarizationRequested');
  };

  const getContextHandlerType = (
    message: Message
  ): 'contextLengthExceeded' | 'summarizationRequested' => {
    if (hasContextLengthExceededContent(message)) {
      return 'contextLengthExceeded';
    }
    return 'summarizationRequested';
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
    isLoadingCompaction,
    errorLoadingSummary,
    preparingManualSummary,

    // Actions
    updateSummary,
    resetMessagesWithSummary,
    openSummaryModal,
    closeSummaryModal,
    hasContextHandlerContent,
    hasContextLengthExceededContent,
    hasSummarizationRequestedContent,
    getContextHandlerType,
    handleContextLengthExceeded,
    handleManualCompaction,
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
