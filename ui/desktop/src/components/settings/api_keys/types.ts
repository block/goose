export interface ProviderResponse {
  supported: boolean;
  name?: string;
  description?: string;
  models?: string[];
  config_status: Record<string, ConfigDetails>;
}

export interface ConfigDetails {
  key: string;
  is_set: boolean;
  location?: string;
}
