import type { PlanEntry } from '@agentclientprotocol/sdk';

export function isTodoWriteToolName(toolName: string): boolean {
  return toolName === 'todo_write' || toolName === 'todo__todo_write';
}

export function parseTodoMarkdown(content: string): PlanEntry[] {
  const entries = content
    .split(/\r?\n/)
    .map((line): PlanEntry | undefined => {
      const match = /^(\s*)[-*+]\s+\[([ xX>\/-])\]\s+(.+?)\s*$/.exec(line);
      if (!match) return undefined;
      const marker = match[2];
      const status =
        marker === 'x' || marker === 'X'
          ? 'completed'
          : marker === '>' || marker === '/' || marker === '-'
            ? 'in_progress'
            : 'pending';
      return {
        content: match[3],
        priority: 'medium',
        status,
        _meta: {
          goose: {
            todo: { depth: Math.floor(match[1].length / 2) },
          },
        },
      };
    })
    .filter((entry): entry is PlanEntry => entry !== undefined);

  if (!entries.some((entry) => entry.status === 'in_progress')) {
    const current = entries.find((entry) => entry.status === 'pending');
    if (current) current.status = 'in_progress';
  }

  return entries;
}

export function todoDepth(entry: PlanEntry): number {
  const goose = entry._meta?.goose;
  if (!goose || typeof goose !== 'object') return 0;
  const todo = (goose as Record<string, unknown>).todo;
  if (!todo || typeof todo !== 'object') return 0;
  const depth = (todo as Record<string, unknown>).depth;
  return typeof depth === 'number' && Number.isFinite(depth) ? Math.max(0, depth) : 0;
}

export function completedTodoCount(entries: PlanEntry[]): number {
  return entries.filter((entry) => entry.status === 'completed').length;
}

export function currentTodo(entries: PlanEntry[]): PlanEntry | undefined {
  return (
    entries.find((entry) => entry.status === 'in_progress') ??
    entries.find((entry) => entry.status === 'pending') ??
    [...entries].reverse().find((entry) => entry.status === 'completed')
  );
}
