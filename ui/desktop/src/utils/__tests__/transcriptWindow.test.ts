import { describe, expect, it } from 'vitest';
import {
  DEFAULT_VISIBLE_MESSAGE_WINDOW,
  EARLIER_MESSAGE_PAGE_SIZE,
  earlierTranscriptWindowStart,
  initialTranscriptWindowStart,
  visibleTranscriptWindowStart,
} from '../transcriptWindow';

describe('transcriptWindow', () => {
  it('keeps short transcripts fully visible', () => {
    expect(initialTranscriptWindowStart(12)).toBe(0);
    expect(initialTranscriptWindowStart(DEFAULT_VISIBLE_MESSAGE_WINDOW)).toBe(0);
  });

  it('pins long transcripts to the latest visible window', () => {
    expect(initialTranscriptWindowStart(241)).toBe(161);
    expect(initialTranscriptWindowStart(80, 20)).toBe(60);
  });

  it('pages earlier history without jumping to the start', () => {
    expect(earlierTranscriptWindowStart(161)).toBe(81);
    expect(earlierTranscriptWindowStart(81)).toBe(1);
    expect(earlierTranscriptWindowStart(1)).toBe(0);
    expect(earlierTranscriptWindowStart(40, EARLIER_MESSAGE_PAGE_SIZE)).toBe(0);
  });

  it('follows the live edge until the user expands history', () => {
    expect(
      visibleTranscriptWindowStart({
        messageCount: 82,
        showAll: false,
        pinnedToLiveEdge: true,
        windowStart: 1,
      })
    ).toBe(2);
  });

  it('keeps an expanded earlier page in place when a new message arrives', () => {
    expect(
      visibleTranscriptWindowStart({
        messageCount: 242,
        showAll: false,
        pinnedToLiveEdge: false,
        windowStart: 81,
      })
    ).toBe(81);
  });

  it('opens the full transcript when search expands it', () => {
    expect(
      visibleTranscriptWindowStart({
        messageCount: 241,
        showAll: true,
        pinnedToLiveEdge: false,
        windowStart: 161,
      })
    ).toBe(0);
  });
});
