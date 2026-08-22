import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SessionDraftStorage } from '../utils/sessionDraftStorage';

export const DRAFT_SAVE_DEBOUNCE_MS = 500;

/**
 * Keeps the unsent input of one chat.
 *
 * Writes are debounced, and the pending write is owned here together with the
 * two things that have to cancel it, so they cannot drift apart:
 *
 * - `clear` drops the pending write before removing the draft. Without that, a
 *   message sent within the debounce window is written back as a draft and
 *   reappears in the input, which is how the removed implementation behaved.
 * - unmount and a key change flush the pending write instead of dropping it,
 *   because leaving the page right after typing is the case this exists for.
 */
export function useChatDraft(key: string) {
  const pendingRef = useRef<{ key: string; text: string } | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cancelTimer = () => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const flush = useCallback(() => {
    cancelTimer();
    const pending = pendingRef.current;
    if (!pending) return;

    pendingRef.current = null;
    SessionDraftStorage.set(pending.key, pending.text);
  }, []);

  // A write can be up to the debounce behind storage, so the pending text is the
  // truth while it is waiting. Callers decide what to render from this.
  const pendingFor = (draftKey: string) => {
    const pending = pendingRef.current;
    return pending && pending.key === draftKey ? pending : null;
  };

  const has = useCallback(() => pendingFor(key) !== null || SessionDraftStorage.has(key), [key]);

  const read = useCallback(() => pendingFor(key)?.text ?? SessionDraftStorage.get(key), [key]);

  const save = useCallback(
    (text: string) => {
      pendingRef.current = { key, text };
      cancelTimer();
      timerRef.current = setTimeout(flush, DRAFT_SAVE_DEBOUNCE_MS);
    },
    [key, flush]
  );

  const clear = useCallback(() => {
    cancelTimer();
    pendingRef.current = null;
    SessionDraftStorage.clear(key);
  }, [key]);

  useEffect(() => flush, [key, flush]);

  // Stable per key, so callers can depend on it without re-running on every render.
  return useMemo(() => ({ has, read, save, clear }), [has, read, save, clear]);
}
