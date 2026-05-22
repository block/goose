import type { GooseSessionNotification } from '@aaif/goose-sdk';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setAcpGooseSessionNotificationHandler } from '../acpConnection';
import {
  createAcpGooseSessionNotificationRouter,
  installAcpSessionNotificationRouters,
} from '../sessionNotificationRouter';

vi.mock('../acpConnection', () => ({
  setAcpNotificationHandler: vi.fn(),
  setAcpGooseSessionNotificationHandler: vi.fn(),
}));

function notification(sessionId: string): GooseSessionNotification {
  return {
    sessionId,
    update: {
      sessionUpdate: 'usage_update',
      used: 10,
      contextLimit: 100,
      accumulatedInputTokens: 3,
      accumulatedOutputTokens: 7,
      accumulatedCost: 0.01,
    },
  };
}

describe('gooseSessionNotificationRouter', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('dispatches notifications only to subscribers for the matching session', async () => {
    const router = createAcpGooseSessionNotificationRouter();
    const sessionOneListener = vi.fn();
    const sessionTwoListener = vi.fn();

    router.subscribe('session-1', sessionOneListener);
    router.subscribe('session-2', sessionTwoListener);

    await router.handler.handleGooseSessionNotification(notification('session-1'));

    expect(sessionOneListener).toHaveBeenCalledTimes(1);
    expect(sessionOneListener).toHaveBeenCalledWith(notification('session-1'));
    expect(sessionTwoListener).not.toHaveBeenCalled();
  });

  it('supports multiple subscribers for one session', async () => {
    const router = createAcpGooseSessionNotificationRouter();
    const firstListener = vi.fn();
    const secondListener = vi.fn();

    router.subscribe('session-1', firstListener);
    router.subscribe('session-1', secondListener);

    await router.handler.handleGooseSessionNotification(notification('session-1'));

    expect(firstListener).toHaveBeenCalledTimes(1);
    expect(secondListener).toHaveBeenCalledTimes(1);
  });

  it('does not dispatch after unsubscribe', async () => {
    const router = createAcpGooseSessionNotificationRouter();
    const listener = vi.fn();
    const unsubscribe = router.subscribe('session-1', listener);

    unsubscribe();
    await router.handler.handleGooseSessionNotification(notification('session-1'));

    expect(listener).not.toHaveBeenCalled();
  });

  it('allows unsubscribe to be called more than once', async () => {
    const router = createAcpGooseSessionNotificationRouter();
    const listener = vi.fn();
    const unsubscribe = router.subscribe('session-1', listener);

    unsubscribe();
    unsubscribe();
    await router.handler.handleGooseSessionNotification(notification('session-1'));

    expect(listener).not.toHaveBeenCalled();
  });

  it('ignores notifications with no subscribers', async () => {
    const router = createAcpGooseSessionNotificationRouter();

    await expect(
      router.handler.handleGooseSessionNotification(notification('session-1'))
    ).resolves.toBeUndefined();
  });

  it('installs the ACP goose session handler explicitly and only once', async () => {
    installAcpSessionNotificationRouters();
    installAcpSessionNotificationRouters();

    expect(setAcpGooseSessionNotificationHandler).toHaveBeenCalledTimes(1);
    expect(setAcpGooseSessionNotificationHandler).toHaveBeenCalledWith({
      handleGooseSessionNotification: expect.any(Function),
    });
  });
});
