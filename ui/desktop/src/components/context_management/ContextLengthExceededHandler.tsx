import React, { useState, useRef, useEffect } from 'react';
import { Message } from '../../types/message';
import { useChatContextManager } from './ContextManager';

interface ContextLengthExceededHandlerProps {
  messages: Message[];
  messageId: string;
  chatId: string;
  workingDir: string;
}

export const ContextLengthExceededHandler: React.FC<ContextLengthExceededHandlerProps> = ({
  messages,
  messageId,
  chatId,
  workingDir,
}) => {
  const {
    summaryContent,
    isLoadingSummary,
    errorLoadingSummary,
    openSummaryModal,
    handleContextLengthExceeded,
  } = useChatContextManager();

  const [hasFetchStarted, setHasFetchStarted] = useState(false);

  // Find the relevant message to check if it's the latest
  const isCurrentMessageLatest =
    messageId === messages[messages.length - 1]?.id ||
    messageId === String(messages[messages.length - 1]?.created);

  // Only allow interaction for the most recent context length exceeded event
  const shouldAllowSummaryInteraction = isCurrentMessageLatest;

  // Use a ref to track if we've started the fetch
  const fetchStartedRef = useRef(false);

  // Function to trigger the async operation properly
  const triggerContextLengthExceeded = () => {
    setHasFetchStarted(true);
    fetchStartedRef.current = true;

    // Call the async function without awaiting it in useEffect
    handleContextLengthExceeded(messages, chatId, workingDir).catch((err) => {
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

    // Reset states for retry
    setHasFetchStarted(false);
    fetchStartedRef.current = false;

    // Trigger the process again
    triggerContextLengthExceeded();
  };

  // Render the notification UI
  return (
    <div className="flex flex-col items-start mt-1 pl-4">
      {/* Horizontal line with text in the middle - shown regardless of loading state */}
      <div className="relative flex items-center py-2 w-full">
        <div className="flex-grow border-t border-gray-300"></div>
        <div className="flex-grow border-t border-gray-300"></div>
      </div>

      {isLoadingSummary && shouldAllowSummaryInteraction ? (
        // Show loading indicator during loading state
        <div className="flex items-center text-xs text-gray-400">
          <span className="mr-2">Preparing summary...</span>
          <span className="animate-spin h-3 w-3 border-2 border-gray-400 rounded-full border-t-transparent"></span>
        </div>
      ) : (
        // Show different UI based on whether it's already handled
        <>
          <span className="text-xs text-gray-400">{`Your conversation has exceeded the model's context capacity`}</span>
          <span className="text-xs text-gray-400">{`Messages above this line remain viewable but are not included in the active context`}</span>
          {/* Only show the button if its last message */}
          {shouldAllowSummaryInteraction && (
            <button
              onClick={() => (errorLoadingSummary ? handleRetry() : openSummaryModal())}
              className="text-xs text-textStandard hover:text-textSubtle transition-colors mt-1 flex items-center"
            >
              {errorLoadingSummary
                ? 'Retry loading summary'
                : 'View or edit summary (you may continue your conversation based on the summary)'}
            </button>
          )}
        </>
      )}
    </div>
  );
};
