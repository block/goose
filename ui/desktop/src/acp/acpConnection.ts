import {
  DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
  GooseClient,
  type GooseClientCallbacks,
  type GooseSessionNotification_unstable as GooseSessionNotification,
  type GooseInitializeRequest,
} from '@aaif/goose-sdk';
import {
  PROTOCOL_VERSION,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionNotification,
} from '@agentclientprotocol/sdk';
import packageJson from '../../package.json';
import { createWebSocketStream } from './createWebSocketStream';
import { createSessionScopedNotificationRouter } from './sessionScopedNotificationRouter';

let clientPromise: Promise<GooseClient> | null = null;
let resolvedClient: GooseClient | null = null;
let permissionHandler: AcpPermissionHandler | null = null;

const sessionRouter = createSessionScopedNotificationRouter<SessionNotification>();
const gooseSessionRouter = createSessionScopedNotificationRouter<GooseSessionNotification>();

export const subscribeToAcpSession = sessionRouter.subscribe;
export const subscribeToAcpGooseSession = gooseSessionRouter.subscribe;

export type AcpPermissionHandler = (
  request: RequestPermissionRequest
) => Promise<RequestPermissionResponse>;

export function setAcpPermissionHandler(handler: AcpPermissionHandler | null): void {
  permissionHandler = handler;
}

function createClientCallbacks(): () => GooseClientCallbacks {
  return () => ({
    requestPermission: async (request) => {
      if (permissionHandler) {
        return permissionHandler(request);
      }

      console.warn('ACP permission request received before a permission handler was registered');
      return {
        outcome: {
          outcome: 'cancelled',
        },
      };
    },
    sessionUpdate: sessionRouter.route,
    unstable_sessionUpdate: gooseSessionRouter.route,
  });
}

function monitorConnection(client: GooseClient): void {
  client.closed
    .then(() => {
      resolvedClient = null;
      clientPromise = null;
    })
    .catch(() => {
      resolvedClient = null;
      clientPromise = null;
    });
}

async function initializeConnection(): Promise<GooseClient> {
  const wsUrl = await window.electron.getAcpUrl();
  if (!wsUrl) {
    throw new Error('ACP URL is not available');
  }

  const stream = createWebSocketStream(wsUrl);
  const client = new GooseClient(createClientCallbacks(), stream);

  await client.initialize({
    protocolVersion: PROTOCOL_VERSION,
    clientCapabilities: {
      _meta: {
        goose: {
          mcpHostCapabilities: DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
        },
      },
    },
    clientInfo: {
      name: packageJson.name,
      version: packageJson.version,
    },
  } satisfies GooseInitializeRequest);

  monitorConnection(client);
  return client;
}

export async function getAcpClient(): Promise<GooseClient> {
  if (resolvedClient) {
    return resolvedClient;
  }

  if (!clientPromise) {
    clientPromise = initializeConnection()
      .then((client) => {
        resolvedClient = client;
        return client;
      })
      .catch((error) => {
        clientPromise = null;
        throw error;
      });
  }

  return clientPromise;
}

export function getAcpClientSync(): GooseClient | null {
  return resolvedClient;
}

export function isAcpClientReady(): boolean {
  return resolvedClient !== null;
}
