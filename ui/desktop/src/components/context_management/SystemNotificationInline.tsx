import React from 'react';
import { useTranslation } from 'react-i18next';
import { Message, SystemNotificationContent } from '../../api';

interface SystemNotificationInlineProps {
  notification: SystemNotificationContent;
}

export const SystemNotificationInline: React.FC<SystemNotificationInlineProps> = ({
  notification,
}) => {
  const { t } = useTranslation();
  const normalizedMessage = notification.msg.trim().toLowerCase();

  const autoCompactMatch = notification.msg
    .trim()
    .match(/^Exceeded auto-compact threshold of (\d+)%\. Performing auto-compaction\.\.\.$/i);
  const compactionFailedMatch = notification.msg.trim().match(/^Compaction failed:?\s*(.*)$/i);

  const text = autoCompactMatch
    ? t('chat.systemNotification.autoCompactThreshold', { percent: autoCompactMatch[1] })
    : normalizedMessage === 'compaction complete'
      ? t('chat.systemNotification.compactionComplete')
      : normalizedMessage === 'conversation cleared'
        ? t('chat.systemNotification.conversationCleared')
        : normalizedMessage ===
            'context limit reached. compacting to continue conversation...'
          ? t('chat.systemNotification.contextLimitReached')
          : normalizedMessage.startsWith(
                'unable to continue: context limit still exceeded after compaction.'
              )
            ? t('chat.systemNotification.contextLimitStillExceeded')
            : compactionFailedMatch
              ? t('chat.systemNotification.compactionFailed', {
                  details: compactionFailedMatch[1] || '',
                })
              : notification.msg;

  const testId =
    normalizedMessage === 'compaction complete'
      ? 'compaction-success-marker'
      : compactionFailedMatch
        ? 'compaction-error-marker'
        : 'system-inline-notification';

  return (
    <div className="text-xs text-gray-400 py-2 text-left" data-testid={testId}>
      {text}
    </div>
  );
};

export function getInlineSystemNotification(
  message: Message
): SystemNotificationContent | undefined {
  return message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification' && content.notificationType === 'inlineMessage'
  );
}
