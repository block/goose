import React from 'react';
import { Message, SystemNotificationContent } from '../../api';

interface CreditsExhaustedNotificationProps {
  notification: SystemNotificationContent;
}

export const CreditsExhaustedNotification: React.FC<CreditsExhaustedNotificationProps> = ({
  notification,
}) => {
  const topUpUrl =
    notification.data &&
    typeof notification.data === 'object' &&
    'top_up_url' in (notification.data as Record<string, unknown>)
      ? ((notification.data as Record<string, unknown>).top_up_url as string | null)
      : null;

  const handleTopUp = () => {
    if (topUpUrl) {
      window.electron.openExternal(topUpUrl);
    }
  };

  return (
    <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-4 my-2">
      <div className="flex items-start gap-3">
        <div className="text-yellow-500 text-lg mt-0.5">⚠️</div>
        <div className="flex-1">
          <div className="text-sm text-yellow-200 whitespace-pre-line">{notification.msg}</div>
          {topUpUrl && (
            <button
              onClick={handleTopUp}
              className="mt-3 inline-flex items-center gap-2 rounded-md bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-medium px-4 py-2 transition-colors"
            >
              Top Up Credits
              <span className="text-xs">↗</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export function getCreditsExhaustedNotification(
  message: Message
): SystemNotificationContent | undefined {
  return message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification' && content.notificationType === 'creditsExhausted'
  );
}
