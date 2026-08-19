export const DEFAULT_VISIBLE_MESSAGE_WINDOW = 80;
export const EARLIER_MESSAGE_PAGE_SIZE = 80;

export function initialTranscriptWindowStart(
  messageCount: number,
  visibleWindow: number = DEFAULT_VISIBLE_MESSAGE_WINDOW
): number {
  if (messageCount <= visibleWindow) {
    return 0;
  }
  return messageCount - visibleWindow;
}

export function earlierTranscriptWindowStart(
  windowStart: number,
  pageSize: number = EARLIER_MESSAGE_PAGE_SIZE
): number {
  return Math.max(0, windowStart - pageSize);
}

export function transcriptMessageKey(message: { id?: string | null; role: string; created: number }): string {
  if (message.id) {
    return message.id;
  }
  return `${message.role}:${message.created}`;
}

export function visibleTranscriptWindowStart(input: {
  messageCount: number;
  showAll: boolean;
  pinnedToLiveEdge: boolean;
  windowStart: number;
  visibleWindow?: number;
}): number {
  if (input.showAll) {
    return 0;
  }

  const latestStart = initialTranscriptWindowStart(
    input.messageCount,
    input.visibleWindow ?? DEFAULT_VISIBLE_MESSAGE_WINDOW
  );
  if (input.pinnedToLiveEdge) {
    return latestStart;
  }

  return Math.max(0, Math.min(input.windowStart, latestStart));
}

export function laterTranscriptWindowStart(
  windowStart: number,
  messageCount: number,
  pageSize: number = EARLIER_MESSAGE_PAGE_SIZE,
  visibleWindow: number = DEFAULT_VISIBLE_MESSAGE_WINDOW
): number {
  const latestStart = initialTranscriptWindowStart(messageCount, visibleWindow);
  return Math.min(latestStart, windowStart + pageSize);
}

export function transcriptWindowForIndex(
  messageIndex: number,
  messageCount: number,
  visibleWindow: number = DEFAULT_VISIBLE_MESSAGE_WINDOW
): number {
  if (messageCount <= visibleWindow) {
    return 0;
  }
  const centered = messageIndex - Math.floor(visibleWindow / 2);
  return Math.max(0, Math.min(centered, messageCount - visibleWindow));
}

export function visibleTranscriptRange(input: {
  messageCount: number;
  showAll: boolean;
  pinnedToLiveEdge: boolean;
  windowStart: number;
  visibleWindow?: number;
}): { start: number; end: number } {
  const visibleWindow = input.visibleWindow ?? DEFAULT_VISIBLE_MESSAGE_WINDOW;
  if (input.showAll) {
    return { start: 0, end: input.messageCount };
  }
  const start = visibleTranscriptWindowStart({ ...input, visibleWindow });
  return {
    start,
    end: Math.min(input.messageCount, start + visibleWindow),
  };
}
