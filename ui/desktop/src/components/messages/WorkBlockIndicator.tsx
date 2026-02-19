import { ChevronRight } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { Message } from '../../api';
import { useReasoningDetail, type WorkBlockDetail } from '../../contexts/ReasoningDetailContext';
import FlyingBird from '../branding/FlyingBird';
import GooseLogo from '../branding/GooseLogo';

/**
 * Extract a one-liner summary from the last assistant message with text content.
 */
function extractOneLiner(messages: Message[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    for (const c of msg.content) {
      if (c.type === 'text' && c.text?.trim()) {
        const clean = c.text.replace(/<[^>]*>/g, '').trim();
        return clean.length > 120 ? `${clean.slice(0, 117)}…` : clean;
      }
    }
  }
  return '';
}

function countToolCalls(messages: Message[]): number {
  let count = 0;
  for (const msg of messages) {
    for (const c of msg.content) {
      if (c.type === 'toolRequest') count++;
    }
  }
  return count;
}

interface WorkBlockIndicatorProps {
  messages: Message[];
  blockId: string;
  isStreaming: boolean;
  agentName?: string;
  modeName?: string;
  sessionId?: string;
  toolCallNotifications?: Map<string, unknown[]>;
}

export default function WorkBlockIndicator({
  messages,
  blockId,
  isStreaming,
  agentName,
  modeName,
  sessionId,
  toolCallNotifications,
}: WorkBlockIndicatorProps) {
  const { toggleWorkBlock, panelDetail, isOpen, updateWorkBlock, closeDetail } =
    useReasoningDetail();

  const hasAutoOpened = useRef(false);

  const oneLiner = useMemo(() => extractOneLiner(messages), [messages]);
  const toolCount = useMemo(() => countToolCalls(messages), [messages]);

  const isActive =
    isOpen && panelDetail?.type === 'workblock' && panelDetail.data.messageId === blockId;

  // Refs keep latest values accessible from effects without adding deps
  const latestRef = useRef({
    messages,
    toolCount,
    isStreaming,
    toolCallNotifications,
    isActive,
  });
  latestRef.current = { messages, toolCount, isStreaming, toolCallNotifications, isActive };

  const buildDetail = useCallback(
    (): WorkBlockDetail => ({
      title: latestRef.current.isStreaming
        ? 'Goose is working on it…'
        : `Worked on ${latestRef.current.messages.length} steps`,
      messageId: blockId,
      messages: latestRef.current.messages,
      toolCount: latestRef.current.toolCount,
      isStreaming: latestRef.current.isStreaming,
      agentName,
      modeName,
      sessionId,
      toolCallNotifications: latestRef.current.toolCallNotifications as
        | Map<string, unknown[]>
        | undefined,
    }),
    [blockId, agentName, modeName, sessionId]
  );

  const handleClick = () => {
    toggleWorkBlock(buildDetail());
  };

  // Auto-open the panel when streaming starts (once per block)
  useEffect(() => {
    if (isStreaming && messages.length > 0 && !hasAutoOpened.current) {
      hasAutoOpened.current = true;
      toggleWorkBlock(buildDetail());
    }
  }, [isStreaming, messages.length, toggleWorkBlock, buildDetail]);

  // Live-update the panel content when messages or tool count change during streaming.
  // messages.length and toolCount are value-based change detectors — they trigger
  // this effect when content meaningfully changes. buildDetail reads from latestRef.
  // biome-ignore lint/correctness/useExhaustiveDependencies: messages.length and toolCount are intentional change detectors
  useEffect(() => {
    if (isActive && isStreaming) {
      updateWorkBlock(buildDetail());
    }
  }, [messages.length, toolCount, isStreaming, isActive, updateWorkBlock, buildDetail]);

  // Auto-close the panel when streaming ends so the final answer is visible
  const prevStreamingRef = useRef(isStreaming);
  useEffect(() => {
    if (prevStreamingRef.current && !isStreaming && isActive) {
      closeDetail();
    }
    prevStreamingRef.current = isStreaming;
  }, [isStreaming, isActive, closeDetail]);

  const displayAgent = agentName || 'Goose Agent';
  const displayMode = modeName || 'assistant';

  return (
    <div className="py-1.5 px-2">
      <button
        type="button"
        onClick={handleClick}
        className={`
          flex items-center gap-2.5 px-3 py-2.5 rounded-lg w-full text-left
          transition-colors duration-150 cursor-pointer
          ${isActive ? 'bg-background-muted' : 'hover:bg-background-muted/50'}
        `}
      >
        {/* Animated goose icon */}
        <div className="shrink-0">
          {isStreaming ? (
            <FlyingBird className="flex-shrink-0" cycleInterval={150} />
          ) : (
            <GooseLogo size="small" hover={false} />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 flex-wrap">
            <span className="text-xs font-medium text-text-default">
              {isStreaming ? 'Goose is working on it…' : `Worked on ${messages.length} steps`}
            </span>
            {toolCount > 0 && !isStreaming && (
              <>
                <span className="text-xs text-text-muted/50 mx-0.5">·</span>
                <span className="text-xs text-text-muted/70">
                  {toolCount} tool{toolCount !== 1 ? 's' : ''} used
                </span>
              </>
            )}
          </div>
          <div className="flex items-center gap-1 mt-0.5">
            <span className="text-xs font-medium text-blue-500">{displayAgent}</span>
            <span className="text-xs text-text-muted/40">/</span>
            <span className="text-xs font-medium text-blue-500">{displayMode}</span>
            {oneLiner && (
              <>
                <span className="text-xs text-text-muted/40 mx-0.5">·</span>
                <span className="text-xs text-text-muted/50 truncate">{oneLiner}</span>
              </>
            )}
          </div>
        </div>

        {/* Chevron */}
        <ChevronRight
          size={14}
          className={`shrink-0 text-text-muted/40 transition-transform ${isActive ? 'rotate-90' : ''}`}
        />
      </button>
    </div>
  );
}
