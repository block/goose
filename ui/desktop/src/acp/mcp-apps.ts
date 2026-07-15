import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import type {
  GooseClient,
  ResourceListChangedNotification_unstable,
  ResourceUpdatedNotification_unstable,
  ToolListItem,
} from '@aaif/goose-sdk';
import type { GooseApp } from '../types/apps';
import { getAcpClient, onAcpClientConnected } from './acpConnection';
import { normalizeAcpError } from './errors';

type JsonRecord = Record<string, unknown>;
export type McpAppTool = ToolListItem;
export type McpAppResourceResponse = {
  uri: string;
  mimeType: string | null;
  text: string;
  _meta?: Record<string, unknown>;
};
export type McpAppResourceNotification =
  | ({ type: 'updated' } & ResourceUpdatedNotification_unstable)
  | ({ type: 'listChanged' } & ResourceListChangedNotification_unstable);

const resourceNotificationListeners = new Set<(notification: McpAppResourceNotification) => void>();
const RESOURCE_REPLAY_MAX_ATTEMPTS = 3;
const RESOURCE_REPLAY_BASE_DELAY_MS = 250;

export function handleAcpResourceUpdated(
  notification: ResourceUpdatedNotification_unstable
): Promise<void> {
  resourceNotificationListeners.forEach((listener) =>
    listener({ type: 'updated', ...notification })
  );
  return Promise.resolve();
}

export function handleAcpResourceListChanged(
  notification: ResourceListChangedNotification_unstable
): Promise<void> {
  resourceNotificationListeners.forEach((listener) =>
    listener({ type: 'listChanged', ...notification })
  );
  return Promise.resolve();
}

export function onMcpAppResourceNotification(
  listener: (notification: McpAppResourceNotification) => void
): () => void {
  resourceNotificationListeners.add(listener);
  return () => resourceNotificationListeners.delete(listener);
}
type ToolCallResponseLike = {
  content?: Array<unknown>;
  structuredContent?: unknown;
  isError?: boolean;
  _meta?: unknown;
};

function isRecord(value: unknown): value is JsonRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringField(record: JsonRecord, key: string): string | undefined {
  const value = record[key];
  return typeof value === 'string' ? value : undefined;
}

function metaField(record: JsonRecord): McpAppResourceResponse['_meta'] {
  const meta = record._meta ?? record.meta;
  return isRecord(meta) ? meta : undefined;
}

function decodeBase64Text(blob: string): string {
  let bytes: Uint8Array;
  if (typeof globalThis.atob === 'function') {
    const binary = globalThis.atob(blob);
    bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  } else {
    bytes = Uint8Array.from(Buffer.from(blob, 'base64'));
  }
  return new TextDecoder().decode(bytes);
}

function flattenReadResourceResult(result: unknown, fallbackUri: string): McpAppResourceResponse {
  const contents = isRecord(result) && Array.isArray(result.contents) ? result.contents : [];
  const first = contents.find(isRecord);
  if (!first) {
    throw new Error(`Resource '${fallbackUri}' returned no contents`);
  }

  const uri = stringField(first, 'uri') ?? fallbackUri;
  const mimeType = stringField(first, 'mimeType') ?? stringField(first, 'mime_type') ?? null;
  const text = stringField(first, 'text') ?? decodeBase64Text(stringField(first, 'blob') ?? '');

  return {
    uri,
    mimeType,
    text,
    _meta: metaField(first),
  };
}

function acpApp(value: unknown): GooseApp | null {
  if (!isRecord(value)) return null;
  return value as GooseApp;
}

export async function listMcpApps(sessionId?: string): Promise<GooseApp[]> {
  const client = await getAcpClient();
  const response = await client.goose.appsList_unstable(sessionId ? { sessionId } : {});
  return (response.apps ?? []).map(acpApp).filter((app): app is GooseApp => !!app);
}

export async function exportMcpApp(name: string): Promise<string> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.appsExport_unstable({ name });
    return response.html;
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to export app');
  }
}

export async function importMcpApp(html: string): Promise<void> {
  try {
    const client = await getAcpClient();
    await client.goose.appsImport_unstable({ html });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to import app');
  }
}

export async function deleteMcpApp(name: string): Promise<void> {
  try {
    const client = await getAcpClient();
    await client.goose.appsDelete_unstable({ name });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to delete app');
  }
}

export async function listMcpAppTools(
  sessionId: string,
  extensionName?: string
): Promise<McpAppTool[]> {
  const client = await getAcpClient();
  const response = await client.goose.toolsList_unstable({ sessionId });
  const tools = response.tools;
  if (!extensionName) return tools;

  const prefix = `${extensionName}__`;
  return tools.filter((tool) => tool.name.startsWith(prefix));
}

export async function readMcpAppResource(
  sessionId: string,
  extensionName: string,
  uri: string
): Promise<McpAppResourceResponse> {
  const client = await getAcpClient();
  const response = await client.goose.resourcesRead_unstable({
    sessionId,
    uri,
    extensionName,
  });
  return flattenReadResourceResult(response.result, uri);
}

export async function subscribeMcpAppResource(
  sessionId: string,
  extensionName: string,
  uri: string,
  subscriberId: string
): Promise<void> {
  const client = await getAcpClient();
  await subscribeMcpAppResourceWithClient(client, sessionId, extensionName, uri, subscriberId);
}

function subscribeMcpAppResourceWithClient(
  client: GooseClient,
  sessionId: string,
  extensionName: string,
  uri: string,
  subscriberId: string
): Promise<void> {
  return client.goose.resourcesSubscribe_unstable({
    sessionId,
    extensionName,
    uri,
    subscriberId,
  });
}

export async function unsubscribeMcpAppResource(
  sessionId: string,
  extensionName: string,
  uri: string,
  subscriberId: string
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.resourcesUnsubscribe_unstable({
    sessionId,
    extensionName,
    uri,
    subscriberId,
  });
}

export class McpAppResourceSubscriptions {
  private readonly uris = new Set<string>();
  private operation = Promise.resolve();
  private latestClient: GooseClient | null = null;
  private subscriptionClient: GooseClient | null = null;
  private disposing = false;
  private readonly stopClientConnected: () => void;

  constructor(
    private readonly sessionId: string,
    private readonly extensionName: string,
    private readonly subscriberId: string,
    private readonly onUpdated: (uri: string) => void,
    private readonly onListChanged: () => void
  ) {
    this.stopClientConnected = onAcpClientConnected((client) => {
      if (this.disposing || (this.latestClient === client && this.subscriptionClient === client)) {
        return;
      }
      this.latestClient = client;
      void this.enqueue(() => this.replay(client));
    });
  }

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.operation.then(operation);
    this.operation = result.then(
      () => undefined,
      () => undefined
    );
    return result;
  }

  subscribe(uri: string): Promise<void> {
    return this.enqueue(async () => {
      const wasSubscribed = this.uris.has(uri);
      this.uris.add(uri);
      try {
        const client = await getAcpClient();
        this.latestClient = client;
        this.subscriptionClient = client;
        await subscribeMcpAppResourceWithClient(
          client,
          this.sessionId,
          this.extensionName,
          uri,
          this.subscriberId
        );
      } catch (error) {
        if (!wasSubscribed) this.uris.delete(uri);
        throw error;
      }
    });
  }

  private async replaySubscription(client: GooseClient, uri: string): Promise<boolean> {
    for (let attempt = 0; attempt < RESOURCE_REPLAY_MAX_ATTEMPTS; attempt += 1) {
      if (this.disposing || this.latestClient !== client || !this.uris.has(uri)) return false;
      try {
        await subscribeMcpAppResourceWithClient(
          client,
          this.sessionId,
          this.extensionName,
          uri,
          this.subscriberId
        );
        return true;
      } catch {
        if (attempt + 1 === RESOURCE_REPLAY_MAX_ATTEMPTS) return false;
        await new Promise<void>((resolve) => {
          setTimeout(resolve, RESOURCE_REPLAY_BASE_DELAY_MS * 2 ** attempt);
        });
      }
    }
    return false;
  }

  private async replay(client: GooseClient): Promise<void> {
    if (this.disposing || this.latestClient !== client || this.subscriptionClient === client) {
      return;
    }
    let replayedAll = true;
    for (const uri of [...this.uris]) {
      if (this.disposing || this.latestClient !== client) return;
      if (!(await this.replaySubscription(client, uri))) {
        replayedAll = false;
        continue;
      }
      if (!this.disposing && this.latestClient === client && this.uris.has(uri)) {
        this.onUpdated(uri);
      }
    }
    if (!this.disposing && this.latestClient === client) {
      this.subscriptionClient = replayedAll ? client : null;
    }
  }

  private async unsubscribeNow(uri: string): Promise<void> {
    if (!this.uris.has(uri)) return;
    await unsubscribeMcpAppResource(this.sessionId, this.extensionName, uri, this.subscriberId);
    this.uris.delete(uri);
  }

  unsubscribe(uri: string): Promise<void> {
    return this.enqueue(() => this.unsubscribeNow(uri));
  }

  notify(notification: McpAppResourceNotification): void {
    if (
      notification.sessionId !== this.sessionId ||
      notification.extensionName !== this.extensionName
    ) {
      return;
    }
    if (notification.type === 'updated') {
      if (this.uris.has(notification.uri)) this.onUpdated(notification.uri);
    } else {
      this.onListChanged();
    }
  }

  dispose(): Promise<void> {
    this.disposing = true;
    this.stopClientConnected();
    return this.enqueue(async () => {
      const failures: unknown[] = [];
      for (const uri of [...this.uris]) {
        try {
          await this.unsubscribeNow(uri);
        } catch {
          try {
            await this.unsubscribeNow(uri);
          } catch (error) {
            failures.push(error);
          }
        }
      }
      if (failures.length) throw failures[0];
    });
  }
}

export async function callMcpAppTool(
  sessionId: string,
  extensionName: string,
  name: string,
  args: Record<string, unknown> | undefined
): Promise<CallToolResult> {
  const fullToolName = `${extensionName}__${name}`;
  const client = await getAcpClient();
  const response = await client.goose.toolsCall_unstable({
    sessionId,
    name: fullToolName,
    arguments: args || {},
  });
  return callToolResponseToMcpResult(response);
}

function callToolResponseToMcpResult(response: ToolCallResponseLike | undefined): CallToolResult {
  return {
    content: (response?.content || []) as unknown as CallToolResult['content'],
    isError: response?.isError || false,
    structuredContent: response?.structuredContent as { [key: string]: unknown } | undefined,
    _meta: response?._meta as { [key: string]: unknown } | undefined,
  };
}
