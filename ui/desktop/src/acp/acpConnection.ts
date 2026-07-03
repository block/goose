import {
  DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
  GooseClient,
  type GooseClientCallbacks,
} from '@aaif/goose-sdk';
import { PROTOCOL_VERSION, type InitializeResponse } from '@agentclientprotocol/sdk';
import packageJson from '../../package.json';
import {
  handleAcpGooseSessionNotification,
  handleAcpSessionNotification,
} from './chatNotifications';
import { createWebSocketStream } from './createWebSocketStream';
import { requestAcpElicitation } from './elicitationRequests';
import { requestAcpPermission } from './permissionRequests';
import { requestAcpRecipeParams } from './recipeParamRequests';

type InitializedAcpClient = {
  client: GooseClient;
  initializeResponse: InitializeResponse;
};

const ACP_INITIALIZE_TIMEOUT_MS = 10_000;

let clientPromise: Promise<InitializedAcpClient> | null = null;
let resolvedClient: InitializedAcpClient | null = null;

function createClientCallbacks(): () => GooseClientCallbacks {
  return () => ({
    requestPermission: requestAcpPermission,
    unstable_createElicitation: requestAcpElicitation,
    unstable_sessionRecipeRequestParams: requestAcpRecipeParams,
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

async function withTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  const timeout = new Promise<T>((_, reject) => {
    timeoutId = setTimeout(() => reject(new Error(message)), timeoutMs);
  });

  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
    }
  }
}

async function initializeConnection(): Promise<InitializedAcpClient> {
  const wsUrl = await window.electron.getAcpUrl();
  if (!wsUrl) {
    throw new Error('ACP URL is not available');
  }

  const stream = createWebSocketStream(wsUrl);
  const client = new GooseClient(createClientCallbacks(), stream);

  try {
    const initializeResponse = await withTimeout(
      client.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: {
          elicitation: { form: {} },
          _meta: {
            goose: {
              mcpHostCapabilities: DEFAULT_GOOSE_MCP_HOST_CAPABILITIES,
              customNotifications: true,
              recipeParameterRequests: true,
            },
          },
        },
        clientInfo: {
          name: packageJson.name,
          version: packageJson.version,
        },
      }),
      ACP_INITIALIZE_TIMEOUT_MS,
      `ACP initialize timed out after ${ACP_INITIALIZE_TIMEOUT_MS}ms`
    );

    monitorConnection(client);
    return { client, initializeResponse };
  } catch (error) {
    stream.close();
    throw error;
  }
}

export async function getAcpClient(): Promise<GooseClient> {
  return (await getInitializedAcpClient()).client;
}

export function getAcpClientSync(): GooseClient | null {
  return resolvedClient?.client ?? null;
}

export async function getAcpInitializeResponse(): Promise<InitializeResponse> {
  return (await getInitializedAcpClient()).initializeResponse;
}

export function isAcpClientReady(): boolean {
  return resolvedClient !== null;
}

async function getInitializedAcpClient(): Promise<InitializedAcpClient> {
  if (resolvedClient) {
    return resolvedClient;
  }

  if (!clientPromise) {
    clientPromise = initializeConnection()
      .then((clientState) => {
        resolvedClient = clientState;
        return clientState;
      })
      .catch((error) => {
        clientPromise = null;
        throw error;
      });
  }

  return clientPromise;
}
