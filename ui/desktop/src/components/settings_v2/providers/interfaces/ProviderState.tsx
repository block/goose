interface ProviderMetadata {
  [key: string]: string | number | boolean | null;
}

// runtime data per instance
export default interface ProviderState {
  id: string;
  name: string;
  isConfigured: boolean;
  metadata: ProviderMetadata;
}
