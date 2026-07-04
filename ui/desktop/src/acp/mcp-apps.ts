import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import type { ToolListItem } from '@repo-makeover/gosling-sdk';
import { getAcpClient } from './acpConnection';

type JsonRecord = Record<string, unknown>;
export type McpAppTool = ToolListItem;
export type McpAppResourceResponse = {
  uri: string;
  mimeType: string | null;
  text: string;
  _meta?: Record<string, unknown>;
};
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

export async function listMcpAppTools(
  sessionId: string,
  extensionName?: string
): Promise<McpAppTool[]> {
  const client = await getAcpClient();
  const response = await client.gosling.toolsList_unstable({ sessionId });
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
  const response = await client.gosling.resourcesRead_unstable({
    sessionId,
    uri,
    extensionName,
  });
  return flattenReadResourceResult(response.result, uri);
}

export async function callMcpAppTool(
  sessionId: string,
  extensionName: string,
  name: string,
  args: Record<string, unknown> | undefined
): Promise<CallToolResult> {
  const fullToolName = `${extensionName}__${name}`;
  const client = await getAcpClient();
  const response = await client.gosling.toolsCall_unstable({
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
