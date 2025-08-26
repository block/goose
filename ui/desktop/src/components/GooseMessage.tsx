import { useEffect, useMemo, useRef } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import { formatMessageTimestamp } from '../utils/timeUtils';
import MarkdownContent from './MarkdownContent';
import ToolCallWithResponse from './ToolCallWithResponse';
import ToolCallChain from './ToolCallChain';
import {
  identifyConsecutiveToolCalls,
  shouldHideMessage,
  getChainForMessage,
} from '../utils/toolCallChaining';
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
import MessageBranchLink from './MessageBranchLink';
import { NotificationEvent } from '../hooks/useMessageStream';
import { cn } from '../utils';
import BranchingIndicator from './BranchingIndicator';
import { GitBranch } from 'lucide-react';

interface GooseMessageProps {
  // messages up to this index are presumed to be "history" from a resumed session, this is used to track older tool confirmation requests
  // anything before this index should not render any buttons, but anything after should
  sessionId: string;
  messageHistoryIndex: number;
  message: Message;
  messages: Message[];
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  appendMessage: (message: Message) => void;
  isStreaming?: boolean; // Whether this message is currently being streamed
  onBranchFromMessage?: (messageId: string) => void;
  onSessionClick?: (sessionId: string) => void;
}

export default function GooseMessage({
  sessionId,
  messageHistoryIndex,
  message,
  metadata,
  messages,
  toolCallNotifications,
  append,
  appendMessage,
  isStreaming = false,
  onBranchFromMessage,
  onSessionClick,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);
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
    const visibleText = text.replace(regex, '').trim();

    return {
      visibleText,
      cotText: cotRaw || null,
    };
  };

  // Split out Chain-of-Thought
  const { visibleText, cotText } = splitChainOfThought(textContent);

  // Extract image paths from the message content
  const imagePaths = extractImagePaths(visibleText);

  // Remove image paths from text for display
  const displayText =
    imagePaths.length > 0 ? removeImagePathsFromText(visibleText, imagePaths) : visibleText;

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Get tool requests from the message
  const toolRequests = getToolRequests(message);

  // Get current message index
  const messageIndex = messages.findIndex((msg) => msg.id === message.id);

  // Enhanced chain detection that works during streaming
  const toolCallChains = useMemo(() => {
    // Always run chain detection, but handle streaming messages specially
    const chains = identifyConsecutiveToolCalls(messages);

    // If this message is streaming and has tool calls but no text,
    // check if it should extend an existing chain
    if (isStreaming && toolRequests.length > 0 && !displayText.trim()) {
      // Look for an existing chain that this message could extend
      const previousMessage = messageIndex > 0 ? messages[messageIndex - 1] : null;
      if (previousMessage) {
        const prevToolRequests = getToolRequests(previousMessage);

        // If previous message has tool calls (with or without text), extend its chain
        if (prevToolRequests.length > 0) {
          // Find if previous message is part of a chain
          const prevChain = chains.find((chain) => chain.includes(messageIndex - 1));
          if (prevChain) {
            // Extend the existing chain to include this streaming message
            const extendedChains = chains.map((chain) =>
              chain === prevChain ? [...chain, messageIndex] : chain
            );
            return extendedChains;
          } else {
            // Create a new chain with previous and current message
            return [...chains, [messageIndex - 1, messageIndex]];
          }
        }
      }
    }

    return chains;
  }, [messages, isStreaming, messageIndex, toolRequests, displayText]);

  // Check if this message should be hidden (part of chain but not first)
  const shouldHide = shouldHideMessage(messageIndex, toolCallChains);

  // Get the chain this message belongs to
  const messageChain = getChainForMessage(messageIndex, toolCallChains);

  // Extract URLs under a few conditions
  // 1. The message is purely text
  // 2. The link wasn't also present in the previous message
  // 3. The message contains the explicit http:// or https:// protocol at the beginning
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

  // Handle tool confirmations
  useEffect(() => {
    if (hasToolConfirmation && !handledToolConfirmations.current.has(toolConfirmationContent.id)) {
      handledToolConfirmations.current.add(toolConfirmationContent.id);

      const handleConfirmation = async (confirmed: boolean) => {
        if (confirmed) {
          // User confirmed, proceed with the tool call
          console.log(`Tool call ${toolConfirmationContent.id} confirmed`);
        } else {
          // User rejected, create an error response
          const errorResponse = createToolErrorResponseMessage(
            toolConfirmationContent.id,
            'Tool call was rejected by the user'
          );
          appendMessage(errorResponse);
        }
      };

      // Auto-handle if this is from message history (already processed)
      if (messageIndex < messageHistoryIndex) {
        handleConfirmation(true);
      }
    }
  }, [
    hasToolConfirmation,
    toolConfirmationContent,
    messageIndex,
    messageHistoryIndex,
    appendMessage,
  ]);

  // If this message should be hidden (part of chain but not first), don't render it
  if (shouldHide) {
    return null;
  }

  // Determine rendering logic based on chain membership and content
  const isFirstInChain = messageChain && messageChain[0] === messageIndex;

  // Check if this message has branching metadata
  const hasBranchingMetadata =
    message.branchingMetadata &&
    (message.branchingMetadata.branchedFrom ||
      (message.branchingMetadata.branchesCreated &&
        message.branchingMetadata.branchesCreated.length > 0));

  return (
    <div className="goose-message flex w-[90%] justify-start min-w-0">
      <div className="flex flex-col w-full min-w-0">
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

        {displayText && (
          <div className="flex flex-col group">
            <div ref={contentRef} className="w-full">
              <MarkdownContent content={displayText} />
            </div>

            {/* Image previews */}
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
                  <div className="text-xs font-mono text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 flex items-center gap-1">
                    {timestamp}
                    {/* Subtle branch icons when not hovering */}
                    {hasBranchingMetadata && (
                      <>
                        {message.branchingMetadata?.branchedFrom && (
                          <GitBranch 
                            className="h-3 w-3 opacity-60 rotate-180"
                          />
                        )}
                        {message.branchingMetadata?.branchesCreated && 
                         message.branchingMetadata.branchesCreated.length > 0 && (
                          <GitBranch 
                            className="h-3 w-3 opacity-60"
                          />
                        )}
                      </>
                    )}
                  </div>
                )}
                {message.content.every((content) => content.type === 'text') && !isStreaming && (
                  <div className="absolute left-0 pt-1 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                    <MessageCopyLink text={displayText} contentRef={contentRef} />
                    {/* Branch button */}
                    {onBranchFromMessage && message.id && (
                      <MessageBranchLink
                        onBranchFromMessage={onBranchFromMessage}
                        messageId={message.id}
                      />
                    )}
                    {/* Branching indicator in hover state */}
                    {hasBranchingMetadata && (
                      <BranchingIndicator
                        branchingMetadata={message.branchingMetadata!}
                        onSessionClick={onSessionClick}
                      />
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {toolRequests.length > 0 && (
          <div className={cn(displayText && 'mt-2')}>
            {isFirstInChain ? (
              <ToolCallChain
                messages={messages}
                chainIndices={messageChain}
                toolCallNotifications={toolCallNotifications}
                toolResponsesMap={toolResponsesMap}
                messageHistoryIndex={messageHistoryIndex}
                isStreaming={isStreaming}
              />
            ) : !messageChain ? (
              <div className="relative flex flex-col w-full">
                <div className="flex flex-col gap-3">
                  {toolRequests.map((toolRequest) => (
                    <div className="goose-message-tool" key={toolRequest.id}>
                      <ToolCallWithResponse
                        isCancelledMessage={
                          messageIndex < messageHistoryIndex &&
                          toolResponsesMap.get(toolRequest.id) == undefined
                        }
                        toolRequest={toolRequest}
                        toolResponse={toolResponsesMap.get(toolRequest.id)}
                        notifications={toolCallNotifications.get(toolRequest.id)}
                        isStreamingMessage={isStreaming}
                        append={append}
                      />
                    </div>
                  ))}
                </div>
                <div className="text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 flex items-center gap-1">
                  {!isStreaming && timestamp}
                  {/* Subtle branch icons when not hovering */}
                  {hasBranchingMetadata && (
                    <>
                      {message.branchingMetadata?.branchedFrom && (
                        <GitBranch 
                          className="h-3 w-3 opacity-60 rotate-180"
                        />
                      )}
                      {message.branchingMetadata?.branchesCreated && 
                       message.branchingMetadata.branchesCreated.length > 0 && (
                        <GitBranch 
                          className="h-3 w-3 opacity-60"
                        />
                      )}
                    </>
                  )}
                </div>
                {/* Hover state for tool requests */}
                <div className="absolute left-0 top-1 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                  {/* Branch button */}
                  {onBranchFromMessage && message.id && (
                    <MessageBranchLink
                      onBranchFromMessage={onBranchFromMessage}
                      messageId={message.id}
                    />
                  )}
                  {/* Branching indicator in hover state */}
                  {hasBranchingMetadata && (
                    <BranchingIndicator
                      branchingMetadata={message.branchingMetadata!}
                      onSessionClick={onSessionClick}
                    />
                  )}
                </div>
              </div>
            ) : null}
          </div>
        )}

        {hasToolConfirmation && (
          <ToolCallConfirmation
            sessionId={sessionId}
            isCancelledMessage={messageIndex == messageHistoryIndex - 1}
            isClicked={messageIndex < messageHistoryIndex}
            toolConfirmationId={toolConfirmationContent.id}
            toolName={toolConfirmationContent.toolName}
          />
        )}
      </div>

      {/* TODO(alexhancock): Re-enable link previews once styled well again */}
      {urls.length > 0 && (
        <div className="mt-4">
          {urls.map((url, index) => (
            <LinkPreview key={index} url={url} />
          ))}
        </div>
      )}

      {/* enable or disable prompts here */}
      {/* NOTE from alexhancock on 1/14/2025 - disabling again temporarily due to non-determinism in when the forms show up */}
      {/* eslint-disable-next-line no-constant-binary-expression */}
      {false && metadata && (
        <div className="flex mt-[16px]">
          <GooseResponseForm message={displayText} metadata={metadata || null} append={append} />
        </div>
      )}
    </div>
  );
}
