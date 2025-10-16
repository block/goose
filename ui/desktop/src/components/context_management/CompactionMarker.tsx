import React from 'react';
import { Message, SystemNotificationContent } from '../../api';

interface CompactionMarkerProps {
  message: Message;
}

export const CompactionMarker: React.FC<CompactionMarkerProps> = ({ message }) => {
  const systemNotification = message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification'
  );

  const markerText = systemNotification?.msg || 'Conversation compacted';

  return <div className="text-xs text-gray-400 py-2 text-left">{markerText}</div>;
};
