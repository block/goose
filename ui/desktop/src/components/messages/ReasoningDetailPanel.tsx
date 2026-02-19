import { Brain, Wrench, X } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { useReasoningDetail } from '../../contexts/ReasoningDetailContext';
import { cn } from '../../utils';
import GooseMessage from '../chat/GooseMessage';
import { Badge } from '../ui/atoms/Badge';
import { StatusDot } from '../ui/atoms/StatusDot';
import { ScrollArea } from '../ui/atoms/scroll-area';
import MarkdownContent from './MarkdownContent';

// Stable empty Map to avoid creating new identity on every render
const EMPTY_TOOL_NOTIFICATIONS = new Map();
const NOOP = () => {};

export default function ReasoningDetailPanel() {
  const { detail, panelDetail, isOpen, closeDetail } = useReasoningDetail();
  const bottomRef = useRef<HTMLDivElement>(null);
  const rafId = useRef<number | null>(null);

  const isWorkBlock = panelDetail?.type === 'workblock';
  const isReasoning = panelDetail?.type === 'reasoning' || (!panelDetail && detail);

  const isWorkBlockStreaming = isWorkBlock && (panelDetail.data.isStreaming ?? false);
  const isReasoningStreaming = isReasoning && detail?.title === 'Thinking...';
  const isLiveStreaming = isWorkBlockStreaming || isReasoningStreaming;

  const title = isWorkBlock ? panelDetail.data.title || 'Activity' : detail?.title || 'Details';

  // Only show agent/mode badge when it's a non-default agent
  const showAgentBadge =
    isWorkBlock && panelDetail.data.agentName && panelDetail.data.agentName !== 'Goose';

  // Auto-scroll during streaming using rAF to avoid smooth-scroll + Radix reflow loop
  useEffect(() => {
    if (!isLiveStreaming) return;

    const tick = () => {
      bottomRef.current?.scrollIntoView({ behavior: 'auto', block: 'end' });
      rafId.current = requestAnimationFrame(tick);
    };

    rafId.current = requestAnimationFrame(tick);

    return () => {
      if (rafId.current) cancelAnimationFrame(rafId.current);
      rafId.current = null;
    };
  }, [isLiveStreaming]);

  return (
    <div
      className={cn(
        'h-full border-l border-border-default bg-background-default flex flex-col transition-[width,opacity] duration-300 ease-in-out overflow-hidden',
        isOpen ? 'w-[400px] min-w-[400px] opacity-100' : 'w-0 min-w-0 opacity-0'
      )}
    >
      {isOpen && (
        <>
          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border-default shrink-0">
            <div className="flex items-center gap-2 min-w-0">
              {isWorkBlock ? (
                <Wrench
                  size={16}
                  className={cn(
                    'shrink-0',
                    isLiveStreaming ? 'text-blue-400 animate-pulse' : 'text-text-muted'
                  )}
                />
              ) : (
                <Brain
                  size={16}
                  className={cn(
                    'shrink-0',
                    isLiveStreaming ? 'text-amber-400 animate-pulse' : 'text-text-muted'
                  )}
                />
              )}
              <h3 className="text-sm font-medium text-text-default truncate">{title}</h3>
              {showAgentBadge && (
                <Badge variant="default" size="sm" className="ml-1">
                  {panelDetail.data.agentName}
                  {panelDetail.data.modeName && panelDetail.data.modeName !== 'default'
                    ? ` / ${panelDetail.data.modeName}`
                    : ''}
                </Badge>
              )}
            </div>
            <button
              type="button"
              onClick={closeDetail}
              className="p-1 rounded-md hover:bg-background-muted transition-colors shrink-0 cursor-pointer"
            >
              <X size={16} className="text-text-muted" />
            </button>
          </div>

          {/* Content */}
          <ScrollArea className="flex-1 min-h-0 px-4 py-4">
            {isWorkBlock && panelDetail.data.messages ? (
              <div className="space-y-3">
                {/* Status bar */}
                {panelDetail.data.toolCount > 0 && (
                  <div className="flex items-center gap-2 text-xs text-text-muted px-1 py-1.5 border-b border-border-default">
                    <StatusDot status={isWorkBlockStreaming ? 'active' : 'completed'} />
                    <span>
                      {panelDetail.data.toolCount} tool
                      {panelDetail.data.toolCount !== 1 ? 's' : ''} used
                    </span>
                  </div>
                )}
                {panelDetail.data.messages.map((msg, i) => {
                  const isLastMsg = i === panelDetail.data.messages.length - 1;
                  return (
                    <div key={msg.id ?? `wb-msg-${i}`} className="text-sm">
                      <GooseMessage
                        sessionId={panelDetail.data.sessionId || ''}
                        message={msg}
                        messages={panelDetail.data.messages}
                        append={NOOP}
                        toolCallNotifications={EMPTY_TOOL_NOTIFICATIONS}
                        isStreaming={isWorkBlockStreaming && isLastMsg}
                        suppressToolCalls
                      />
                    </div>
                  );
                })}
                <div ref={bottomRef} />
              </div>
            ) : isReasoning && detail ? (
              <div className="text-sm text-text-muted/90">
                <MarkdownContent content={detail.content} />
                <div ref={bottomRef} />
              </div>
            ) : null}
          </ScrollArea>
        </>
      )}
    </div>
  );
}
