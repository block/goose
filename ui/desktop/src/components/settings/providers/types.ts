// transformation of the response provided by secrets/provider endpoint
export interface Provider {
    id: string;
    name: string;
    keys: string[];
    description: string;
    canDelete?: boolean;
    supported: boolean;
    order: number;
}

export interface SecretDetails {
    key: string;
    is_set: boolean;
    location?: string;
}

// returned by the secrets/providers endpoint
export interface ProviderResponse {
    supported: boolean;
    name?: string;
    description?: string;
    models?: string[];
    secret_status: Record<string, SecretDetails>;
}

// Represents the backend's secret structure for a single secret
export interface RawSecretStatus {
    location: string;  // Where the secret is stored (e.g., "keyring")
    is_set: boolean;   // Whether the secret is configured
}

// Represents the transformed structure of a secret in the frontend
export interface TransformedSecret {
    key: string;       // The secret's key (e.g., "OPENAI_API_KEY")
    location: string;  // Where the secret is stored (e.g., "keyring")
    is_set: boolean;    // Whether the secret is set
}