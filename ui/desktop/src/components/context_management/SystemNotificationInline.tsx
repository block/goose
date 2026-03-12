import React from 'react';
import { Message, SystemNotificationContent } from '../../api';

interface SystemNotificationInlineProps {
  notification: SystemNotificationContent;
}

export const SystemNotificationInline: React.FC<SystemNotificationInlineProps> = ({
  notification,
}) => {
  return <div className="text-xs text-gray-400 py-2 text-left">{notification.msg}</div>;
};

export function getInlineSystemNotification(
  message: Message
): SystemNotificationContent | undefined {
  return message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification' && content.notificationType === 'inlineMessage'
  );
}
