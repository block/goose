import React, { useState, useRef, useEffect } from 'react';
import { Message } from '../../types/message';
import { useChatContextManager } from './ContextManager';

interface ContextLengthExceededHandlerProps {
  messages: Message[];
  messageId: string;
}

export const ContextLengthExceededHandler: React.FC<ContextLengthExceededHandlerProps> = ({
  messages,
  messageId,
}) => {
  const { fetchSummary, summaryContent, isLoadingSummary, errorLoadingSummary, openSummaryModal } =
    useChatContextManager();

  const [hasFetchStarted, setHasFetchStarted] = useState(false);

  // Find the relevant message to check if it's the latest
  const isCurrentMessageLatest =
    messageId === messages[messages.length - 1].id ||
    messageId === messages[messages.length - 1].created.toString();

  // Only allow interaction for the most recent context length exceeded event
  const shouldAllowSummaryInteraction = isCurrentMessageLatest;

  // Use a ref to track if we've started the fetch
  const fetchStartedRef = useRef(false);

  useEffect(() => {
    // Automatically fetch summary if conditions are met
    if (
      !summaryContent &&
      !hasFetchStarted &&
      shouldAllowSummaryInteraction &&
      !fetchStartedRef.current
    ) {
      setHasFetchStarted(true);
      fetchStartedRef.current = true;
      fetchSummary(messages);
    }
  }, [fetchSummary, hasFetchStarted, messages, shouldAllowSummaryInteraction, summaryContent]);

  // Handle retry
  const handleRetry = () => {
    if (!shouldAllowSummaryInteraction) return;
    fetchSummary(messages);
  };

  // Render the notification UI
  return (
    <div className="flex flex-col items-start mt-1 pl-4">
      {isLoadingSummary && shouldAllowSummaryInteraction ? (
        // Only show loading indicator during loading state
        <div className="flex items-center text-xs text-gray-400">
          <span className="mr-2">Preparing summary...</span>
          <span className="animate-spin h-3 w-3 border-2 border-gray-400 rounded-full border-t-transparent"></span>
        </div>
      ) : (
        // Show different UI based on whether it's already handled
        <>
          <span className="text-xs text-gray-400 italic">{'Session summarized'}</span>

          {/* Only show the button if its last message */}
          {shouldAllowSummaryInteraction && (
            <button
              onClick={() => (errorLoadingSummary ? handleRetry() : openSummaryModal())}
              className="text-xs text-textStandard hover:text-textSubtle transition-colors mt-1 flex items-center"
            >
              {errorLoadingSummary ? 'Retry loading summary' : 'View or edit summary'}
            </button>
          )}
        </>
      )}
    </div>
  );
};
