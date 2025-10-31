import React from 'react';
import { Message, SystemNotificationContent } from '../../api';

interface SystemNotificationInlineProps {
  message: Message;
}

export const SystemNotificationInline: React.FC<SystemNotificationInlineProps> = ({ message }) => {
  const systemNotification = message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification' && content.notificationType === 'inlineMessage'
  );

  if (!systemNotification?.msg) {
    return null;
  }

  return <div className="text-xs text-gray-400 py-2 text-left">{systemNotification.msg}</div>;
};
