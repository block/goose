import { describe, expect, it } from 'vitest';
import type { Message } from '../../types/message';
import {
  buildToolCallLookups,
  getPreviousResolvedModels,
  identifyConsecutiveToolCalls,
  messageToolLookupsChanged,
} from '../toolCallChaining';

function textMessage(id: string, text: string): Message {
  return {
    id,
    role: 'assistant',
    created: 1,
    content: [{ type: 'text', text }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

describe('toolCallChaining caches', () => {
  it('reuses tool lookups when only a text message identity changes', () => {
    const first = textMessage('one', 'hello');
    const second = textMessage('two', 'wor');
    const initial = buildToolCallLookups([first, second]);
    const next = buildToolCallLookups([first, textMessage('two', 'world')]);

    expect(next).toBe(initial);
  });

  it('reuses tool lookups when a non-tool message is appended', () => {
    const first = textMessage('one', 'hello');
    const initial = buildToolCallLookups([first]);
    const next = buildToolCallLookups([first, textMessage('two', 'world')]);

    expect(next).toBe(initial);
  });

  it('reuses tool-call chains when only the last text message changes', () => {
    const first = textMessage('one', 'hello');
    const second = textMessage('two', 'wor');
    const initial = identifyConsecutiveToolCalls([first, second]);
    const next = identifyConsecutiveToolCalls([first, textMessage('two', 'world')]);

    expect(next).toBe(initial);
  });

  it('extends previous resolved models when a new message is appended', () => {
    const first: Message = {
      id: 'one',
      role: 'assistant',
      created: 1,
      content: [{ type: 'text', text: 'hello' }],
      metadata: {
        userVisible: true,
        agentVisible: true,
        inference: { resolvedModel: 'gpt-test', requestedModel: 'gpt-test', provider: 'test' },
      },
    };
    const second = textMessage('two', 'world');
    const initial = getPreviousResolvedModels([first]);
    const next = getPreviousResolvedModels([first, second]);

    expect(initial).toEqual([null]);
    expect(next[0]).toBe(initial[0]);
    expect(next[1]).toBe('gpt-test');
  });

  it('ignores lookup rebuilds for messages that do not use the changed tool data', () => {
    const request: Message = {
      id: 'request',
      role: 'assistant',
      created: 1,
      content: [
        {
          type: 'toolRequest',
          id: 'tool-1',
          toolCall: { name: 'developer__shell' },
        },
      ],
      metadata: { userVisible: true, agentVisible: true },
    };
    const other = textMessage('other', 'hello');
    const previous = buildToolCallLookups([request, other]);
    const next = buildToolCallLookups([
      request,
      {
        id: 'response',
        role: 'user',
        created: 2,
        content: [
          {
            type: 'toolResponse',
            id: 'tool-1',
            toolResult: { status: 'success' },
          },
        ],
        metadata: { userVisible: true, agentVisible: true },
      },
    ]);

    expect(messageToolLookupsChanged(other, previous, next)).toBe(false);
    expect(messageToolLookupsChanged(request, previous, next)).toBe(true);
  });

  it('reuses previous resolved models when only the last text message changes', () => {
    const first: Message = {
      id: 'one',
      role: 'assistant',
      created: 1,
      content: [{ type: 'text', text: 'hello' }],
      metadata: {
        userVisible: true,
        agentVisible: true,
        inference: { resolvedModel: 'gpt-test', requestedModel: 'gpt-test', provider: 'test' },
      },
    };
    const second = textMessage('two', 'wor');
    const initial = getPreviousResolvedModels([first, second]);
    const next = getPreviousResolvedModels([first, textMessage('two', 'world')]);

    expect(next).toBe(initial);
    expect(next[1]).toBe('gpt-test');
  });
});
