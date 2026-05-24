import type { LoadSessionResponse, NewSessionResponse } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import { acpLoadSessionMeta, acpNewSessionToSession, messageToAcpPromptContent } from '../sessions';
import type { Message } from '../../api';

describe('acpLoadSessionMeta', () => {
  it('extracts extension results from ACP response metadata', () => {
    const extensionResults = [
      {
        name: 'developer',
        status: 'success',
      },
    ];

    const response = {
      sessionId: 'session-1',
      _meta: {
        extensionResults,
      },
    } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults,
      recipe: undefined,
      userRecipeValues: undefined,
      workingDir: undefined,
      configOptions: undefined,
    });
  });

  it('extracts recipe session metadata and config options from ACP response metadata', () => {
    const recipe = {
      title: 'Recipe Session',
      description: 'test recipe',
      instructions: 'Do the recipe',
    };
    const userRecipeValues = { target: 'desktop' };
    const configOptions = [
      {
        id: 'mode',
        name: 'Mode',
        type: 'select',
        currentValue: 'approve',
        options: [{ value: 'approve', name: 'Manual' }],
        category: 'mode',
      },
    ];

    const response = {
      sessionId: 'session-1',
      configOptions,
      _meta: {
        recipe,
        userRecipeValues,
        workingDir: '/tmp/project',
      },
    } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults: undefined,
      recipe,
      userRecipeValues,
      workingDir: '/tmp/project',
      configOptions,
    });
  });

  it('handles responses without metadata', () => {
    const response = { sessionId: 'session-1' } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults: undefined,
      recipe: undefined,
      userRecipeValues: undefined,
      workingDir: undefined,
      configOptions: undefined,
    });
  });
});

describe('acpNewSessionToSession', () => {
  it('creates a desktop session snapshot from an ACP new session response', () => {
    const response = {
      sessionId: 'session-1',
      configOptions: [],
    } as unknown as NewSessionResponse;

    const session = acpNewSessionToSession(response, '/tmp/project');

    expect(session).toMatchObject({
      id: 'session-1',
      name: 'New Chat',
      working_dir: '/tmp/project',
      message_count: 0,
      extension_data: {},
      conversation: [],
    });
    expect(session.created_at).toEqual(expect.any(String));
    expect(session.updated_at).toEqual(expect.any(String));
  });
});

describe('messageToAcpPromptContent', () => {
  it('converts text and image content into ACP prompt blocks', () => {
    const message: Message = {
      id: 'message-1',
      role: 'user',
      created: 123,
      content: [
        { type: 'text', text: 'Describe this' },
        { type: 'image', data: 'abc123', mimeType: 'image/png' },
      ],
      metadata: { userVisible: true, agentVisible: true },
    };

    expect(messageToAcpPromptContent(message)).toEqual([
      { type: 'text', text: 'Describe this' },
      { type: 'image', data: 'abc123', mimeType: 'image/png' },
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
