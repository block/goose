import {
  DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
  GooseClient,
  type GooseClientCallbacks,
} from '@aaif/goose-sdk';
import { PROTOCOL_VERSION } from '@agentclientprotocol/sdk';
import packageJson from '../../package.json';
import {
  handleAcpGooseSessionNotification,
  handleAcpSessionNotification,
} from './chatNotifications';
import { createWebSocketStream } from './createWebSocketStream';
import { requestAcpElicitation } from './elicitationRequests';
import { requestAcpPermission } from './permissionRequests';

let clientPromise: Promise<GooseClient> | null = null;
let resolvedClient: GooseClient | null = null;

function createClientCallbacks(): () => GooseClientCallbacks {
  return () => ({
    requestPermission: requestAcpPermission,
    unstable_createElicitation: requestAcpElicitation,
    sessionUpdate: handleAcpSessionNotification,
    unstable_sessionUpdate: handleAcpGooseSessionNotification,
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
      elicitation: { form: {} },
      _meta: {
        goose: {
          mcpHostCapabilities: DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
          customNotifications: true,
        },
      },
    },
    clientInfo: {
      name: packageJson.name,
      version: packageJson.version,
    },
  });

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
