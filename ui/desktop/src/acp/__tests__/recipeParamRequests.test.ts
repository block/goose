import type { RequestRecipeParams_unstable } from '@aaif/goose-sdk';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  cancelAcpRecipeParamRequest,
  getAcpRecipeParamRequestsSnapshot,
  requestAcpRecipeParams,
} from '../recipeParamRequests';

vi.mock('../../acpChatFeatureFlag', () => ({
  USE_ACP_CHAT: true,
}));

function recipeParamRequest(): RequestRecipeParams_unstable {
  return {
    sessionId: 'session-1',
    parameters: [
      {
        key: 'topic',
        description: 'Topic',
        input_type: 'string',
        requirement: 'user_prompt',
      },
    ],
  };
}

function setRecipeParameters(values: Record<string, string>): void {
  Object.defineProperty(window, 'appConfig', {
    configurable: true,
    value: {
      get: vi.fn((key: string) => (key === 'recipeParameters' ? values : undefined)),
    },
  });
}

function cancelPendingRecipeParamRequests(): void {
  for (const request of getAcpRecipeParamRequestsSnapshot()) {
    cancelAcpRecipeParamRequest(request.id);
  }
}

describe('ACP recipe param requests', () => {
  beforeEach(() => {
    cancelPendingRecipeParamRequests();
  });

  afterEach(() => {
    cancelPendingRecipeParamRequests();
    Reflect.deleteProperty(window, 'appConfig');
  });

  it('keeps missing user_prompt parameters pending for user input', async () => {
    setRecipeParameters({});

    const response = requestAcpRecipeParams(recipeParamRequest());
    const [pendingRequest] = getAcpRecipeParamRequestsSnapshot();

    expect(pendingRequest).toMatchObject({
      sessionId: 'session-1',
      parameters: [
        {
          key: 'topic',
          requirement: 'user_prompt',
        },
      ],
      initialValues: {},
    });

    cancelAcpRecipeParamRequest(pendingRequest.id);
    await expect(response).resolves.toEqual({ action: 'cancel' });
  });

  it('auto-submits when user_prompt parameters already have configured values', async () => {
    setRecipeParameters({ topic: 'release notes' });

    await expect(requestAcpRecipeParams(recipeParamRequest())).resolves.toEqual({
      action: 'submit',
      values: { topic: 'release notes' },
    });
    expect(getAcpRecipeParamRequestsSnapshot()).toEqual([]);
  });
});
