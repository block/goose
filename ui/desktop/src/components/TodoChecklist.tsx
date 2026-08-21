import type { PlanEntry } from '@agentclientprotocol/sdk';
import { Check, Circle, LoaderCircle } from 'lucide-react';
import { defineMessages, useIntl } from '../i18n';
import { cn } from '../utils';
import { todoDepth } from '../utils/todoPlan';

const i18n = defineMessages({
  completed: { id: 'todoChecklist.status.completed', defaultMessage: 'Completed' },
  inProgress: { id: 'todoChecklist.status.inProgress', defaultMessage: 'In progress' },
  pending: { id: 'todoChecklist.status.pending', defaultMessage: 'Pending' },
});

export default function TodoChecklist({
  entries,
  className,
}: {
  entries: PlanEntry[];
  className?: string;
}) {
  const intl = useIntl();

  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      {entries.map((entry, index) => {
        const completed = entry.status === 'completed';
        const inProgress = entry.status === 'in_progress';
        const statusLabel = completed
          ? intl.formatMessage(i18n.completed)
          : inProgress
            ? intl.formatMessage(i18n.inProgress)
            : intl.formatMessage(i18n.pending);
        const StatusIcon = completed ? Check : inProgress ? LoaderCircle : Circle;

        return (
          <div
            key={`${entry.content}-${index}`}
            className="flex min-w-0 items-start gap-2 text-sm"
            style={{ paddingLeft: `${Math.min(todoDepth(entry), 4) * 16}px` }}
          >
            <StatusIcon
              aria-label={statusLabel}
              className={cn(
                'mt-0.5 size-3.5 shrink-0',
                completed && 'text-text-secondary',
                inProgress && 'text-text-primary animate-pulse motion-reduce:animate-none',
                !completed && !inProgress && 'text-text-secondary/70'
              )}
            />
            <span
              className={cn(
                'min-w-0 break-words leading-5',
                completed ? 'text-text-secondary line-through' : 'text-text-primary'
              )}
            >
              {entry.content}
            </span>
          </div>
        );
      })}
    </div>
  );
}
