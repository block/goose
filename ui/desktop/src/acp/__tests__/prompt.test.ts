import { describe, expect, it } from 'vitest';
import type { Message } from '../../types/message';
import { messageToAcpPromptContent } from '../prompt';

describe('messageToAcpPromptContent', () => {
  it('converts text and image content into ACP prompt blocks', () => {
    const message: Message = {
      id: 'message-1',
      role: 'user',
      created: 123,
      content: [
        { type: 'text', text: 'Describe this' },
        {
          type: 'image',
          data: 'abc123',
          mimeType: 'image/png',
          _meta: { source: 'acp' },
          annotations: { priority: 0.5 },
        },
      ],
      metadata: { userVisible: true, agentVisible: true },
    };

    expect(messageToAcpPromptContent(message)).toEqual([
      { type: 'text', text: 'Describe this' },
      {
        type: 'image',
        data: 'abc123',
        mimeType: 'image/png',
        _meta: { source: 'acp' },
        annotations: { priority: 0.5 },
      },
    ]);
  });

  it('forwards text annotations for assistant-only skill instructions', () => {
    const message: Message = {
      id: 'message-skills',
      role: 'user',
      created: 123,
      content: [
        {
          type: 'text',
          text: 'Use the load_skill tool to load the following skills: "review".',
          annotations: { audience: ['assistant'] },
        },
        { type: 'text', text: 'Review this PR' },
      ],
      metadata: {
        userVisible: true,
        agentVisible: true,
        chips: [{ label: 'review', type: 'skill' }],
      },
    };

    expect(messageToAcpPromptContent(message)).toEqual([
      {
        type: 'text',
        text: 'Use the load_skill tool to load the following skills: "review".',
        annotations: { audience: ['assistant'] },
      },
      { type: 'text', text: 'Review this PR' },
    ]);
  });

  it('omits empty text content and unsupported content blocks', () => {
    const message: Message = {
      id: 'message-1',
      role: 'user',
      created: 123,
      content: [
        { type: 'text', text: '   ' },
        {
          type: 'toolResponse',
          id: 'tool-1',
          toolResult: { status: 'success', value: [] },
        },
      ],
      metadata: { userVisible: true, agentVisible: true },
    } as Message;

    expect(messageToAcpPromptContent(message)).toEqual([]);
  });
});
