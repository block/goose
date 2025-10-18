import React from 'react';
import { Message, ConversationCompacted } from '../../api';

interface CompactionMarkerProps {
  message: Message;
}

export const CompactionMarker: React.FC<CompactionMarkerProps> = ({ message }) => {
  const compactionContent = message.content.find(
    (content): content is ConversationCompacted & { type: 'conversationCompacted' } =>
      content.type === 'conversationCompacted'
  );

  const markerText = compactionContent?.msg || 'Conversation compacted';

  return <div className="text-xs text-gray-400 py-2 text-left">{markerText}</div>;
};
