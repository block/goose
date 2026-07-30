import { acpChatSessionStore, subscribeToAcpChatSession } from './acp/chatSessionStore';
import { AppEvents } from './constants/events';
import { ChatState } from './types/chatState';

export type SessionStreamState = 'idle' | 'loading' | 'streaming' | 'waiting' | 'error';

const BUSY_STREAM_STATES: ReadonlySet<SessionStreamState> = new Set<SessionStreamState>([
  'loading',
  'streaming',
  'waiting',
]);

export function sessionStreamStateFor(
  chatState: ChatState | undefined,
  sessionLoadError: string | undefined
): SessionStreamState {
  if (chatState === ChatState.LoadingConversation) return 'loading';
  if (
    chatState === ChatState.Streaming ||
    chatState === ChatState.Thinking ||
    chatState === ChatState.Compacting
  ) {
    return 'streaming';
  }
  // A prompt paused on a permission/elicitation request still has an active
  // server run, so it must not report as idle.
  if (chatState === ChatState.WaitingForUserInput) return 'waiting';
  if (sessionLoadError) return 'error';
  return 'idle';
}

// Other desktop windows run their own backends and cannot see this window's
// runs; relay status through the main process so their archive/delete guards
// can. A chat component can unmount mid-run (session eviction), so reporting
// falls back to the ACP store, which outlives the component — otherwise the
// session would stay cached as busy until this window reloads.
export function reportSessionStatus(sessionId: string, streamState: SessionStreamState): void {
  window.electron?.broadcastSessionStatus?.({ sessionId, streamState });
}

// This window's own sidebar tracks status from SESSION_STATUS_UPDATE, which
// only BaseChat emits, so an unmounted chat leaves it stale here too.
function reportSessionStatusLocally(sessionId: string, streamState: SessionStreamState): void {
  window.dispatchEvent(
    new CustomEvent(AppEvents.SESSION_STATUS_UPDATE, { detail: { sessionId, streamState } })
  );
  reportSessionStatus(sessionId, streamState);
}

const watchers = new Map<string, () => void>();

function stopWatching(sessionId: string): void {
  watchers.get(sessionId)?.();
  watchers.delete(sessionId);
}

export function beginSessionStatusReporting(sessionId: string): void {
  stopWatching(sessionId);
}

export function endSessionStatusReporting(sessionId: string): void {
  stopWatching(sessionId);

  const snapshot = acpChatSessionStore.getSnapshot(sessionId);
  const streamState = sessionStreamStateFor(snapshot?.chatState, snapshot?.sessionLoadError);
  if (!BUSY_STREAM_STATES.has(streamState)) {
    reportSessionStatusLocally(sessionId, 'idle');
    return;
  }

  const unsubscribe = subscribeToAcpChatSession(sessionId, (next) => {
    const nextStreamState = sessionStreamStateFor(next.chatState, next.sessionLoadError);
    reportSessionStatusLocally(sessionId, nextStreamState);
    if (!BUSY_STREAM_STATES.has(nextStreamState)) {
      stopWatching(sessionId);
    }
  });
  watchers.set(sessionId, unsubscribe);
}
