import React, { useState } from 'react';
import { Message, SummarizationRequestedContent } from '../../types/message';
import { Search } from 'lucide-react';
import { Button } from '../ui/button';
import SummaryViewModal from '../SummaryViewModal';
import { cn } from '../../utils';

interface CompactionMarkerProps {
  message: Message;
  messages?: Message[];
}

export const CompactionMarker: React.FC<CompactionMarkerProps> = ({ message, messages }) => {
  const [showSummaryModal, setShowSummaryModal] = useState(false);

  const compactionContent = message.content.find(
    (content) => content.type === 'summarizationRequested'
  ) as SummarizationRequestedContent | undefined;

  const markerText = compactionContent?.msg || 'Conversation compacted';
  const summaryText = compactionContent?.summary || null;

  // Check if this is a compaction message that mentions summarization
  // Show button if:
  // 1. There's an embedded summary in this message, OR
  // 2. The message text indicates a summary was created (even if stored elsewhere)
  const showSummaryButton = summaryText || markerText.toLowerCase().includes('summarized');

  // Check if this message was part of a summarization (user visible but not agent visible)
  const isSummarizedMessage =
    message.metadata?.userVisible === true && message.metadata?.agentVisible === false;

  return (
    <div
      className={cn(
        'flex items-center justify-between py-2',
        isSummarizedMessage && 'opacity-50 italic'
      )}
    >
      <div className="text-xs text-gray-400 text-left">{markerText}</div>
      {showSummaryButton && (
        <Button
          onClick={() => setShowSummaryModal(true)}
          size="sm"
          variant="ghost"
          className="ml-2 text-xs"
        >
          <Search className="w-3 h-3 mr-1" />
          View Summary
        </Button>
      )}
      {showSummaryModal && (
        <SummaryViewModal
          isOpen={showSummaryModal}
          onClose={() => setShowSummaryModal(false)}
          messages={messages}
          summaryText={summaryText || undefined}
        />
      )}
    </div>
  );
};
