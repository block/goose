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

export interface Provider {
  id: string; // Lowercase key (e.g., "openai")
  name: string; // Provider name (e.g., "OpenAI")
  description: string; // Description of the provider
  models: string[]; // List of supported models
  requiredKeys: string[]; // List of required keys
}
