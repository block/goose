import { useMemo, useEffect, useRef } from 'react';
import { ChevronRight } from 'lucide-react';
import { useReasoningDetail, type WorkBlockDetail } from '../../contexts/ReasoningDetailContext';
import type { Message } from '../../api';
import FlyingBird from '../branding/FlyingBird';
import GooseLogo from '../branding/GooseLogo';

/**
 * Extract a one-liner summary from the last assistant message with text content.
 * Used as a preview in the collapsed work block indicator.
 */
function extractOneLiner(messages: Message[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    for (const c of msg.content) {
      if (c.type === 'text' && c.text?.trim()) {
        const clean = c.text.replace(/<[^>]*>/g, '').trim();
        return clean.length > 120 ? clean.slice(0, 117) + '…' : clean;
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
  const { toggleWorkBlock, panelDetail, isOpen, updateWorkBlock } = useReasoningDetail();

  const hasAutoOpened = useRef(false);

  const oneLiner = useMemo(() => extractOneLiner(messages), [messages]);
  const toolCount = useMemo(() => countToolCalls(messages), [messages]);

  const isActive =
    isOpen && panelDetail?.type === 'workblock' && panelDetail.data.messageId === blockId;

  const buildDetail = (): WorkBlockDetail => ({
    title: isStreaming ? 'Goose is working on it…' : `Worked on ${messages.length} steps`,
    messageId: blockId,
    messages,
    toolCount,
    isStreaming,
    agentName,
    modeName,
    sessionId,
    toolCallNotifications: toolCallNotifications as Map<string, unknown[]> | undefined,
  });

  const handleClick = () => {
    toggleWorkBlock(buildDetail());
  };

  // Auto-open the panel when streaming starts (once per block)
  useEffect(() => {
    if (isStreaming && messages.length > 0 && !hasAutoOpened.current) {
      hasAutoOpened.current = true;
      toggleWorkBlock(buildDetail());
    }
  }, [isStreaming, messages.length]); // eslint-disable-line react-hooks/exhaustive-deps

  // Live-update the panel content when messages change during streaming
  useEffect(() => {
    if (isActive && isStreaming && updateWorkBlock) {
      updateWorkBlock(buildDetail());
    }
  }, [messages, isStreaming, isActive]); // eslint-disable-line react-hooks/exhaustive-deps

  const displayAgent = agentName || 'Goose Agent';
  const displayMode = modeName || 'assistant';

  return (
    <div className="py-1.5 px-2">
      <button
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
