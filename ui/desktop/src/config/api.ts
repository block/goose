import { Value } from 'yaml';

export interface UpsertConfigQuery {
  key: string;
  value: Value;
  isSecret?: boolean;
}

export interface ConfigKeyQuery {
  key: string;
}

export interface ExtensionQuery {
  name: string;
  config: Value;
}

export interface ConfigResponse {
  config: Record<string, Value>;
}

const API_BASE = 'http://localhost:3000'; // Update this with your actual API base URL

export class ConfigAPI {
  static async readAllConfig(): Promise<ConfigResponse> {
    const response = await fetch(`${API_BASE}/config`);
    if (!response.ok) {
      throw new Error('Failed to fetch config');
    }
    return response.json();
  }

  static async upsertConfig(query: UpsertConfigQuery): Promise<string> {
    const params = new URLSearchParams({
      key: query.key,
      value: JSON.stringify(query.value),
      ...(query.isSecret && { is_secret: String(query.isSecret) }),
    });

    const response = await fetch(`${API_BASE}/config/upsert?${params}`, {
      method: 'POST',
    });

    if (!response.ok) {
      throw new Error('Failed to upsert config');
    }
    return response.text();
  }

  static async removeConfig(key: string): Promise<string> {
    const params = new URLSearchParams({ key });
    const response = await fetch(`${API_BASE}/config/remove?${params}`, {
      method: 'DELETE',
    });

    if (!response.ok) {
      throw new Error('Failed to remove config');
    }
    return response.text();
  }

  static async readConfig(key: string): Promise<Value> {
    const params = new URLSearchParams({ key });
    const response = await fetch(`${API_BASE}/config/read?${params}`);

    if (!response.ok) {
      throw new Error('Failed to read config');
    }
    return response.json();
  }

  static async addExtension(extension: ExtensionQuery): Promise<string> {
    const response = await fetch(`${API_BASE}/config/extension`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(extension),
    });

    if (!response.ok) {
      throw new Error('Failed to add extension');
    }
    return response.text();
  }

  static async removeExtension(name: string): Promise<string> {
    const params = new URLSearchParams({ key: name });
    const response = await fetch(`${API_BASE}/config/extension?${params}`, {
      method: 'DELETE',
    });

    if (!response.ok) {
      throw new Error('Failed to remove extension');
    }
    return response.text();
  }
}
