import { Content } from '../types/message';

export interface ToolSchema {
  type: 'object';
  required: string[];
  properties: Record<
    string,
    {
      type: string;
      description: string;
      enum?: string[];
      items?: {
        type: string;
      };
    }
  >;
}

export interface FrontendToolDefinition {
  name: string;
  description: string;
  inputSchema: ToolSchema;
}

export interface FrontendExtensionConfig {
  type: 'frontend';
  name: string;
  tools: FrontendToolDefinition[];
  instructions?: string;
}

export type ToolExecutor = (args: Record<string, unknown>) => Promise<Content[]>;

export interface RegisteredTool extends FrontendToolDefinition {
  execute: ToolExecutor;
}
