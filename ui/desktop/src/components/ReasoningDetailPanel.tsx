import { useEffect, useRef } from 'react';
import { X, Brain } from 'lucide-react';
import { useReasoningDetail } from '../contexts/ReasoningDetailContext';
import MarkdownContent from './MarkdownContent';
import { ScrollArea } from './ui/scroll-area';
import { cn } from '../utils';

export default function ReasoningDetailPanel() {
  const { detail, isOpen, closeDetail } = useReasoningDetail();
  const bottomRef = useRef<HTMLDivElement>(null);
  const isLiveStreaming = detail?.title === 'Thinking...';

  // Auto-scroll to bottom during live streaming
  useEffect(() => {
    if (isLiveStreaming && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [detail?.content, isLiveStreaming]);

  return (
    <div
      className={cn(
        'h-full border-l border-border-default bg-background-default flex flex-col transition-[width,opacity] duration-300 ease-in-out overflow-hidden',
        isOpen ? 'w-[400px] min-w-[400px] opacity-100' : 'w-0 min-w-0 opacity-0'
      )}
    >
      {detail && (
        <>
          <div className="flex items-center justify-between px-4 py-3 border-b border-border-default shrink-0">
            <div className="flex items-center gap-2 min-w-0">
              <Brain
                size={16}
                className={cn(
                  'shrink-0',
                  isLiveStreaming ? 'text-amber-400 animate-pulse' : 'text-text-muted'
                )}
              />
              <h3 className="text-sm font-medium text-text-default truncate">
                {detail.title}
              </h3>
            </div>
            <button
              onClick={closeDetail}
              className="p-1 rounded-md hover:bg-background-muted transition-colors shrink-0 cursor-pointer"
            >
              <X size={16} className="text-text-muted" />
            </button>
          </div>
          <ScrollArea className="flex-1 min-h-0" paddingX={4} paddingY={4}>
            <div className="text-sm text-text-muted/90">
              <MarkdownContent content={detail.content} />
              <div ref={bottomRef} />
            </div>
          </ScrollArea>
        </>
      )}
    </div>
  );
}
