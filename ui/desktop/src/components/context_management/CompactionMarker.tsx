import React from 'react';
import { Message, SummarizationRequested } from '../../api';

interface CompactionMarkerProps {
  message: Message;
}

export const CompactionMarker: React.FC<CompactionMarkerProps> = ({ message }) => {
  const compactionContent = message.content.find(
    (content): content is SummarizationRequested & { type: 'summarizationRequested' } =>
      content.type === 'summarizationRequested'
  );

  const markerText = compactionContent?.msg || 'Conversation compacted';

  return <div className="text-xs text-gray-400 py-2 text-left">{markerText}</div>;
};
