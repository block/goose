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
