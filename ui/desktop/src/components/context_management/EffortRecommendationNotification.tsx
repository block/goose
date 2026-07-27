import React, { useState } from 'react';
import { Check, Lightbulb } from 'lucide-react';
import type { Message, SystemNotificationContent } from '../../types/message';
import type { ThinkingEffort } from '../../types/providers';
import { acpSetSessionThinkingEffort } from '../../acp/providers';
import { acpChatSessionStore, useAcpChatSessionSnapshot } from '../../acp/chatSessionStore';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  title: {
    id: 'effortRecommendationNotification.title',
    defaultMessage: 'Thinking effort suggestion',
  },
  switchEffort: {
    id: 'effortRecommendationNotification.switchEffort',
    defaultMessage: 'Switch thinking effort to {effort}',
  },
  switchEffortSave: {
    id: 'effortRecommendationNotification.switchEffortSave',
    defaultMessage: 'Switch thinking effort to {effort} to save tokens',
  },
  applied: {
    id: 'effortRecommendationNotification.applied',
    defaultMessage: 'Thinking effort set to {effort} for this session',
  },
  alreadySet: {
    id: 'effortRecommendationNotification.alreadySet',
    defaultMessage: 'Thinking effort is already set to {effort} for this session',
  },
  failed: {
    id: 'effortRecommendationNotification.failed',
    defaultMessage: 'Could not update the thinking effort. Please try again.',
  },
  effortLow: {
    id: 'effortRecommendationNotification.effortLow',
    defaultMessage: 'low',
  },
  effortMedium: {
    id: 'effortRecommendationNotification.effortMedium',
    defaultMessage: 'medium',
  },
  effortHigh: {
    id: 'effortRecommendationNotification.effortHigh',
    defaultMessage: 'high',
  },
  effortMax: {
    id: 'effortRecommendationNotification.effortMax',
    defaultMessage: 'max',
  },
});

const EFFORT_NAME_MESSAGES = {
  low: i18n.effortLow,
  medium: i18n.effortMedium,
  high: i18n.effortHigh,
  max: i18n.effortMax,
} as const;

const EFFORT_RANK: Record<string, number> = { off: 0, low: 1, medium: 2, high: 3, max: 4 };

type RecommendableEffort = 'low' | 'medium' | 'high';

interface EffortRecommendationNotificationProps {
  notification: SystemNotificationContent;
  sessionId: string;
}

function getRecommendedEffort(data: unknown): RecommendableEffort | null {
  if (!data || typeof data !== 'object') {
    return null;
  }
  const effort = (data as Record<string, unknown>).recommendedEffort;
  if (effort === 'low' || effort === 'medium' || effort === 'high') {
    return effort;
  }
  return null;
}

function knownEffort(value: unknown): keyof typeof EFFORT_RANK | null {
  return typeof value === 'string' && value in EFFORT_RANK ? value : null;
}

function getRecommendationTimeEffort(data: unknown): string | null {
  if (!data || typeof data !== 'object') {
    return null;
  }
  return knownEffort((data as Record<string, unknown>).currentEffort);
}

export const EffortRecommendationNotification: React.FC<EffortRecommendationNotificationProps> = ({
  notification,
  sessionId,
}) => {
  const intl = useIntl();
  const [status, setStatus] = useState<'idle' | 'pending' | 'applied' | 'failed'>('idle');
  const snapshot = useAcpChatSessionSnapshot(sessionId);
  const recommendedEffort = getRecommendedEffort(notification.data);

  if (!recommendedEffort) {
    return null;
  }

  const recommendationTimeEffort = getRecommendationTimeEffort(notification.data);
  // A recommendation below the effort the session ran at when it was made is
  // a downgrade offer; "satisfied" then means at-or-below the recommendation.
  const isLowering =
    recommendationTimeEffort !== null &&
    EFFORT_RANK[recommendedEffort] < EFFORT_RANK[recommendationTimeEffort];
  const satisfiesRecommendation = (effort: string | null): boolean =>
    effort !== null &&
    (isLowering
      ? EFFORT_RANK[effort] <= EFFORT_RANK[recommendedEffort]
      : EFFORT_RANK[effort] >= EFFORT_RANK[recommendedEffort]);

  const liveEffort = knownEffort(snapshot?.thinkingEffort);
  const currentEffort = liveEffort ?? recommendationTimeEffort;
  const alreadySatisfied = satisfiesRecommendation(currentEffort);
  const liveOutsideRecommendation = liveEffort !== null && !satisfiesRecommendation(liveEffort);
  const liveOptions = snapshot?.thinkingEffortOptions ?? null;
  const modelCannotApply = liveOptions !== null && !liveOptions.includes(recommendedEffort);

  const effortName = intl.formatMessage(EFFORT_NAME_MESSAGES[recommendedEffort]);

  const handleApply = async () => {
    const clickTimeSnapshot = acpChatSessionStore.getSnapshot(sessionId);
    const clickTimeEffort = knownEffort(clickTimeSnapshot?.thinkingEffort);
    const clickTimeOptions = clickTimeSnapshot?.thinkingEffortOptions ?? null;
    if (
      satisfiesRecommendation(clickTimeEffort) ||
      (clickTimeOptions !== null && !clickTimeOptions.includes(recommendedEffort))
    ) {
      return;
    }
    setStatus('pending');
    try {
      await acpSetSessionThinkingEffort(sessionId, recommendedEffort as ThinkingEffort);
      setStatus('applied');
    } catch {
      setStatus('failed');
    }
  };

  const renderStatusLine = () => {
    if (modelCannotApply) {
      return null;
    }
    if (status === 'applied' && !liveOutsideRecommendation) {
      return (
        <div className="mt-3 inline-flex items-center gap-2 text-sm font-medium text-blue-800 dark:text-blue-200">
          <Check className="h-3.5 w-3.5" />
          {intl.formatMessage(i18n.applied, { effort: effortName })}
        </div>
      );
    }
    if (alreadySatisfied) {
      const currentName =
        currentEffort in EFFORT_NAME_MESSAGES
          ? intl.formatMessage(
              EFFORT_NAME_MESSAGES[currentEffort as keyof typeof EFFORT_NAME_MESSAGES]
            )
          : currentEffort;
      return (
        <div className="mt-3 inline-flex items-center gap-2 text-sm font-medium text-blue-800/70 dark:text-blue-200/70">
          <Check className="h-3.5 w-3.5" />
          {intl.formatMessage(i18n.alreadySet, { effort: currentName })}
        </div>
      );
    }
    return (
      <button
        onClick={handleApply}
        disabled={status === 'pending'}
        className="mt-3 inline-flex items-center gap-2 rounded-md bg-blue-600 hover:bg-blue-500 dark:bg-blue-700 dark:hover:bg-blue-600 disabled:opacity-60 text-white text-sm font-medium px-4 py-2 transition-colors"
      >
        {intl.formatMessage(isLowering ? i18n.switchEffortSave : i18n.switchEffort, {
          effort: effortName,
        })}
      </button>
    );
  };

  return (
    <div className="rounded-lg border border-blue-600/30 dark:border-blue-500/30 bg-blue-500/10 dark:bg-blue-500/10 p-4 my-2">
      <div className="flex items-start gap-3">
        <Lightbulb className="h-4 w-4 text-blue-600 dark:text-blue-400 mt-0.5 shrink-0" />
        <div className="flex-1">
          <div className="text-sm font-semibold text-blue-800 dark:text-blue-200">
            {intl.formatMessage(i18n.title)}
          </div>
          <div className="text-sm text-blue-800/80 dark:text-blue-200/80 mt-1">
            {notification.msg}
          </div>
          {renderStatusLine()}
          {status === 'failed' && !alreadySatisfied && (
            <div className="text-sm text-red-600 dark:text-red-400 mt-2">
              {intl.formatMessage(i18n.failed)}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export function getEffortRecommendationNotification(
  message: Message
): SystemNotificationContent | undefined {
  return message.content.find(
    (content): content is SystemNotificationContent & { type: 'systemNotification' } =>
      content.type === 'systemNotification' && content.notificationType === 'effortRecommendation'
  );
}
