import { describe, expect, it } from 'vitest';
import {
  BOTTOM_SCROLL_THRESHOLD,
  distanceFromBottom,
  isViewportAtBottom,
  shouldUnfollowOnScroll,
} from '../scrollFollow';

describe('scrollFollow', () => {
  it('measures the remaining distance to the end of the transcript', () => {
    expect(
      distanceFromBottom({
        scrollHeight: 2000,
        scrollTop: 1400,
        clientHeight: 400,
      })
    ).toBe(200);
  });

  it('treats the live edge as the bottom of the latest bubble', () => {
    expect(
      isViewportAtBottom({
        scrollHeight: 2000,
        scrollTop: 1600,
        clientHeight: 400,
      })
    ).toBe(true);
    expect(
      isViewportAtBottom({
        scrollHeight: 2000,
        scrollTop: 1399,
        clientHeight: 400,
      })
    ).toBe(false);
    expect(BOTTOM_SCROLL_THRESHOLD).toBe(200);
  });

  it('does not treat programmatic growth as a user scroll-up', () => {
    expect(
      shouldUnfollowOnScroll({
        isFollowing: true,
        isProgrammatic: true,
        isAtBottom: false,
      })
    ).toBe(false);
  });

  it('unfollows only when the user actually leaves the live edge', () => {
    expect(
      shouldUnfollowOnScroll({
        isFollowing: true,
        isProgrammatic: false,
        isAtBottom: false,
      })
    ).toBe(true);
    expect(
      shouldUnfollowOnScroll({
        isFollowing: true,
        isProgrammatic: false,
        isAtBottom: true,
      })
    ).toBe(false);
  });
});
