import type { GooseSessionNotification } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import {
  type AcpGooseSessionNotificationHandler,
  type AcpNotificationHandler,
  setAcpGooseSessionNotificationHandler,
  setAcpNotificationHandler,
} from './acpConnection';
import {
  createSessionScopedNotificationRouter,
  type SessionScopedNotificationListener,
} from './sessionScopedNotificationRouter';

export type AcpSessionNotificationListener = SessionScopedNotificationListener<SessionNotification>;
export type AcpGooseSessionNotificationListener =
  SessionScopedNotificationListener<GooseSessionNotification>;

export interface AcpSessionNotificationRouter {
  handler: AcpNotificationHandler;
  subscribe(sessionId: string, listener: AcpSessionNotificationListener): () => void;
}

export interface AcpGooseSessionNotificationRouter {
  handler: AcpGooseSessionNotificationHandler;
  subscribe(sessionId: string, listener: AcpGooseSessionNotificationListener): () => void;
}

export function createAcpSessionNotificationRouter(): AcpSessionNotificationRouter {
  const router = createSessionScopedNotificationRouter<SessionNotification>();

  return {
    handler: {
      handleSessionNotification: router.route,
    },
    subscribe: router.subscribe,
  };
}

export function createAcpGooseSessionNotificationRouter(): AcpGooseSessionNotificationRouter {
  const router = createSessionScopedNotificationRouter<GooseSessionNotification>();

  return {
    handler: {
      handleGooseSessionNotification: router.route,
    },
    subscribe: router.subscribe,
  };
}

const acpSessionNotificationRouter = createAcpSessionNotificationRouter();
const acpGooseSessionNotificationRouter = createAcpGooseSessionNotificationRouter();
let installed = false;

export function installAcpSessionNotificationRouters(): void {
  if (installed) {
    return;
  }

  setAcpNotificationHandler(acpSessionNotificationRouter.handler);
  setAcpGooseSessionNotificationHandler(acpGooseSessionNotificationRouter.handler);
  installed = true;
}

export function subscribeToAcpSession(
  sessionId: string,
  listener: AcpSessionNotificationListener
): () => void {
  return acpSessionNotificationRouter.subscribe(sessionId, listener);
}

export function subscribeToAcpGooseSession(
  sessionId: string,
  listener: AcpGooseSessionNotificationListener
): () => void {
  return acpGooseSessionNotificationRouter.subscribe(sessionId, listener);
}
