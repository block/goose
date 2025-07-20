import { Message } from '../types/message';
import { getApiUrl } from '../config';
import { FullExtensionConfig } from '../extensions';
import { safeJsonParse } from '../utils/jsonUtils';

export interface Parameter {
  key: string;
  description: string;
  input_type: string;
  default?: string;
  requirement: 'required' | 'optional' | 'user_prompt';
}

export interface Recipe {
  title: string;
  description: string;
  instructions: string;
  prompt?: string;
  activities?: string[];
  parameters?: Parameter[];
  author?: {
    contact?: string;
    metadata?: string;
  };
  extensions?: FullExtensionConfig[];
  goosehints?: string;
  context?: string[];
  profile?: string;
  mcps?: number;
  // Properties added for scheduled execution
  scheduledJobId?: string;
  isScheduledExecution?: boolean;
}

export interface CreateRecipeRequest {
  messages: Message[];
  title: string;
  description: string;
  activities?: string[];
  author?: {
    contact?: string;
    metadata?: string;
  };
}

export interface CreateRecipeResponse {
  recipe: Recipe | null;
  error: string | null;
}

export async function createRecipe(request: CreateRecipeRequest): Promise<CreateRecipeResponse> {
  const url = getApiUrl('/recipes/create');
  console.log('Creating recipe at:', url);
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
    console.error('Failed to create recipe:', {
      status: response.status,
      statusText: response.statusText,
      error: errorText,
    });
    throw new Error(`Failed to create recipe: ${response.statusText} (${errorText})`);
  }

  return safeJsonParse<CreateRecipeResponse>(response, 'Server failed to create recipe:');
}

export interface EncodeRecipeRequest {
  recipe: Recipe;
}

export interface EncodeRecipeResponse {
  deeplink: string;
}

export interface DecodeRecipeRequest {
  deeplink: string;
}

export interface DecodeRecipeResponse {
  recipe: Recipe;
}

export async function encodeRecipe(recipe: Recipe): Promise<string> {
  const url = getApiUrl('/recipes/encode');

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ recipe } as EncodeRecipeRequest),
  });

  if (!response.ok) {
    throw new Error(`Failed to encode recipe: ${response.status} ${response.statusText}`);
  }

  const data: EncodeRecipeResponse = await response.json();
  return data.deeplink;
}

export async function decodeRecipe(deeplink: string): Promise<Recipe> {
  const url = getApiUrl('/recipes/decode');

  console.log('Decoding recipe from deeplink:', deeplink);
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ deeplink } as DecodeRecipeRequest),
  });

  if (!response.ok) {
    console.error('Failed to decode deeplink:', {
      status: response.status,
      statusText: response.statusText,
    });
    throw new Error(`Failed to decode deeplink: ${response.status} ${response.statusText}`);
  }

  const data: DecodeRecipeResponse = await response.json();
  if (!data.recipe) {
    console.error('Decoded recipe is null:', data);
    throw new Error('Decoded recipe is null');
  }
  return data.recipe;
}

export async function generateDeepLink(recipe: Recipe): Promise<string> {
  const encoded = await encodeRecipe(recipe);
  return `goose://recipe?config=${encoded}`;
}
