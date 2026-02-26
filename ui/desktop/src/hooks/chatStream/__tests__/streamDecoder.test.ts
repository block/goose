import { describe, expect, it } from 'vitest';
import { pushMessage } from '../streamDecoder';
import type { Message } from '@/api';

function makeTextMessage(
  text: string,
  id: string = 'msg-1',
  role: string = 'assistant'
): Message {
  return {
    role,
    id,
    created: Date.now(),
    content: [{ type: 'text' as const, text }],
    metadata: { agentVisible: true, userVisible: true },
  };
}

function makeToolRequestMessage(
  toolName: string,
  id: string = 'msg-1',
  toolId: string = 'tool-1'
): Message {
  return {
    role: 'assistant',
    id,
    created: Date.now(),
    content: [
      {
        type: 'toolRequest' as const,
        id: toolId,
        toolCall: { name: toolName, arguments: {} },
      },
    ],
    metadata: { agentVisible: true, userVisible: true },
  };
}

describe('pushMessage', () => {
  it('appends a new message when the list is empty', () => {
    const msg = makeTextMessage('hello');
    const result = pushMessage([], msg);
    expect(result).toHaveLength(1);
    expect(result[0]).toBe(msg);
  });

  it('appends a message with a different role', () => {
    const existing = makeTextMessage('hello', 'msg-1', 'assistant');
    const user = makeTextMessage('world', 'msg-2', 'user');
    const result = pushMessage([existing], user);
    expect(result).toHaveLength(2);
  });

  it('appends a message with a different id', () => {
    const existing = makeTextMessage('hello', 'msg-1');
    const next = makeTextMessage('world', 'msg-2');
    const result = pushMessage([existing], next);
    expect(result).toHaveLength(2);
  });

  it('accumulates text when same role and same id (streaming deltas)', () => {
    const chunk1 = makeTextMessage('Hello', 'msg-1');
    const chunk2 = makeTextMessage(' world', 'msg-1');
    const chunk3 = makeTextMessage('!', 'msg-1');

    let messages = pushMessage([], chunk1);
    messages = pushMessage(messages, chunk2);
    messages = pushMessage(messages, chunk3);

    expect(messages).toHaveLength(1);
    const content = messages[0].content[0];
    expect(content.type).toBe('text');
    expect((content as { text: string }).text).toBe('Hello world!');
  });

  it('accumulates many small deltas (simulating real streaming)', () => {
    const deltas = [
      'Now', ' I', ' have', ' a', ' comprehensive', ' understanding',
      ' of', ' the', ' component', ' structure', '.',
    ];

    let messages: Message[] = [];
    for (const delta of deltas) {
      messages = pushMessage(messages, makeTextMessage(delta, 'msg-stream'));
    }

    expect(messages).toHaveLength(1);
    const text = (messages[0].content[0] as { text: string }).text;
    expect(text).toBe('Now I have a comprehensive understanding of the component structure.');
  });

  it('does not merge text with non-text content types', () => {
    const textMsg = makeTextMessage('hello', 'msg-1');
    const toolMsg = makeToolRequestMessage('shell', 'msg-1');
    const result = pushMessage([textMsg], toolMsg);
    // Tool request should be appended, not merged
    expect(result).toHaveLength(2);
  });

  it('does not merge when last content is non-text', () => {
    const toolMsg = makeToolRequestMessage('shell', 'msg-1');
    const textMsg = makeTextMessage('done', 'msg-1');
    const result = pushMessage([toolMsg], textMsg);
    expect(result).toHaveLength(2);
  });

  it('preserves earlier content items when accumulating', () => {
    // Message with multiple content items, last one is text
    const msg1: Message = {
      role: 'assistant',
      id: 'msg-1',
      created: Date.now(),
      content: [
        { type: 'text' as const, text: 'thinking...' },
        { type: 'text' as const, text: 'Hello' },
      ],
      metadata: { agentVisible: true, userVisible: true },
    };
    const delta = makeTextMessage(' world', 'msg-1');
    const result = pushMessage([msg1], delta);

    expect(result).toHaveLength(1);
    expect(result[0].content).toHaveLength(2);
    // First content item preserved
    expect((result[0].content[0] as { text: string }).text).toBe('thinking...');
    // Last content item accumulated
    expect((result[0].content[1] as { text: string }).text).toBe('Hello world');
  });

  it('handles the "single dot" streaming bug scenario', () => {
    // This is the exact bug: 843 chunks where the last one is just "."
    // Without the fix, only "." would be shown
    const chunks = ['Now', ' I', ' have', ' analyzed', ' the', ' codebase', '.'];

    let messages: Message[] = [];
    for (const chunk of chunks) {
      messages = pushMessage(messages, makeTextMessage(chunk, 'msg-real'));
    }

    expect(messages).toHaveLength(1);
    const text = (messages[0].content[0] as { text: string }).text;
    expect(text).toBe('Now I have analyzed the codebase.');
    // Critical: NOT just "."
    expect(text).not.toBe('.');
  });

  it('handles interleaved streaming from different message IDs', () => {
    const msg1_chunk1 = makeTextMessage('First', 'msg-1');
    const msg1_chunk2 = makeTextMessage(' message', 'msg-1');
    const msg2_chunk1 = makeTextMessage('Second', 'msg-2');

    let messages = pushMessage([], msg1_chunk1);
    messages = pushMessage(messages, msg1_chunk2);
    messages = pushMessage(messages, msg2_chunk1);

    expect(messages).toHaveLength(2);
    expect((messages[0].content[0] as { text: string }).text).toBe('First message');
    expect((messages[1].content[0] as { text: string }).text).toBe('Second');
  });

  it('uses incomingMsg metadata (created, etc) on accumulation', () => {
    const chunk1 = makeTextMessage('Hello', 'msg-1');
    chunk1.created = 1000;
    const chunk2 = makeTextMessage(' world', 'msg-1');
    chunk2.created = 2000;

    const result = pushMessage([chunk1], chunk2);
    // Should use the latest message's metadata
    expect(result[0].created).toBe(2000);
  });
});
