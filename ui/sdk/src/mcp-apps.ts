import { RESOURCE_MIME_TYPE } from "@modelcontextprotocol/ext-apps/app-bridge";
import type {
  McpUiAppResourceConfig,
  McpUiAppToolConfig,
} from "@modelcontextprotocol/ext-apps/server";
import type {
  BlobResourceContents,
  ReadResourceResult,
  TextResourceContents,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";

export const GOSLING_MCP_UI_EXTENSION_ID = "io.modelcontextprotocol/ui" as const;

export interface GoslingMcpUiExtensionSettings {
  mimeTypes: string[];
}

export interface GoslingMcpHostCapabilities {
  extensions: Record<string, GoslingMcpUiExtensionSettings>;
}

export type GoslingToolUiMetadata = Extract<
  McpUiAppToolConfig["_meta"],
  { ui: unknown }
>["ui"];

export type GoslingToolMetadata = NonNullable<Tool["_meta"]> & {
  ui?: GoslingToolUiMetadata;
  gosling_extension?: string;
};

export type GoslingSessionTool = Tool & {
  meta?: GoslingToolMetadata;
  _meta?: GoslingToolMetadata;
};

export type GoslingTextResourceContents = TextResourceContents;

export type GoslingBlobResourceContents = BlobResourceContents;

export type GoslingResourceContents = TextResourceContents | BlobResourceContents;

export type GoslingReadResourceResult = ReadResourceResult;

export type GoslingResourceMetadata = NonNullable<
  Extract<NonNullable<McpUiAppResourceConfig["_meta"]>, { ui?: unknown }>["ui"]
>;

export interface GoslingMcpAppToolPayload {
  toolName: string;
  extensionName: string;
  resourceUri: string;
  toolMeta?: GoslingToolMetadata;
  resourceResult?: GoslingReadResourceResult | null;
  readError?: string;
}

export interface GoslingToolCallUpdateMeta {
  gosling?: {
    mcpApp?: GoslingMcpAppToolPayload;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export const DEFAULT_GOSLING_MCP_HOST_CAPABILITIES: GoslingMcpHostCapabilities = {
  extensions: {
    [GOSLING_MCP_UI_EXTENSION_ID]: {
      mimeTypes: [RESOURCE_MIME_TYPE],
    },
  },
};
