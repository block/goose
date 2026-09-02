import type { RecipeExtensionDto } from '@aaif/goose-sdk';

export type Envs = Record<string, string>;

export type ExtensionConfig = RecipeExtensionDto;

export type ExtensionEntry = ExtensionConfig & {
  enabled: boolean;
};

export type ExtensionLoadResult = {
  error?: string | null;
  name: string;
  success: boolean;
};
