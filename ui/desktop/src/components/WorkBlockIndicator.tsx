import { useMemo } from 'react';
import { ChevronRight } from 'lucide-react';
import { useReasoningDetail, WorkBlockDetail } from '../contexts/ReasoningDetailContext';
import { Message } from '../api';
import { getToolRequests, getTextAndImageContent } from '../types/message';
import FlyingBird from './FlyingBird';
import GooseLogo from './GooseLogo';

/**
 * Extract a short one-liner summary from messages for preview.
 */
function extractOneLiner(messages: Message[]): string {
  // Iterate in reverse to always show the LATEST narrative message
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    const { textContent } = getTextAndImageContent(msg);
    const line = textContent?.trim();
    if (line && line.length > 0) {
      const firstLine = line.split('\n').find((l: string) => l.trim().length > 0) || '';
      return firstLine.length > 120 ? firstLine.slice(0, 117) + '…' : firstLine;
    }
  }
  return 'Working on your request';
}

/**
 * Count total tool calls across messages.
 */
function countToolCalls(messages: Message[]): number {
  let count = 0;
  for (const msg of messages) {
    count += getToolRequests(msg).length;
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
  const { toggleWorkBlock, panelDetail, isOpen } = useReasoningDetail();

  const oneLiner = useMemo(() => extractOneLiner(messages), [messages]);
  const toolCount = useMemo(() => countToolCalls(messages), [messages]);

  const isActive =
    isOpen && panelDetail?.type === 'workblock' && panelDetail.data.messageId === blockId;

  const handleClick = () => {
    const detail: WorkBlockDetail = {
      title: isStreaming ? 'Goose is working on it…' : `Worked on ${messages.length} steps`,
      messageId: blockId,
      messages: messages,
      toolCount,
      agentName,
      modeName,
      sessionId,
      toolCallNotifications: toolCallNotifications as Map<string, unknown[]> | undefined,
    };
    toggleWorkBlock(detail);
  };

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
