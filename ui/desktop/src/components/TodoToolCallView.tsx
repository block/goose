import { ChevronDown } from 'lucide-react';
import { useState } from 'react';
import { defineMessages, useIntl } from '../i18n';
import { cn } from '../utils';
import { completedTodoCount, parseTodoMarkdown } from '../utils/todoPlan';
import type { LoadingStatus } from './ToolCallWithResponse';
import TodoChecklist from './TodoChecklist';

const i18n = defineMessages({
  updating: { id: 'todoTool.updating', defaultMessage: 'Updating task list' },
  updated: { id: 'todoTool.updated', defaultMessage: 'Task list updated' },
  updatedWithProgress: {
    id: 'todoTool.updatedWithProgress',
    defaultMessage: 'Task list updated · {done}/{total}',
  },
  failed: { id: 'todoTool.failed', defaultMessage: 'Failed to update task list' },
  expand: { id: 'todoTool.expand', defaultMessage: 'Show updated task list' },
  collapse: { id: 'todoTool.collapse', defaultMessage: 'Hide updated task list' },
});

export default function TodoToolCallView({
  content,
  status,
}: {
  content: string;
  status: LoadingStatus;
}) {
  const intl = useIntl();
  const [expanded, setExpanded] = useState(false);
  const entries = parseTodoMarkdown(content);
  const done = completedTodoCount(entries);
  const label =
    status === 'loading'
      ? intl.formatMessage(i18n.updating)
      : status === 'error'
        ? intl.formatMessage(i18n.failed)
        : entries.length > 0
          ? intl.formatMessage(i18n.updatedWithProgress, { done, total: entries.length })
          : intl.formatMessage(i18n.updated);

  return (
    <div className="w-full text-sm font-sans">
      <button
        type="button"
        className="group flex min-h-8 w-full items-center gap-2 rounded-md py-1 text-left hover:bg-background-secondary/60"
        onClick={() => entries.length > 0 && setExpanded((value) => !value)}
        aria-expanded={entries.length > 0 ? expanded : undefined}
        aria-label={
          entries.length > 0
            ? expanded
              ? intl.formatMessage(i18n.collapse)
              : intl.formatMessage(i18n.expand)
            : undefined
        }
      >
        <span
          className={cn(
            'min-w-0 flex-1 truncate',
            status === 'loading' && 'tool-call-name-loading',
            status === 'error' && 'text-text-danger'
          )}
        >
          {label}
        </span>
        {entries.length > 0 && (
          <ChevronDown
            className={cn(
              'size-4 shrink-0 text-text-secondary transition-transform',
              expanded && 'rotate-180'
            )}
          />
        )}
      </button>
      {expanded && <TodoChecklist entries={entries} className="px-2 pb-2 pt-1" />}
    </div>
  );
}
