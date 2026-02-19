import { useEffect, useRef } from 'react';
import { X, Brain, Wrench } from 'lucide-react';
import { useReasoningDetail } from '../contexts/ReasoningDetailContext';
import MarkdownContent from './MarkdownContent';
import { ScrollArea } from './ui/atoms/scroll-area';
import { cn } from '../utils';
import GooseMessage from './GooseMessage';

export default function ReasoningDetailPanel() {
  const { detail, panelDetail, isOpen, closeDetail } = useReasoningDetail();
  const bottomRef = useRef<HTMLDivElement>(null);

  const isWorkBlock = panelDetail?.type === 'workblock';
  const isReasoning = panelDetail?.type === 'reasoning' || (!panelDetail && detail);

  // Work block is "live" when it is actively streaming
  const isWorkBlockStreaming = isWorkBlock && (panelDetail.data.isStreaming ?? false);
  const isReasoningStreaming = isReasoning && detail?.title === 'Thinking...';
  const isLiveStreaming = isWorkBlockStreaming || isReasoningStreaming;

  const title = isWorkBlock
    ? panelDetail.data.title || 'Work Block'
    : detail?.title || 'Details';

  // Auto-scroll to bottom during live streaming
  useEffect(() => {
    if (isLiveStreaming && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [detail?.content, panelDetail, isLiveStreaming]);

  return (
    <div
      className={cn(
        'h-full border-l border-border-default bg-background-default flex flex-col transition-[width,opacity] duration-300 ease-in-out overflow-hidden',
        isOpen ? 'w-[400px] min-w-[400px] opacity-100' : 'w-0 min-w-0 opacity-0'
      )}
    >
      {isOpen && (
        <>
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
              <h3 className="text-sm font-medium text-text-default truncate">
                {title}
              </h3>
              {isWorkBlock && panelDetail.data.agentName && (
                <span className="text-xs text-blue-500 font-medium ml-1">
                  {panelDetail.data.agentName}
                  {panelDetail.data.modeName && ` · ${panelDetail.data.modeName}`}
                </span>
              )}
            </div>
            <button
              onClick={closeDetail}
              className="p-1 rounded-md hover:bg-background-muted transition-colors shrink-0 cursor-pointer"
            >
              <X size={16} className="text-text-muted" />
            </button>
          </div>

          <ScrollArea className="flex-1 min-h-0 px-4 py-4">
            {isWorkBlock && panelDetail.data.messages ? (
              <div className="space-y-3">
                {panelDetail.data.toolCount > 0 && (
                  <div className="text-xs text-text-muted px-1 py-1.5 border-b border-border-default">
                    {panelDetail.data.toolCount} tool{panelDetail.data.toolCount !== 1 ? 's' : ''} used
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
                        append={() => {}}
                        toolCallNotifications={new Map()}
                        isStreaming={isWorkBlockStreaming && isLastMsg}
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
