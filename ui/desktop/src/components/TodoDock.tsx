import type { PlanEntry } from '@agentclientprotocol/sdk';
import { ChevronDown } from 'lucide-react';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { defineMessages, useIntl } from '../i18n';
import { cn } from '../utils';
import { completedTodoCount, currentTodo } from '../utils/todoPlan';
import TodoChecklist from './TodoChecklist';

const i18n = defineMessages({
  progress: { id: 'todoDock.progress', defaultMessage: '{done} of {total} completed' },
  current: { id: 'todoDock.current', defaultMessage: 'Current: {task}' },
  expand: { id: 'todoDock.expand', defaultMessage: 'Expand task list' },
  collapse: { id: 'todoDock.collapse', defaultMessage: 'Collapse task list' },
});

export default function TodoDock({ entries, active }: { entries: PlanEntry[]; active: boolean }) {
  const intl = useIntl();
  const [collapsed, setCollapsed] = useState(true);
  const [visible, setVisible] = useState(false);
  const [renderedEntries, setRenderedEntries] = useState(entries);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const signature = useMemo(() => JSON.stringify(entries), [entries]);
  const previousSignature = useRef('');

  useEffect(() => {
    const entriesChanged = signature !== previousSignature.current;
    if (entriesChanged) {
      previousSignature.current = signature;
      if (closeTimer.current) clearTimeout(closeTimer.current);
      setRenderedEntries(entries);
    }
    if (entries.length === 0) {
      setVisible(false);
      return;
    }

    const complete = entries.every((entry) => entry.status === 'completed');
    if (!active) {
      setVisible(false);
      return;
    }
    setVisible(true);
    if (complete && entriesChanged) {
      closeTimer.current = setTimeout(() => setVisible(false), 650);
    }
  }, [active, entries, signature]);

  useEffect(
    () => () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
    },
    []
  );

  if (renderedEntries.length === 0) return null;

  const done = completedTodoCount(renderedEntries);
  const current = currentTodo(renderedEntries);
  const toggleLabel = collapsed
    ? intl.formatMessage(i18n.expand)
    : intl.formatMessage(i18n.collapse);

  return (
    <div
      aria-hidden={!visible}
      className={cn(
        'grid transition-[grid-template-rows,opacity,transform,margin] duration-200 ease-out motion-reduce:transition-none',
        visible
          ? 'grid-rows-[1fr] opacity-100 translate-y-0 mb-2'
          : 'pointer-events-none grid-rows-[0fr] opacity-0 translate-y-1 mb-0'
      )}
    >
      <div className="min-h-0 overflow-hidden">
        <div className="rounded-xl border border-border-primary bg-background-primary shadow-sm">
          <button
            type="button"
            className="flex min-h-10 w-full items-center gap-2 px-3 py-2 text-left hover:bg-background-secondary/50"
            onClick={() => setCollapsed((value) => !value)}
            aria-expanded={!collapsed}
            aria-label={toggleLabel}
          >
            <span className="shrink-0 text-sm font-medium text-text-primary" aria-live="polite">
              {intl.formatMessage(i18n.progress, { done, total: renderedEntries.length })}
            </span>
            {collapsed && current && (
              <span className="min-w-0 flex-1 truncate text-sm text-text-secondary">
                {intl.formatMessage(i18n.current, { task: current.content })}
              </span>
            )}
            <ChevronDown
              className={cn(
                'ml-auto size-4 shrink-0 text-text-secondary transition-transform duration-200 motion-reduce:transition-none',
                !collapsed && 'rotate-180'
              )}
            />
          </button>
          {!collapsed && (
            <TodoChecklist
              entries={renderedEntries}
              className="max-h-44 overflow-y-auto border-t border-border-primary px-3 py-2.5"
            />
          )}
        </div>
      </div>
    </div>
  );
}
