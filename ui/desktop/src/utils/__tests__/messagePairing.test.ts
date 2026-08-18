import { describe, expect, it } from 'vitest';
import type { Message } from '../../types/message';
import { laterPairingChanged, onlyLastMessageChanged } from '../messagePairing';

function textMessage(id: string, text: string): Message {
  return {
    id,
    role: 'assistant',
    created: 1,
    content: [{ type: 'text', text }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

function toolResponseMessage(id: string): Message {
  return {
    id,
    role: 'user',
    created: 1,
    content: [
      {
        type: 'toolResponse',
        id: `${id}-tool`,
        toolResult: { status: 'success' },
      },
    ],
    metadata: { userVisible: true, agentVisible: true },
  };
}

describe('messagePairing', () => {
  it('detects a last-message-only identity change', () => {
    const first = textMessage('one', 'hello');
    const previousLast = textMessage('two', 'wor');
    const nextLast = textMessage('two', 'world');

    expect(onlyLastMessageChanged([first, previousLast], [first, nextLast])).toBe(true);
    expect(onlyLastMessageChanged([first, previousLast], [textMessage('one', 'hello'), nextLast])).toBe(
      false
    );
  });

  it('ignores streamed text updates for earlier messages', () => {
    const first = textMessage('one', 'hello');
    const previousLast = textMessage('two', 'wor');
    const nextLast = textMessage('two', 'world');

    expect(laterPairingChanged([first, previousLast], [first, nextLast], first)).toBe(false);
  });

  it('treats a later tool response as a pairing change', () => {
    const first = textMessage('one', 'hello');
    const previousLast = textMessage('two', 'world');
    const nextLast = toolResponseMessage('two');

    expect(laterPairingChanged([first, previousLast], [first, nextLast], first)).toBe(true);
  });
});
