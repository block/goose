import { useMemo, useRef } from 'react';
import ImagePreview from './ImagePreview';
import { formatMessageTimestamp } from '../utils/timeUtils';
import MarkdownContent from './MarkdownContent';
import ToolCallWithResponse from './ToolCallWithResponse';
import {
  getTextAndImageContent,
  getToolRequests,
  getToolResponses,
  getToolConfirmationContent,
  getElicitationContent,
  getPendingToolConfirmationIds,
  NotificationEvent,
} from '../types/message';
import { Message } from '../api';
import ToolCallConfirmation from './ToolCallConfirmation';
import ElicitationRequest from './ElicitationRequest';
import MessageCopyLink from './MessageCopyLink';
import { cn } from '../utils';
import { identifyConsecutiveToolCalls, shouldHideTimestamp } from '../utils/toolCallChaining';

interface GooseMessageProps {
  sessionId: string;
  message: Message;
  messages: Message[];
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  isStreaming?: boolean; // Whether this message is currently being streamed
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<void>;
}

export default function GooseMessage({
  sessionId,
  message,
  messages,
  toolCallNotifications,
  append,
  isStreaming = false,
  submitElicitationResponse,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);

  let { textContent, imagePaths } = getTextAndImageContent(message);

  const splitChainOfThought = (text: string): { displayText: string; cotText: string | null } => {
    const regex = /<think>([\s\S]*?)<\/think>/i;
    const match = text.match(regex);
    if (!match) {
      return { displayText: text, cotText: null };
    }

    const cotRaw = match[1].trim();
    const displayText = text.replace(regex, '').trim();

    return {
      displayText,
      cotText: cotRaw || null,
    };
  };

  const { displayText, cotText } = splitChainOfThought(textContent);

  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);
  const toolRequests = getToolRequests(message);
  const messageIndex = messages.findIndex((msg) => msg.id === message.id);
  const toolConfirmationContent = getToolConfirmationContent(message);
  const elicitationContent = getElicitationContent(message);
  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);
  const hideTimestamp = useMemo(
    () => shouldHideTimestamp(messageIndex, toolCallChains),
    [messageIndex, toolCallChains]
  );
  const hasToolConfirmation = toolConfirmationContent !== undefined;
  const hasElicitation = elicitationContent !== undefined;

  const toolResponsesMap = useMemo(() => {
    const responseMap = new Map();

    if (messageIndex !== undefined && messageIndex >= 0) {
      for (let i = messageIndex + 1; i < messages.length; i++) {
        const responses = getToolResponses(messages[i]);

        for (const response of responses) {
          const matchingRequest = toolRequests.find((req) => req.id === response.id);
          if (matchingRequest) {
            responseMap.set(response.id, response);
          }
        }
      }
    }

    return responseMap;
  }, [messages, messageIndex, toolRequests]);

  const pendingConfirmationIds = getPendingToolConfirmationIds(messages);

  return (
    <div className="goose-message flex w-[90%] justify-start min-w-0">
      <div className="flex flex-col w-full min-w-0">
        {cotText && (
          <details className="bg-bgSubtle border border-border-default rounded p-2 mb-2">
            <summary className="cursor-pointer text-sm text-text-muted select-none">
              Show thinking
            </summary>
            <div className="mt-2">
              <MarkdownContent content={cotText} />
            </div>
          </details>
        )}

        {(displayText.trim() || imagePaths.length > 0) && (
          <div className="flex flex-col group">
            {displayText.trim() && (
              <div ref={contentRef} className="w-full">
                <MarkdownContent content={displayText} />
              </div>
            )}

            {imagePaths.length > 0 && (
              <div className="mt-4">
                {imagePaths.map((imagePath, index) => (
                  <ImagePreview key={index} src={imagePath} />
                ))}
              </div>
            )}

            {toolRequests.length === 0 && (
              <div className="relative flex justify-start">
                {!isStreaming && (
                  <div className="text-xs font-mono text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
                    {timestamp}
                  </div>
                )}
                {message.content.every((content) => content.type === 'text') && !isStreaming && (
                  <div className="absolute left-0 pt-1">
                    <MessageCopyLink text={displayText} contentRef={contentRef} />
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {toolRequests.length > 0 && (
          <div className={cn(displayText && 'mt-2')}>
            <div className="relative flex flex-col w-full">
              <div className="flex flex-col gap-3">
                {toolRequests.map((toolRequest) => (
                  <div className="goose-message-tool" key={toolRequest.id}>
                    <ToolCallWithResponse
                      sessionId={sessionId}
                      isCancelledMessage={false}
                      toolRequest={toolRequest}
                      toolResponse={toolResponsesMap.get(toolRequest.id)}
                      notifications={toolCallNotifications.get(toolRequest.id)}
                      isStreamingMessage={isStreaming}
                      isPendingApproval={pendingConfirmationIds.has(toolRequest.id)}
                      append={append}
                    />
                  </div>
                ))}
              </div>
              <div className="text-xs text-text-muted transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 pt-1">
                {!isStreaming && !hideTimestamp && timestamp}
              </div>
            </div>
          </div>
        )}

        {hasToolConfirmation && (
          <ToolCallConfirmation
            sessionId={sessionId}
            isCancelledMessage={false}
            isClicked={false}
            actionRequiredContent={toolConfirmationContent}
          />
        )}

        {hasElicitation && submitElicitationResponse && (
          <ElicitationRequest
            isCancelledMessage={false}
            isClicked={false}
            actionRequiredContent={elicitationContent}
            onSubmit={submitElicitationResponse}
          />
        )}
      </div>
    </div>
  );
}
