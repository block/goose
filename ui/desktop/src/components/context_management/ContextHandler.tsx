import React, { useState, useRef, useEffect } from 'react';
import { Message } from '../../types/message';
import { useChatContextManager } from './ChatContextManager';
import { Button } from '../ui/button';

interface ContextHandlerProps {
  messages: Message[];
  messageId: string;
  chatId: string;
  workingDir: string;
  contextType: 'contextLengthExceeded' | 'summarizationRequested';
  onSummaryComplete?: () => void; // Add callback for when summary is complete
}

export const ContextHandler: React.FC<ContextHandlerProps> = ({
  messages,
  messageId,
  chatId,
  workingDir,
  contextType,
  onSummaryComplete,
}) => {
  const {
    summaryContent,
    isLoadingCompaction,
    errorLoadingSummary,
    openSummaryModal,
    handleContextLengthExceeded,
  } = useChatContextManager();
  const [hasFetchStarted, setHasFetchStarted] = useState(false);
  const [retryCount, setRetryCount] = useState(0);

  const isContextLengthExceeded = contextType === 'contextLengthExceeded';

  // Find the relevant message to check if it's the latest
  const isCurrentMessageLatest =
    messageId === messages[messages.length - 1]?.id ||
    messageId === String(messages[messages.length - 1]?.created);

  // Only allow interaction for the most recent context length exceeded event
  const shouldAllowSummaryInteraction = isCurrentMessageLatest;

  // Use a ref to track if we've started the fetch
  const fetchStartedRef = useRef(false);
  const hasCalledSummaryComplete = useRef(false);

  // Call onSummaryComplete when summary is ready
  useEffect(() => {
    if (summaryContent && shouldAllowSummaryInteraction && !hasCalledSummaryComplete.current) {
      hasCalledSummaryComplete.current = true;
      // Delay the scroll slightly to ensure the content is rendered
      setTimeout(() => {
        onSummaryComplete?.();
      }, 100);
    }

    // Reset the flag when summary is cleared
    if (!summaryContent) {
      hasCalledSummaryComplete.current = false;
    }
  }, [summaryContent, shouldAllowSummaryInteraction, onSummaryComplete]);

  // Scroll when summarization starts (loading state)
  useEffect(() => {
    if (isLoadingCompaction && shouldAllowSummaryInteraction) {
      // Delay the scroll slightly to ensure the loading content is rendered
      setTimeout(() => {
        onSummaryComplete?.();
      }, 100);
    }
  }, [isLoadingCompaction, shouldAllowSummaryInteraction, onSummaryComplete]);

  // Function to trigger the async operation properly
  const triggerContextLengthExceeded = () => {
    setHasFetchStarted(true);
    fetchStartedRef.current = true;

    // Call the async function without awaiting it in useEffect
    handleContextLengthExceeded(messages).catch((err) => {
      console.error('Error handling context length exceeded:', err);
    });
  };

  useEffect(() => {
    if (
      !summaryContent &&
      !hasFetchStarted &&
      shouldAllowSummaryInteraction &&
      !fetchStartedRef.current
    ) {
      // Use the wrapper function instead of calling the async function directly
      triggerContextLengthExceeded();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    hasFetchStarted,
    messages,
    shouldAllowSummaryInteraction,
    summaryContent,
    chatId,
    workingDir,
  ]);

  // Handle retry - Call the async function properly
  const handleRetry = () => {
    if (!shouldAllowSummaryInteraction) return;

    // Only increment retry counter if there's an error
    if (errorLoadingSummary) {
      setRetryCount((prevCount) => prevCount + 1);
    }

    // Reset states for retry
    setHasFetchStarted(false);
    fetchStartedRef.current = false;

    // Trigger the process again
    triggerContextLengthExceeded();
  };

  // Function to open a new goose window
  const openNewSession = () => {
    try {
      // Use the workingDir from props directly without reassignment to avoid TypeScript error
      const sessionWorkingDir = window.appConfig?.get('GOOSE_WORKING_DIR') || workingDir;
      console.log(`Creating new chat window with working dir: ${sessionWorkingDir}`);
      window.electron.createChatWindow(undefined, sessionWorkingDir as string);
    } catch (error) {
      console.error('Error creating new window:', error);
      // Fallback to basic window.open if the electron API fails
      window.open('/', '_blank');
    }
  };

  // Render the notification UI
  const renderLoadingState = () => (
    <div className="flex items-center text-xs text-gray-400">
      <span className="mr-2">Preparing summary...</span>
      <span className="animate-spin h-3 w-3 border-2 border-gray-400 rounded-full border-t-transparent"></span>
    </div>
  );

  const renderFailedState = () => (
    <>
      <span className="text-xs text-gray-400">
        {isContextLengthExceeded
          ? `Your conversation has exceeded the model's context capacity`
          : `Summarization requested`}
      </span>
      <span className="text-xs text-gray-400">
        {isContextLengthExceeded
          ? `This conversation has too much information to continue. Extension data often takes up significant space.`
          : `Summarization failed. Continue chatting or start a new session.`}
      </span>
      <Button onClick={openNewSession} className="text-xs transition-colors mt-1 flex items-center">
        Click here to start a new session
      </Button>
    </>
  );

  const renderRetryState = () => (
    <>
      <span className="text-xs text-gray-400">
        {isContextLengthExceeded
          ? `Your conversation has exceeded the model's context capacity`
          : `Summarization requested`}
      </span>
      <Button onClick={handleRetry} className="text-xs transition-colors mt-1 flex items-center">
        Retry loading summary
      </Button>
    </>
  );

  const renderSuccessState = () => (
    <>
      <span className="text-xs text-gray-400">
        {isContextLengthExceeded
          ? `Your conversation has exceeded the model's context capacity and a summary was prepared.`
          : `A summary of your conversation was prepared as requested.`}
      </span>
      <span className="text-xs text-gray-400">
        {isContextLengthExceeded
          ? `Messages above this line remain viewable but specific details are not included in active context.`
          : `This summary includes key points from your conversation.`}
      </span>
      {shouldAllowSummaryInteraction && (
        <Button
          onClick={openSummaryModal}
          className="text-xs transition-colors mt-1 flex items-center"
        >
          View or edit summary{' '}
          {isContextLengthExceeded
            ? '(you may continue your conversation based on the summary)'
            : ''}
        </Button>
      )}
    </>
  );

  // Render persistent summarized notification when we shouldn't show interaction options
  const renderPersistentMarker = () => (
    <span className="text-xs text-gray-400">
      Session summarized — messages above this line are not included in the conversation
    </span>
  );

  const renderContentState = () => {
    // If this is not the latest context event message but we have a valid summary,
    // show the persistent marker
    if (!shouldAllowSummaryInteraction && summaryContent) {
      return renderPersistentMarker();
    }

    // For the latest message with the context event
    if (shouldAllowSummaryInteraction) {
      if (errorLoadingSummary) {
        return retryCount >= 2 ? renderFailedState() : renderRetryState();
      }

      if (summaryContent) {
        return renderSuccessState();
      }
    }

    // Fallback to showing at least the persistent marker
    return renderPersistentMarker();
  };

  return (
    <div className="flex flex-col items-start mt-1 pl-4">
      {/* Horizontal line with text in the middle - shown regardless of loading state */}
      <div className="relative flex items-center py-2 w-full">
        <div className="flex-grow border-t border-gray-300"></div>
        <div className="flex-grow border-t border-gray-300"></div>
      </div>

      {isLoadingCompaction && shouldAllowSummaryInteraction
        ? renderLoadingState()
        : renderContentState()}
    </div>
  );
};
