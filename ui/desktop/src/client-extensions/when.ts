import type { ExtensionHostContext } from './types';

export function evaluateWhenClause(clause: string | undefined, context: ExtensionHostContext): boolean {
  if (!clause || !clause.trim()) {
    return true;
  }

  const trimmed = clause.trim();

  if (trimmed === 'session.active') {
    return context.sessionId !== null && context.sessionId.length > 0;
  }

  if (trimmed === '!session.active') {
    return context.sessionId === null || context.sessionId.length === 0;
  }

  console.warn(`[client-extensions] Unknown when clause "${clause}" — defaulting to visible`);
  return true;
}
