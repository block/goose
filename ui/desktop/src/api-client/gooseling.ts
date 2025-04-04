import { Message } from '../types/message';
import { getApiUrl } from '../config';

export interface Gooseling {
  title: string;
  description: string;
  instructions: string;
  activities?: string[];
  author?: {
    contact?: string;
    metadata?: string;
  };
  extensions?: any[];
  goosehints?: string;
  context?: string[];
}

export interface CreateGooselingRequest {
  messages: Message[];
  title: string;
  description: string;
  activities?: string[];
  author?: {
    contact?: string;
    metadata?: string;
  };
}

export interface CreateGooselingResponse {
  gooseling: Gooseling;
}

export interface LoadGooselingRequest {
  gooseling: Gooseling;
  provider: string;
  model?: string;
  version?: string;
}

export interface LoadGooselingResponse {
  version: string;
}

export async function createGooseling(
  request: CreateGooselingRequest
): Promise<CreateGooselingResponse> {
  const url = getApiUrl('/api/gooseling/create');
  console.log('Creating gooseling at:', url);
  console.log('Request:', JSON.stringify(request, null, 2));

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const errorText = await response.text();
    console.error('Failed to create gooseling:', {
      status: response.status,
      statusText: response.statusText,
      error: errorText,
    });
    throw new Error(`Failed to create gooseling: ${response.statusText} (${errorText})`);
  }

  return response.json();
}

export async function loadGooseling(request: LoadGooselingRequest): Promise<LoadGooselingResponse> {
  const response = await fetch(getApiUrl('/api/gooseling/load'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': window.appConfig.get('secretKey'),
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to load gooseling: ${response.statusText}`);
  }

  return response.json();
}
