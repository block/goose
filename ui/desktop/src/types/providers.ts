export type ProviderType = 'Preferred' | 'Builtin' | 'Declarative' | 'Custom';

export type ThinkingEffort = 'off' | 'low' | 'medium' | 'high' | 'max';

export type ConfigKey = {
  default?: string | null;
  device_code_flow?: boolean;
  name: string;
  oauth_flow: boolean;
  primary?: boolean;
  required: boolean;
  secret: boolean;
};

export type UpdateCustomProviderRequest = {
  api_key: string;
  api_url: string;
  base_path?: string | null;
  catalog_provider_id?: string | null;
  display_name: string;
  engine: string;
  headers?: Record<string, string> | null;
  models: string[];
  preserves_thinking?: boolean | null;
  requires_auth?: boolean;
  supports_streaming?: boolean | null;
};
