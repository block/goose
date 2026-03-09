import { useCallback, useEffect, useRef, useState } from 'react';
import { canTrack, trackFeedbackSubmitted, trackFeedbackDismissed } from '../utils/analytics';
import { ChatState } from '../types/chatState';

const EXCHANGE_INTERVAL = 5; // Show after every Nth exchange
const IN_SESSION_COOLDOWN_MS = 2 * 60 * 60 * 1000; // 2 hours
const CROSS_SESSION_COOLDOWN_MS = 24 * 60 * 60 * 1000; // 24 hours

interface UseFeedbackPromptOptions {
  messageCount: number;
  chatState: ChatState;
  provider?: string | null;
  model?: string | null;
}

export function useFeedbackPrompt({
  messageCount,
  chatState,
  provider,
  model,
}: UseFeedbackPromptOptions) {
  const [showFeedback, setShowFeedback] = useState(false);
  const exchangeCountRef = useRef(0);
  const lastPromptTimeRef = useRef(0);
  const prevChatStateRef = useRef<ChatState>(chatState);

  useEffect(() => {
    const wasStreaming = prevChatStateRef.current !== ChatState.Idle;
    const isNowIdle = chatState === ChatState.Idle;
    prevChatStateRef.current = chatState;

    // Only trigger when transitioning from streaming to idle
    if (!wasStreaming || !isNowIdle) return;

    // Check telemetry at event time (it can change during the session)
    if (!canTrack()) return;

    exchangeCountRef.current += 1;
    const count = exchangeCountRef.current;

    // Only show on every Nth exchange, never before the first interval
    if (count < EXCHANGE_INTERVAL || count % EXCHANGE_INTERVAL !== 0) return;

    // In-session cooldown
    const now = Date.now();
    if (now - lastPromptTimeRef.current < IN_SESSION_COOLDOWN_MS) return;

    // Cross-session cooldown (async check)
    (async () => {
      try {
        const lastTimestamp =
          (await window.electron.getSetting('lastFeedbackTimestamp')) as number;
        if (lastTimestamp && now - lastTimestamp < CROSS_SESSION_COOLDOWN_MS) return;
      } catch {
        // If settings read fails, proceed anyway
      }

      lastPromptTimeRef.current = now;
      setShowFeedback(true);
    })();
  }, [chatState]);

  const onRate = useCallback(
    (rating: 1 | 2 | 3 | 4) => {
      setShowFeedback(false);
      trackFeedbackSubmitted(rating, messageCount, provider || undefined, model || undefined);
      try {
        window.electron.setSetting('lastFeedbackTimestamp', Date.now());
      } catch {
        // Best-effort persistence
      }
    },
    [messageCount, provider, model]
  );

  const onDismiss = useCallback(() => {
    setShowFeedback(false);
    trackFeedbackDismissed(messageCount);
    try {
      window.electron.setSetting('lastFeedbackTimestamp', Date.now());
    } catch {
      // Best-effort persistence
    }
  }, [messageCount]);

  return { showFeedback, onRate, onDismiss };
}
