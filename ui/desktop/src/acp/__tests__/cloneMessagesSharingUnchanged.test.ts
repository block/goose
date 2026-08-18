import { describe, expect, it } from 'vitest';
import type { Message } from '../../types/message';
import { cloneMessagesSharingUnchanged } from '../adapter/shared';

function message(id: string, text: string): Message {
  return {
    id,
    role: 'assistant',
    created: 1,
    content: [{ type: 'text', text }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

describe('cloneMessagesSharingUnchanged', () => {
  it('keeps the published object when the live message was not rewritten', () => {
    const first = message('one', 'hello');
    const second = message('two', 'wor');
    const publishedFirst = { ...first, content: [{ ...first.content[0] }] };
    const nextSecond = message('two', 'world');
    const publishedByLive = new Map<Message, Message>([[first, publishedFirst]]);

    const previousPublished = [publishedFirst, second];
    const cloned = cloneMessagesSharingUnchanged(
      [first, nextSecond],
      publishedByLive,
      previousPublished
    );

    expect(cloned[0]).toBe(publishedFirst);
    expect(cloned[1]).not.toBe(nextSecond);
    expect(cloned[1].content).toEqual([{ type: 'text', text: 'world' }]);
  });

  it('reuses the previous published array when nothing changed', () => {
    const first = message('one', 'hello');
    const publishedFirst = { ...first, content: [{ ...first.content[0] }] };
    const previousPublished = [publishedFirst];
    const publishedByLive = new Map<Message, Message>([[first, publishedFirst]]);

    const cloned = cloneMessagesSharingUnchanged([first], publishedByLive, previousPublished);

    expect(cloned).toBe(previousPublished);
  });

  it('clones every message when there is no previous snapshot', () => {
    const original = message('one', 'hello');
    const cloned = cloneMessagesSharingUnchanged([original], undefined, undefined);

    expect(cloned).toHaveLength(1);
    expect(cloned[0]).not.toBe(original);
    expect(cloned[0]).toEqual(original);
  });
});
