import { useEffect, useMemo, useRef } from 'react';
import ImagePreview from './ImagePreview';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import { formatMessageTimestamp } from '../utils/timeUtils';
import MarkdownContent from './MarkdownContent';
import CommentableMarkdown from './CommentableMarkdown';
import MessageComments from './MessageComments';
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
import { NotificationEvent } from '../hooks/useMessageStream';
import { MessageComment, TextSelection } from '../types/comment';
import { cn } from '../utils';
import AvatarImage from './AvatarImage';

interface GooseMessageProps {
  // messages up to this index are presumed to be "history" from a resumed session, this is used to track older tool confirmation requests
  // anything before this index should not render any buttons, but anything after should
  sessionId: string;
  messageHistoryIndex: number;
  message: Message;
  messages: Message[];
  messageIndex?: number; // Optional: the index of this message in the messages array (for tool chaining)
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  appendMessage: (message: Message) => void;
  isStreaming?: boolean; // Whether this message is currently being streamed
  // Comment-related props
  comments?: MessageComment[];
  activeSelection?: TextSelection | null;
  activePosition?: { x: number; y: number } | null;
  activeMessageId?: string | null;
  isCreatingComment?: boolean;
  onSelectionChange?: (selection: TextSelection | null, position?: { x: number; y: number }, messageId?: string) => void;
  onCreateComment?: (messageId: string, selection: TextSelection, content: string) => void;
  onUpdateComment?: (commentId: string, content: string) => void;
  onDeleteComment?: (commentId: string) => void;
  onReplyToComment?: (parentId: string, content: string) => void;
  onResolveComment?: (commentId: string, resolved: boolean) => void;
  onCancelComment?: () => void;
  onFocusComment?: (commentId: string) => void;
}

export default function GooseMessage({
  sessionId,
  messageHistoryIndex,
  message,
  messages,
  messageIndex: passedMessageIndex,
  toolCallNotifications,
  append,
  appendMessage,
  isStreaming = false,
  // Comment props
  comments = [],
  activeSelection,
  activePosition,
  activeMessageId,
  isCreatingComment = false,
  onSelectionChange,
  onCreateComment,
  onUpdateComment,
  onDeleteComment,
  onReplyToComment,
  onResolveComment,
  onCancelComment,
  onFocusComment,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);
  // Track which tool confirmations we've already handled to prevent infinite loops
  const handledToolConfirmations = useRef<Set<string>>(new Set());

  // Extract text content from the message
  let textContent = getTextContent(message);

  // Use passed message index or fallback to finding it
  const messageIndex = passedMessageIndex !== undefined ? passedMessageIndex : messages.findIndex((msg) => msg.id === message.id);
  
  // Create a unique message ID that combines multiple factors for uniqueness
  const uniqueMessageId = useMemo(() => {
    if (message.id) return message.id;
    
    // Create a unique ID based on message content, timestamp, and index
    const contentHash = message.content
      .map(c => c.type === 'text' ? c.text.slice(0, 50) : c.type)
      .join('|');
    const timestamp = message.created;
    const role = message.role;
    
    // Create a hash-like ID that's more unique
    const uniqueId = `${role}-${timestamp}-${messageIndex}-${contentHash.length}`;
    return uniqueId;
  }, [message.id, message.content, message.created, message.role, messageIndex]);

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

  // If this message should be hidden (part of chain but not first), don't render it
  if (shouldHide) {
    return null;
  }

  // Determine rendering logic based on chain membership and content
  const isFirstInChain = messageChain && messageChain[0] === messageIndex;

  // Get sender info for Goose - use message sender if available, otherwise default to "Goose"
  const senderInfo = message.sender || {
    userId: 'goose',
    displayName: 'Goose',
    avatarUrl: null,
  };

  // Check if this is from a collaborator (has sender info and is from Matrix)
  const isFromCollaborator = message.sender && (message as any).metadata?.isFromCollaborator;

  return (
    <div className="goose-message flex w-full justify-start min-w-0 gap-3 relative">
      {/* Goose Avatar on the left side with optional user badge */}
      <div className="flex-shrink-0 relative" style={{ marginTop: '0.25rem' }}>
        {/* Main Goose dot avatar */}
        <div className="w-8 h-8 flex items-center justify-center">
          <div className="w-2 h-2 bg-gray-900 dark:bg-gray-100 rounded-full"></div>
        </div>
        
        {/* Small user badge for collaborator messages - positioned to the left */}
        {isFromCollaborator && senderInfo.avatarUrl && (
          <div className="absolute top-0 w-4 h-4 rounded-full border-2 border-background-default overflow-hidden bg-background-default" style={{ right: '20px' }}>
            <AvatarImage
              avatarUrl={senderInfo.avatarUrl}
              displayName={senderInfo.displayName || 'User'}
              size="xs"
            />
          </div>
        )}
        {isFromCollaborator && !senderInfo.avatarUrl && (
          <div className="absolute top-0 w-4 h-4 rounded-full border-2 border-background-default bg-blue-500 flex items-center justify-center" style={{ right: '20px' }}>
            <span className="text-[8px] text-white font-bold">
              {(senderInfo.displayName || senderInfo.userId || 'U').charAt(0).toUpperCase()}
            </span>
          </div>
        )}
      </div>
      
      {/* Message content - back to original single column layout */}
      <div className="flex flex-col flex-1 min-w-0">
        {/* Username and timestamp header - only show if we have text content or it's the start of a chain */}
        {(displayText || isFirstInChain) && (
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-semibold text-text-prominent">
              {isFromCollaborator ? `Goose (${senderInfo.displayName || 'User'})` : 'Goose'}
            </span>
            <span className="text-xs text-text-muted font-mono">
              {timestamp}
            </span>
          </div>
        )}

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
              <CommentableMarkdown
                content={displayText}
                messageId={uniqueMessageId}
                comments={comments}
                onSelectionChange={onSelectionChange}
                onCreateComment={onSelectionChange} // This triggers the comment input to show
                onFocusComment={onFocusComment}
              />
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
              </div>
            ) : null}
          </div>
        )}

        {hasToolConfirmation && (
          <ToolCallConfirmation
            sessionId={sessionId}
            isCancelledMessage={messageIndex == messageHistoryIndex - 1}
            isClicked={messageIndex < messageHistoryIndex}
            toolConfirmationContent={toolConfirmationContent}
          />
        )}

      </div>

      {/* Floating comments - positioned absolutely to the right */}
      {/* Only show if this message has existing comments OR if this is the active message for commenting */}
      {((comments && comments.length > 0) || (isCreatingComment && activeMessageId === uniqueMessageId)) && (
        <div className="absolute left-full ml-4 top-0 w-80 z-10">
          <MessageComments
            messageId={uniqueMessageId}
            comments={comments || []}
            activeSelection={activeSelection}
            activePosition={activePosition}
            isCreatingComment={isCreatingComment && activeMessageId === uniqueMessageId}
            onCreateComment={onCreateComment || (() => {})}
            onUpdateComment={onUpdateComment || (() => {})}
            onDeleteComment={onDeleteComment || (() => {})}
            onReplyToComment={onReplyToComment || (() => {})}
            onResolveComment={onResolveComment || (() => {})}
            onCancelComment={onCancelComment || (() => {})}
          />
        </div>
      )}
    </div>
  );
}
