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
