import React, { useEffect, useMemo, useRef } from 'react';
import LinkPreview from './LinkPreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';
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

interface GooseMessageProps {
  messageHistoryIndex: number;
  message: Message;
  messages: Message[];
  metadata?: string[];
  append: (value: string) => void;
  appendMessage: (message: Message) => void;
}

export default function GooseMessage({
  messageHistoryIndex,
  message,
  metadata,
  messages,
  append,
  appendMessage,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Extract text content from the message
  let textContent = getTextContent(message);

  // Get tool requests from the message
  const toolRequests = getToolRequests(message);

  // Extract URLs under a few conditions
  // 1. The message is purely text
  // 2. The link wasn't also present in the previous message
  // 3. The message contains the explicit http:// or https:// protocol at the beginning
  const messageIndex = messages?.findIndex((msg) => msg.id === message.id);
  const previousMessage = messageIndex > 0 ? messages[messageIndex - 1] : null;
  const previousUrls = previousMessage ? extractUrls(getTextContent(previousMessage)) : [];
  const urls = toolRequests.length === 0 ? extractUrls(textContent, previousUrls) : [];

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
    if (messageIndex == messageHistoryIndex - 1 && hasToolConfirmation) {
      appendMessage(
        createToolErrorResponseMessage(toolConfirmationContent.id, 'The tool call is cancelled.')
      );
    }
  }, []);

  return (
    <div className="goose-message flex w-[90%] justify-start opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex flex-col w-full">
        {textContent && (
          <div className="flex flex-col group">
            <div
              className={`goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 ${toolRequests.length > 0 ? 'rounded-b-none' : ''}`}
            >
              <div ref={contentRef}>{<MarkdownContent content={textContent} />}</div>
            </div>
            {/* Only show MessageCopyLink if there's text content and no tool requests/responses */}
            {textContent && message.content.every((content) => content.type === 'text') && (
              <div className="flex justify-end mr-2">
                <MessageCopyLink text={textContent} contentRef={contentRef} />
              </div>
            )}
          </div>
        )}

        {toolRequests.length > 0 && (
          <div
            className={`goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 ${textContent ? '' : 'rounded-t-2xl'} rounded-b-2xl px-4 pt-4 pb-2 mt-1`}
          >
            {toolRequests.map((toolRequest) => (
              <ToolCallWithResponse
                // If the message is resumed and not matched tool response, it means the tool is broken or cancelled.
                isCancelledMessage={
                  messageIndex < messageHistoryIndex &&
                  toolResponsesMap.get(toolRequest.id) == undefined
                }
                key={toolRequest.id}
                toolRequest={toolRequest}
                toolResponse={toolResponsesMap.get(toolRequest.id)}
              />
            ))}
          </div>
        )}

        {hasToolConfirmation && (
          <ToolCallConfirmation
            isCancelledMessage={messageIndex == messageHistoryIndex - 1}
            isClicked={messageIndex < messageHistoryIndex - 1}
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
          <GooseResponseForm message={textContent} metadata={metadata} append={append} />
        </div>
      )}
    </div>
  );
}
