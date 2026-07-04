import { memo, useMemo, useRef } from 'react';
import ImagePreview from './ImagePreview';
import { formatMessageTimestamp } from '../utils/timeUtils';
import MarkdownContent from './MarkdownContent';
import ThinkingContent from './ThinkingContent';
import ToolCallWithResponse from './ToolCallWithResponse';
import {
  getTextAndImageContent,
  getThinkingContent,
  getToolRequests,
  getToolConfirmationContent,
  getElicitationContent,
  getAnyToolConfirmationData,
  ToolConfirmationData,
  NotificationEvent,
  ToolResponseMessageContent,
  type Message,
} from '../types/message';
import ToolCallConfirmation from './ToolCallConfirmation';
import ElicitationRequest from './ElicitationRequest';
import MessageCopyLink from './MessageCopyLink';
import { cn } from '../utils';

interface GoslingMessageProps {
  sessionId: string;
  message: Message;
  hideTimestamp: boolean;
  toolResponsesById: Map<string, ToolResponseMessageContent>;
  confirmationByToolRequestId: Map<string, ToolConfirmationData>;
  pendingConfirmationIds: Set<string>;
  toolRequestIds: Set<string>;
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  isStreaming: boolean;
  submitElicitationResponse?: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<boolean>;
}

function GoslingMessage({
  sessionId,
  message,
  hideTimestamp,
  toolResponsesById,
  confirmationByToolRequestId,
  pendingConfirmationIds,
  toolRequestIds,
  toolCallNotifications,
  append,
  isStreaming,
  submitElicitationResponse,
}: GoslingMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);

  const { textContent: displayText, imagePaths } = getTextAndImageContent(message);
  const thinkingContent = getThinkingContent(message);

  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);
  const toolRequests = getToolRequests(message);
  const toolConfirmationContent = getToolConfirmationContent(message);
  const elicitationContent = getElicitationContent(message);
  const hasToolConfirmation = toolConfirmationContent !== undefined;
  const hasElicitation = elicitationContent !== undefined;
  const elicitationData =
    elicitationContent?.data.actionType === 'elicitation'
      ? (elicitationContent.data as typeof elicitationContent.data & {
          isSubmitted?: boolean;
          isCancelled?: boolean;
        })
      : undefined;

  const toolConfirmationShownInline = useMemo(() => {
    if (!toolConfirmationContent) return false;
    const confirmationData = getAnyToolConfirmationData(message);
    if (!confirmationData) return false;

    return toolRequestIds.has(confirmationData.id);
  }, [toolConfirmationContent, message, toolRequestIds]);

  return (
    <div className="gosling-message flex w-[90%] justify-start min-w-0">
      <div className="flex flex-col w-full min-w-0">
        {thinkingContent && (
          <ThinkingContent
            content={thinkingContent}
            isExpanded={
              isStreaming &&
              !displayText.trim() &&
              imagePaths.length === 0 &&
              toolRequests.length === 0
            }
          />
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
                  <div className="text-xs font-mono text-text-secondary pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
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
                {toolRequests.map((toolRequest) => {
                  const toolResponse = toolResponsesById.get(toolRequest.id);
                  const hasResponse = toolResponse !== undefined;
                  const isPending = pendingConfirmationIds.has(toolRequest.id);
                  const confirmationContent = confirmationByToolRequestId.get(toolRequest.id);
                  const isApprovalClicked = confirmationContent && !isPending && hasResponse;
                  return (
                    <div className="gosling-message-tool" key={toolRequest.id}>
                      <ToolCallWithResponse
                        sessionId={sessionId}
                        isCancelledMessage={false}
                        toolRequest={toolRequest}
                        toolResponse={toolResponse}
                        notifications={toolCallNotifications.get(toolRequest.id)}
                        isStreamingMessage={isStreaming}
                        isPendingApproval={isPending}
                        append={append}
                        confirmationContent={confirmationContent}
                        isApprovalClicked={isApprovalClicked}
                      />
                    </div>
                  );
                })}
              </div>
              <div className="text-xs text-text-secondary transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 pt-1">
                {!isStreaming && !hideTimestamp && timestamp}
              </div>
            </div>
          </div>
        )}

        {hasToolConfirmation && !toolConfirmationShownInline && (
          <ToolCallConfirmation
            sessionId={sessionId}
            isClicked={false}
            actionRequiredContent={toolConfirmationContent}
          />
        )}

        {hasElicitation && submitElicitationResponse && (
          <ElicitationRequest
            isCancelledMessage={elicitationData?.isCancelled === true}
            isClicked={elicitationData?.isSubmitted === true}
            actionRequiredContent={elicitationContent}
            onSubmit={submitElicitationResponse}
          />
        )}
      </div>
    </div>
  );
}

function areGoslingMessagePropsEqual(prev: GoslingMessageProps, next: GoslingMessageProps): boolean {
  if (
    prev.sessionId !== next.sessionId ||
    prev.message !== next.message ||
    prev.hideTimestamp !== next.hideTimestamp ||
    prev.metadata !== next.metadata ||
    prev.append !== next.append ||
    prev.isStreaming !== next.isStreaming ||
    prev.submitElicitationResponse !== next.submitElicitationResponse
  ) {
    return false;
  }

  const toolRequests = getToolRequests(prev.message);
  for (const toolRequest of toolRequests) {
    const id = toolRequest.id;
    if (
      prev.toolResponsesById.get(id) !== next.toolResponsesById.get(id) ||
      prev.confirmationByToolRequestId.get(id) !== next.confirmationByToolRequestId.get(id) ||
      prev.pendingConfirmationIds.has(id) !== next.pendingConfirmationIds.has(id) ||
      prev.toolCallNotifications.get(id) !== next.toolCallNotifications.get(id)
    ) {
      return false;
    }
  }

  const confirmationData = getAnyToolConfirmationData(prev.message);
  if (confirmationData) {
    return (
      prev.toolRequestIds.has(confirmationData.id) === next.toolRequestIds.has(confirmationData.id)
    );
  }

  return true;
}

export default memo(GoslingMessage, areGoslingMessagePropsEqual);
