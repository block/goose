import { useEffect, useMemo, useRef, useState } from 'react';
import ImagePreview from '../shared/ImagePreview';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import MarkdownContent from '../messages/MarkdownContent';
import ToolCallWithResponse from '../messages/ToolCallWithResponse';
import { Brain, ChevronRight } from 'lucide-react';
import {
  getTextAndImageContent,
  getToolRequests,
  getToolResponses,
  getToolConfirmationContent,
  getElicitationContent,
  getPendingToolConfirmationIds,
  getAnyToolConfirmationData,
} from '../../types/message';
import type {
  MessageWithAttribution,
  ToolConfirmationData,
  NotificationEvent,
} from '../../types/message';
import type { Message } from '../../api';
import ToolCallConfirmation from '../messages/ToolCallConfirmation';
import ElicitationRequest from '../messages/ElicitationRequest';
import MessageCopyLink from '../messages/MessageCopyLink';
import { cn } from '../../utils';
import { identifyConsecutiveToolCalls, shouldHideTimestamp } from '../../utils/toolCallChaining';
import { AppEvents } from '../../constants/events';
import { useReasoningDetail } from '../../contexts/ReasoningDetailContext';
import {
  extractGenerativeSpec,
  hasPartialGenerativeSpec,
  stripPartialGenerativeSpec,
} from '../../utils/generativeSpec';
import { GooseGenerativeUI } from '../ui/design-system/goose-renderer';
import { useNavigate } from 'react-router-dom';

function ThinkingSection({
  cotText,
  isStreaming,
  messageId,
}: {
  cotText: string;
  isStreaming: boolean;
  messageId?: string;
}) {
  const {
    toggleDetail,
    openDetail,
    updateContent,
    isOpen: isPanelOpen,
    detail,
  } = useReasoningDetail();
  const hasAutoOpened = useRef(false);
  const preview =
    cotText
      .split('\n')
      .find((l) => l.trim())
      ?.slice(0, 80) || 'Reasoning...';
  const isThisMessageOpen = isPanelOpen && detail?.messageId === messageId;

  // Auto-open reasoning panel during streaming and live-update content
  useEffect(() => {
    if (isStreaming && cotText.length > 0) {
      if (!hasAutoOpened.current) {
        hasAutoOpened.current = true;
        openDetail({ title: 'Thinking...', content: cotText, messageId: messageId ?? '' });
      } else if (isThisMessageOpen) {
        updateContent(cotText);
      }
    }
    if (!isStreaming && hasAutoOpened.current) {
      hasAutoOpened.current = false;
      if (isThisMessageOpen) {
        updateContent(cotText);
      }
    }
  }, [isStreaming, cotText, messageId, openDetail, updateContent, isThisMessageOpen]);

  const handleClick = () => {
    toggleDetail({
      title: isStreaming ? 'Thinking...' : 'Thought process',
      content: cotText,
      messageId: messageId ?? '',
    });
  };

  return (
    <div className="mb-2">
      <button
        onClick={handleClick}
        className={cn(
          'flex items-center gap-2 px-3 py-2 rounded-lg border transition-colors select-none group',
          isStreaming
            ? 'bg-background-muted/50 border-border-default/50 hover:bg-background-muted cursor-pointer'
            : 'bg-background-muted/50 border-border-default/50 hover:bg-background-muted cursor-pointer',
          isThisMessageOpen && 'bg-background-muted border-border-default'
        )}
      >
        <Brain
          size={16}
          className={cn('text-text-muted shrink-0', isStreaming && 'animate-pulse text-amber-400')}
        />
        <span className="text-sm font-medium text-text-muted">
          {isStreaming ? 'Thinking...' : 'Thought process'}
        </span>
        {!isStreaming && (
          <span className="text-xs text-text-muted/60 truncate text-left max-w-[300px]">
            — {preview}
          </span>
        )}
        <ChevronRight
          size={14}
          className={cn(
            'text-text-muted/50 shrink-0 transition-transform duration-200',
            isThisMessageOpen && 'rotate-90'
          )}
        />
      </button>
    </div>
  );
}

interface GooseMessageProps {
  sessionId: string;
  message: Message;
  messages: Message[];
  metadata?: string[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  append: (value: string) => void;
  isStreaming: boolean;
  suppressToolCalls?: boolean;
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
  isStreaming,
  suppressToolCalls,
  submitElicitationResponse,
}: GooseMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);
  const [responseStyle, setResponseStyle] = useState(() => localStorage.getItem('response_style'));

  useEffect(() => {
    const handleStyleChange = () => {
      setResponseStyle(localStorage.getItem('response_style'));
    };
    window.addEventListener('storage', handleStyleChange);
    window.addEventListener(AppEvents.RESPONSE_STYLE_CHANGED, handleStyleChange);
    return () => {
      window.removeEventListener('storage', handleStyleChange);
      window.removeEventListener(AppEvents.RESPONSE_STYLE_CHANGED, handleStyleChange);
    };
  }, []);

  const hideToolCalls = responseStyle === 'hidden';

  let { textContent, imagePaths } = getTextAndImageContent(message);

  const stripInternalTags = (text: string, streaming: boolean): string => {
    let cleaned = text
      // Strip complete <tool_call>...</tool_call> and <tool_result>...</tool_result> XML tags
      .replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '')
      .replace(/<tool_result>[\s\S]*?<\/tool_result>/gi, '');

    if (streaming) {
      // During streaming, also strip incomplete/partial tool call tags that haven't closed yet
      cleaned = cleaned.replace(/<tool_call>[\s\S]*$/gi, '').replace(/<tool_result>[\s\S]*$/gi, '');

      // Strip partial JSON tool call fragments that appear during streaming
      // e.g., 'developer.shell", "arguments": {"command": "cd ...'
      // These are fragments of tool_use blocks being streamed as text
      cleaned = cleaned.replace(/[a-zA-Z_]+\.\w+",\s*"arguments":\s*\{[\s\S]*$/g, '');
      // Also strip Ollama-style XML function calls: <function=name><parameter=...>
      cleaned = cleaned.replace(/<function=[\s\S]*$/gi, '');
    }

    return cleaned.trim();
  };

  const splitChainOfThought = (
    text: string,
    streaming: boolean
  ): { displayText: string; cotText: string | null } => {
    const regex = /<think>([\s\S]*?)<\/think>/i;
    const match = text.match(regex);
    if (!match) {
      return { displayText: stripInternalTags(text, streaming), cotText: null };
    }

    const cotRaw = match[1].trim();
    const displayText = stripInternalTags(text.replace(regex, '').trim(), streaming);

    return {
      displayText,
      cotText: cotRaw || null,
    };
  };

  const { displayText, cotText } = splitChainOfThought(textContent, isStreaming);

  const navigate = useNavigate();

  // Generative UI: detect and extract json-render specs from message text
  const generativeResult = useMemo(() => {
    if (!displayText.trim()) return null;

    // During streaming, if a partial spec is detected, strip it and don't render
    if (isStreaming && hasPartialGenerativeSpec(displayText)) {
      return { partial: true as const, cleanText: stripPartialGenerativeSpec(displayText) };
    }

    const extracted = extractGenerativeSpec(displayText);
    if (extracted) {
      return { partial: false as const, ...extracted };
    }
    return null;
  }, [displayText, isStreaming]);

  const handleGenerativeAction = useMemo(() => {
    return (actionName: string, params?: Record<string, unknown>) => {
      switch (actionName) {
        case 'navigate':
          if (params?.path) navigate(params.path as string);
          break;
        case 'create_session':
          window.dispatchEvent(new CustomEvent('create-new-session', { detail: params }));
          break;
        case 'open_session':
          if (params?.sessionId) navigate(`/sessions/${params.sessionId}`);
          break;
        case 'run_recipe':
          if (params?.recipeId) navigate(`/recipes/${params.recipeId}`);
          break;
        case 'run_eval':
          if (params?.datasetId) navigate(`/evaluate?dataset=${params.datasetId}`);
          break;
        case 'install_extension':
          if (params?.name) {
            window.dispatchEvent(new CustomEvent('install-extension', { detail: params }));
          }
          break;
      }
    };
  }, [navigate]);

  // Determine the text to actually render as markdown
  const renderedText = useMemo(() => {
    if (!generativeResult) return displayText;
    if (generativeResult.partial) return generativeResult.cleanText;
    return generativeResult.beforeText;
  }, [displayText, generativeResult]);

  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);
  const modelInfo = (message as MessageWithAttribution)._modelInfo;
  const routingInfo = (message as MessageWithAttribution)._routingInfo;
  const toolRequests = getToolRequests(message);
  const messageIndex = messages.findIndex((msg) => msg.id === message.id);

  const toolConfirmationContent = getToolConfirmationContent(message);
  const elicitationContent = getElicitationContent(message);

  const findConfirmationForToolAcrossMessages = (
    toolRequestId: string
  ): ToolConfirmationData | undefined => {
    for (const msg of messages) {
      const confirmationData = getAnyToolConfirmationData(msg);
      if (confirmationData && confirmationData.id === toolRequestId) {
        return confirmationData;
      }
    }
    return undefined;
  };
  const toolCallChains = useMemo(() => identifyConsecutiveToolCalls(messages), [messages]);
  const hideTimestamp = useMemo(
    () => shouldHideTimestamp(messageIndex, toolCallChains),
    [messageIndex, toolCallChains]
  );
  const hasToolConfirmation = toolConfirmationContent !== undefined;
  const hasElicitation = elicitationContent !== undefined;

  const toolConfirmationShownInline = useMemo(() => {
    if (!toolConfirmationContent) return false;
    const confirmationData = getAnyToolConfirmationData(message);
    if (!confirmationData) return false;

    for (const msg of messages) {
      const requests = getToolRequests(msg);
      if (requests.some((req) => req.id === confirmationData.id)) {
        return true;
      }
    }
    return false;
  }, [toolConfirmationContent, message, messages]);

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

  // In hidden mode, if message has only tool calls (no text, images, thinking),
  // show a minimal indicator with routing info instead of the full tool call panels.
  // This ensures the user sees that work is being done and which agent is handling it.
  const isToolOnlyMessage =
    hideToolCalls &&
    !displayText.trim() &&
    imagePaths.length === 0 &&
    !cotText &&
    !hasToolConfirmation &&
    !hasElicitation &&
    toolRequests.length > 0 &&
    toolRequests.every((req) => !pendingConfirmationIds.has(req.id));

  if (isToolOnlyMessage && !isStreaming) {
    // For completed tool-only messages in hidden mode, show routing info if available
    if (!routingInfo || routingInfo.agentName === 'Goose Agent') {
      return null;
    }
    // Show just the agent badge for non-default agents
    return (
      <div className="goose-message flex w-[90%] justify-start min-w-0">
        <div className="flex flex-col w-full min-w-0">
          <div className="flex items-center gap-1.5 mb-1">
            <div className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-blue-500/10 border border-blue-500/20">
              <span className="text-xs font-medium text-blue-400">{routingInfo.agentName}</span>
              <span className="text-xs text-blue-300/70">›</span>
              <span className="text-xs text-blue-300">{routingInfo.modeSlug}</span>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="goose-message flex w-[90%] justify-start min-w-0">
      <div className="flex flex-col w-full min-w-0">
        {cotText && (
          <ThinkingSection
            cotText={cotText}
            isStreaming={isStreaming && !displayText.trim()}
            messageId={message.id ?? undefined}
          />
        )}

        {routingInfo && routingInfo.agentName !== 'Goose Agent' && (
          <div className="flex items-center gap-1.5 mb-1">
            <div className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-blue-500/10 border border-blue-500/20">
              <span className="text-xs font-medium text-blue-400">{routingInfo.agentName}</span>
              <span className="text-xs text-blue-300/70">›</span>
              <span className="text-xs text-blue-300">{routingInfo.modeSlug}</span>
            </div>
          </div>
        )}

        {(renderedText.trim() ||
          imagePaths.length > 0 ||
          (generativeResult && !generativeResult.partial)) && (
          <div className="flex flex-col group">
            {renderedText.trim() && (
              <div ref={contentRef} className="w-full">
                <MarkdownContent content={renderedText} />
              </div>
            )}

            {generativeResult && !generativeResult.partial && (
              <div className="mt-3 rounded-xl border border-border-default bg-background-default overflow-hidden">
                <GooseGenerativeUI
                  spec={generativeResult.spec}
                  onAction={handleGenerativeAction}
                  loading={isStreaming}
                />
              </div>
            )}

            {generativeResult && !generativeResult.partial && generativeResult.afterText && (
              <div className="mt-3 w-full">
                <MarkdownContent content={generativeResult.afterText} />
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
                    {routingInfo && (
                      <>
                        <span className="mx-1 opacity-50">·</span>
                        <span className="text-blue-400">{routingInfo.agentName}</span>
                        <span className="mx-1 opacity-50">›</span>
                        <span className="text-blue-300">{routingInfo.modeSlug}</span>
                      </>
                    )}
                    {modelInfo && (
                      <>
                        <span className="mx-1 opacity-50">·</span>
                        <span>{modelInfo.model}</span>
                      </>
                    )}
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

        {toolRequests.length > 0 &&
          (() => {
            // When suppressToolCalls is set (work block final answer), hide all non-pending tool calls
            // In hidden response style mode, also hide completed tool calls
            const shouldHideCompleted = hideToolCalls || suppressToolCalls;
            const visibleToolRequests = shouldHideCompleted
              ? toolRequests.filter((req) => pendingConfirmationIds.has(req.id))
              : toolRequests;
            if (visibleToolRequests.length === 0) return null;
            return (
              <div className={cn(displayText && 'mt-2')}>
                <div className="relative flex flex-col w-full">
                  <div className="flex flex-col gap-3">
                    {visibleToolRequests.map((toolRequest) => {
                      const hasResponse = toolResponsesMap.has(toolRequest.id);
                      const isPending = pendingConfirmationIds.has(toolRequest.id);
                      const confirmationContent = findConfirmationForToolAcrossMessages(
                        toolRequest.id
                      );
                      const isApprovalClicked = confirmationContent && !isPending && hasResponse;
                      return (
                        <div className="goose-message-tool" key={toolRequest.id}>
                          <ToolCallWithResponse
                            sessionId={sessionId}
                            isCancelledMessage={false}
                            toolRequest={toolRequest}
                            toolResponse={toolResponsesMap.get(toolRequest.id)}
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
                  <div className="text-xs text-text-muted transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 pt-1">
                    {!isStreaming && !hideTimestamp && timestamp}
                  </div>
                </div>
              </div>
            );
          })()}

        {hasToolConfirmation && !toolConfirmationShownInline && (
          <ToolCallConfirmation
            sessionId={sessionId}
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
