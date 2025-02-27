import React, { useMemo } from 'react';
import LinkPreview from './LinkPreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';
import MarkdownContent from './MarkdownContent';
import ToolCallWithResponse from './ToolCallWithResponse';
import { Message, getTextContent, getToolRequests, getToolResponses } from '../types/message';

interface GooseMessageProps {
  message: Message;
  messages: Message[];
  metadata?: string[];
  append: (value: string) => void;
}

export default function GooseMessage({ message, metadata, messages, append }: GooseMessageProps) {
  // Extract text content from the message
  const textContent = getTextContent(message);

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

  return (
    <div className="goose-message flex w-[90%] justify-start opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex flex-col w-full">
        {/* Always show the top content area if there are tool calls, even if textContent is empty */}
        {(textContent || toolRequests.length > 0) && (
          <div
            className={`goose-message-content bg-bgSubtle rounded-2xl px-4 py-2 ${toolRequests.length > 0 ? 'rounded-b-none' : ''}`}
          >
            {textContent ? <MarkdownContent content={textContent} /> : null}
          </div>
        )}

        {toolRequests.length > 0 && (
          <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 mt-1">
            {toolRequests.map((toolRequest) => (
              <ToolCallWithResponse
                key={toolRequest.id}
                toolRequest={toolRequest}
                toolResponse={toolResponsesMap.get(toolRequest.id)}
              />
            ))}
          </div>
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
