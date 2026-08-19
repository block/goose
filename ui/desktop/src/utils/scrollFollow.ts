export const BOTTOM_SCROLL_THRESHOLD = 200;

export function distanceFromBottom(viewport: {
  scrollHeight: number;
  scrollTop: number;
  clientHeight: number;
}): number {
  return viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
}

export function isViewportAtBottom(
  viewport: {
    scrollHeight: number;
    scrollTop: number;
    clientHeight: number;
  },
  threshold: number = BOTTOM_SCROLL_THRESHOLD
): boolean {
  return distanceFromBottom(viewport) <= threshold;
}

export function shouldUnfollowOnScroll(input: {
  isFollowing: boolean;
  isProgrammatic: boolean;
  isAtBottom: boolean;
}): boolean {
  return input.isFollowing && !input.isProgrammatic && !input.isAtBottom;
}
