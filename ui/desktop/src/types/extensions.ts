import type { RecipeExtensionDto } from '@aaif/goose-sdk';

export type Envs = Record<string, string>;

type LegacySseExtensionConfig = {
  description?: string | null;
  name: string;
  type: 'sse';
  uri?: string | null;
};

type FrontendTool = {
  _meta?: Record<string, unknown>;
  annotations?: Record<string, unknown>;
  description?: string;
  execution?: Record<string, unknown>;
  icons?: unknown[];
  inputSchema: Record<string, unknown>;
  name: string;
  outputSchema?: Record<string, unknown>;
  title?: string;
};

type FrontendExtensionConfig = {
  available_tools?: string[] | null;
  bundled?: boolean | null;
  description?: string | null;
  instructions?: string | null;
  name: string;
  tools: FrontendTool[];
  type: 'frontend';
};

type InlinePythonExtensionConfig = {
  available_tools?: string[] | null;
  code: string;
  dependencies?: string[] | null;
  description?: string | null;
  name: string;
  timeout?: number | null;
  type: 'inline_python';
};

export type ExtensionConfig =
  | RecipeExtensionDto
  | LegacySseExtensionConfig
  | FrontendExtensionConfig
  | InlinePythonExtensionConfig;

export type ExtensionEntry = ExtensionConfig & {
  enabled: boolean;
};

export type ExtensionLoadResult = {
  error?: string | null;
  name: string;
  success: boolean;
};
