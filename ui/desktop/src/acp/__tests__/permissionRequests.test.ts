import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  cancelAcpPermissionRequestsForSession,
  requestAcpPermission,
  resolveAcpPermissionRequest,
} from '../permissionRequests';

vi.mock('../../acpChatFeatureFlag', () => ({
  USE_ACP_CHAT: true,
}));

vi.mock('../chatSessionStore', () => ({
  acpChatSessionStore: {
    applyPermissionRequest: vi.fn(),
  },
}));

function permissionRequest(sessionId: string, toolCallId: string): RequestPermissionRequest {
  return {
    sessionId,
    options: [
      { optionId: 'allow-once', name: 'Allow once', kind: 'allow_once' },
      { optionId: 'reject-once', name: 'Deny once', kind: 'reject_once' },
    ],
    toolCall: {
      toolCallId,
      title: 'Read file',
      rawInput: { path: 'README.md' },
      content: [
        {
          type: 'content',
          content: { type: 'text', text: 'Allow reading README.md?' },
        },
      ],
    },
  };
}

const TEST_SESSION_IDS = ['session-1', 'session-2'];

async function expectStillPending(promise: Promise<RequestPermissionResponse>): Promise<void> {
  let settled = false;
  promise.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    }
  );

  await Promise.resolve();

  expect(settled).toBe(false);
}

describe('ACP permission requests', () => {
  beforeEach(() => {
    for (const sessionId of TEST_SESSION_IDS) {
      cancelAcpPermissionRequestsForSession(sessionId);
    }
  });

  afterEach(() => {
    for (const sessionId of TEST_SESSION_IDS) {
      cancelAcpPermissionRequestsForSession(sessionId);
    }
  });

  it('keeps permission requests pending until explicit resolve', async () => {
    const response = requestAcpPermission(permissionRequest('session-1', 'tool-1'));

    await expectStillPending(response);

    expect(resolveAcpPermissionRequest('session-1', 'tool-1', 'allow_once')).toBe(true);
    await expect(response).resolves.toEqual({
      outcome: {
        outcome: 'selected',
        optionId: 'allow-once',
      },
    });
  });

  it('cancels only pending requests for the requested session', async () => {
    const sessionOneResponse = requestAcpPermission(permissionRequest('session-1', 'tool-1'));
    const sessionTwoResponse = requestAcpPermission(permissionRequest('session-2', 'tool-2'));

    cancelAcpPermissionRequestsForSession('session-1');

    await expect(sessionOneResponse).resolves.toEqual({
      outcome: {
        outcome: 'cancelled',
      },
    });
    await expectStillPending(sessionTwoResponse);

    expect(resolveAcpPermissionRequest('session-2', 'tool-2', 'deny_once')).toBe(true);
    await expect(sessionTwoResponse).resolves.toEqual({
      outcome: {
        outcome: 'selected',
        optionId: 'reject-once',
      },
    });
  });

  it('cancels an older duplicate request for the same session and tool call', async () => {
    const firstResponse = requestAcpPermission(permissionRequest('session-1', 'tool-1'));
    const secondResponse = requestAcpPermission(permissionRequest('session-1', 'tool-1'));

    await expect(firstResponse).resolves.toEqual({
      outcome: {
        outcome: 'cancelled',
      },
    });
    await expectStillPending(secondResponse);

    expect(resolveAcpPermissionRequest('session-1', 'tool-1', 'allow_once')).toBe(true);
    await expect(secondResponse).resolves.toEqual({
      outcome: {
        outcome: 'selected',
        optionId: 'allow-once',
      },
    });
  });
});
