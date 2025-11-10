import React from 'react';
import { Message, SummarizationRequestedContent } from '../../types/message';

interface CompactionMarkerProps {
  message: Message;
}

export const CompactionMarker: React.FC<CompactionMarkerProps> = ({ message }) => {
  const compactionContent = message.content.find(
    (content) => content.type === 'summarizationRequested'
  ) as SummarizationRequestedContent | undefined;

  const markerText = compactionContent?.msg || 'Conversation compacted';

  return <div className="text-xs text-gray-400 py-2 text-left">{markerText}</div>;
};
