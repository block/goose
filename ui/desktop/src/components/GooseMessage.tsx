import { useEffect, useMemo, useRef } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import { formatMessageTimestamp } from '../utils/timeUtils';
import MarkdownContent from './MarkdownContent';
import ToolCallWithResponse from './ToolCallWithResponse';
import {
  Message,
  getTextContent,
  getToolRequests,
  getToolResponses,
  getToolConfirmationContent,
  createToolErrorResponseMessage,
} from '../types/message';
import ToolCallConfirmation from './ToolCallConfirmation';
import MessageCopyLink from './MessageCopyLink';
import { NotificationEvent } from '../hooks/useMessageStream';

interface GooseMessageProps {
  // messages up to this index are presumed to be "history" from a resumed session, this is used to track older tool confirmation requests
  // anything before this index should not render any buttons, but anything after should
  messageHistoryIndex: number;
  message: Message;
  messages: Message[];
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  appendMessage: (message: Message) => void;
  isStreaming?: boolean; // Whether this message is currently being streamed
}

export default function GooseMessage({
  messageHistoryIndex,
  message,
  metadata,
  messages,
  toolCallNotifications,
  append,
  appendMessage,
  isStreaming = false,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  // Track which tool confirmations we've already handled to prevent infinite loops
  const handledToolConfirmations = useRef<Set<string>>(new Set());

  // Extract text content from the message
  let textContent = getTextContent(message);

  // Utility to split Chain-of-Thought (CoT) from the visible assistant response.
  // If the text contains a <think>...</think> block, everything inside is treated as the
  // CoT and removed from the user-visible text.
  const splitChainOfThought = (text: string): { visibleText: string; cotText: string | null } => {
    const regex = /<think>([\s\S]*?)<\/think>/i;
    const match = text.match(regex);
    if (!match) {
      return { visibleText: text, cotText: null };
    }

    const cotRaw = match[1].trim();
    const visible = text.replace(match[0], '').trim();
    return { visibleText: visible, cotText: cotRaw.length > 0 ? cotRaw : null };
  };

  const { visibleText: textWithoutCot, cotText } = splitChainOfThought(textContent);

  // Extract image paths from the visible part of the message (exclude CoT)
  const imagePaths = extractImagePaths(textWithoutCot);

  // Remove image paths from text for display
  const displayText =
    imagePaths.length > 0 ? removeImagePathsFromText(textWithoutCot, imagePaths) : textWithoutCot;

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Get tool requests from the message
  const toolRequests = getToolRequests(message);

  // Extract URLs under a few conditions
  // 1. The message is purely text
  // 2. The link wasn't also present in the previous message
  // 3. The message contains the explicit http:// or https:// protocol at the beginning
  const messageIndex = messages?.findIndex((msg) => msg.id === message.id);
  const previousMessage = messageIndex > 0 ? messages[messageIndex - 1] : null;
  const previousUrls = previousMessage ? extractUrls(getTextContent(previousMessage)) : [];
  const urls = toolRequests.length === 0 ? extractUrls(displayText, previousUrls) : [];

  const toolConfirmationContent = getToolConfirmationContent(message);
  const hasToolConfirmation = toolConfirmationContent !== undefined;

  // Find tool responses that correspond to the tool requests in this message
  const toolResponsesMap = useMemo(() => {
    const responseMap = new Map();

    // Look for tool responses in subsequent messages
    if (messageIndex !== undefined && messageIndex >= 0) {
      for (let i = messageIndex + 1; i < messages.length; i++) {
        const responses = getToolResponses(messages[i]);

        for (const response of responses) {
          // Check if this response matches any of our tool requests
          const matchingRequest = toolRequests.find((req) => req.id === response.id);
          if (matchingRequest) {
            responseMap.set(response.id, response);
          }
        }
      }
    }

    return responseMap;
  }, [messages, messageIndex, toolRequests]);

  useEffect(() => {
    // If the message is the last message in the resumed session and has tool confirmation, it means the tool confirmation
    // is broken or cancelled, to contonue use the session, we need to append a tool response to avoid mismatch tool result error.
    if (
      messageIndex === messageHistoryIndex - 1 &&
      hasToolConfirmation &&
      toolConfirmationContent &&
      !handledToolConfirmations.current.has(toolConfirmationContent.id)
    ) {
      // Only append the error message if there isn't already a response for this tool confirmation
      const hasExistingResponse = messages.some((msg) =>
        getToolResponses(msg).some((response) => response.id === toolConfirmationContent.id)
      );

      if (!hasExistingResponse) {
        // Mark this tool confirmation as handled to prevent infinite loop
        handledToolConfirmations.current.add(toolConfirmationContent.id);

        appendMessage(
          createToolErrorResponseMessage(toolConfirmationContent.id, 'The tool call is cancelled.')
        );
      }
    }
  }, [
    messageIndex,
    messageHistoryIndex,
    hasToolConfirmation,
    toolConfirmationContent,
    messages,
    appendMessage,
  ]);

  return (
    <div className="goose-message flex w-[90%] justify-start min-w-0">
      <div className="flex flex-col w-full min-w-0">
        {/* Chain-of-Thought (hidden by default) */}
        {cotText && (
          <details className="bg-bgSubtle border border-borderSubtle rounded p-2 mb-2">
            <summary className="cursor-pointer text-sm text-textSubtle select-none">
              Show thinking
            </summary>
            <div className="mt-2">
              <MarkdownContent content={cotText} />
            </div>
          </details>
        )}

        {/* Visible assistant response */}
        {displayText && (
          <div className="flex flex-col group">
            <div className={`goose-message-content py-2`}>
              <div ref={contentRef}>{<MarkdownContent content={displayText} />}</div>
            </div>

            {/* Render images if any */}
            {imagePaths.length > 0 && (
              <div className="flex flex-wrap gap-2 mt-2 mb-2">
                {imagePaths.map((imagePath, index) => (
                  <ImagePreview key={index} src={imagePath} alt={`Image ${index + 1}`} />
                ))}
              </div>
            )}

            {/* Only show timestamp and copy link when not streaming */}
            <div className="relative flex justify-start">
              {toolRequests.length === 0 && !isStreaming && (
                <div className="text-xs font-mono text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
                  {timestamp}
                </div>
              )}
              {displayText &&
                message.content.every((content) => content.type === 'text') &&
                !isStreaming && (
                  <div className="absolute left-0 pt-1">
                    <MessageCopyLink text={displayText} contentRef={contentRef} />
                  </div>
                )}
            </div>
          </div>
        )}

        {toolRequests.length > 0 && (
          <div className="relative flex flex-col w-full">
            {toolRequests.map((toolRequest) => (
              <div className={`goose-message-tool pb-2`} key={toolRequest.id}>
                <ToolCallWithResponse
                  // If the message is resumed and not matched tool response, it means the tool is broken or cancelled.
                  isCancelledMessage={
                    messageIndex < messageHistoryIndex &&
                    toolResponsesMap.get(toolRequest.id) == undefined
                  }
                  toolRequest={toolRequest}
                  toolResponse={toolResponsesMap.get(toolRequest.id)}
                  notifications={toolCallNotifications.get(toolRequest.id)}
                  isStreamingMessage={isStreaming}
                />
              </div>
            ))}
            <div className="text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
              {!isStreaming && timestamp}
            </div>
          </div>
        )}

        {hasToolConfirmation && (
          <ToolCallConfirmation
            isCancelledMessage={messageIndex == messageHistoryIndex - 1}
            isClicked={messageIndex < messageHistoryIndex}
            toolConfirmationId={toolConfirmationContent.id}
            toolName={toolConfirmationContent.toolName}
          />
        )}
      </div>

      {/* TODO(alexhancock): Re-enable link previews once styled well again */}
      {false && urls.length > 0 && (
        <div className="flex flex-wrap mt-[16px]">
          {urls.map((url, index) => (
            <LinkPreview key={index} url={url} />
          ))}
        </div>
      )}

      {/* enable or disable prompts here */}
      {/* NOTE from alexhancock on 1/14/2025 - disabling again temporarily due to non-determinism in when the forms show up */}
      {false && metadata && (
        <div className="flex mt-[16px]">
          <GooseResponseForm message={displayText} metadata={metadata || null} append={append} />
        </div>
      )}
    </div>
  );
}
