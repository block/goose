import {
  createRecipe as apiCreateRecipe,
  encodeRecipe as apiEncodeRecipe,
  decodeRecipe as apiDecodeRecipe,
} from '../api';
import type {
  CreateRecipeRequest as ApiCreateRecipeRequest,
  CreateRecipeResponse as ApiCreateRecipeResponse,
  RecipeParameter,
  Message as ApiMessage,
  Role,
  MessageContent,
} from '../api';
import type { Message as FrontendMessage } from '../types/message';

// Re-export OpenAPI types with frontend-specific additions
export type Parameter = RecipeParameter;
export type Recipe = import('../api').Recipe & {
  // TODO: Separate these from the raw recipe type
  // Properties added for scheduled execution
  scheduledJobId?: string;
  isScheduledExecution?: boolean;
  // TODO: Separate these from the raw recipe type
  // Legacy frontend properties (not in OpenAPI schema)
  profile?: string;
  goosehints?: string;
  mcps?: number;
};

// Create frontend-compatible type that accepts frontend Message until we can refactor.
export interface CreateRecipeRequest {
  // TODO: Fix this type to match Message OpenAPI spec
  messages: FrontendMessage[];
  title: string;
  description: string;
  activities?: string[];
  author?: {
    contact?: string;
    metadata?: string;
  };
}

export type CreateRecipeResponse = ApiCreateRecipeResponse;

function convertFrontendMessageToApiMessage(frontendMessage: FrontendMessage): ApiMessage {
  // TODO: Fix this type to match Message OpenAPI spec
  return {
    id: frontendMessage.id,
    role: frontendMessage.role as Role,
    content: frontendMessage.content.map((content) => ({
      ...content,
      // Convert toolCall to match API expectations
      ...(content.type === 'toolRequest' && 'toolCall' in content
        ? {
            toolCall: content.toolCall as unknown as { [key: string]: unknown },
          }
        : {}),
    })) as MessageContent[],
    created: frontendMessage.created,
  };
}

export async function createRecipe(request: CreateRecipeRequest): Promise<CreateRecipeResponse> {
  console.log('Creating recipe with request:', JSON.stringify(request, null, 2));

  try {
    const apiRequest: ApiCreateRecipeRequest = {
      messages: request.messages.map(convertFrontendMessageToApiMessage),
      title: request.title,
      description: request.description,
      activities: request.activities || undefined,
      author: request.author
        ? {
            contact: request.author.contact || undefined,
            metadata: request.author.metadata || undefined,
          }
        : undefined,
    };

    const response = await apiCreateRecipe({
      body: apiRequest,
    });

    if (!response.data) {
      throw new Error('No data returned from API');
    }

    return response.data;
  } catch (error) {
    console.error('Failed to create recipe:', error);
    throw error;
  }
}

export async function encodeRecipe(recipe: Recipe): Promise<string> {
  try {
    const response = await apiEncodeRecipe({
      body: { recipe },
    });

    if (!response.data) {
      throw new Error('No data returned from API');
    }

    return response.data.deeplink;
  } catch (error) {
    console.error('Failed to encode recipe:', error);
    throw error;
  }
}

export async function decodeRecipe(deeplink: string): Promise<Recipe> {
  console.log('Decoding recipe from deeplink:', deeplink);

  try {
    const response = await apiDecodeRecipe({
      body: { deeplink },
    });

    if (!response.data) {
      throw new Error('No data returned from API');
    }

    if (!response.data.recipe) {
      console.error('Decoded recipe is null:', response.data);
      throw new Error('Decoded recipe is null');
    }

    return response.data.recipe as Recipe;
  } catch (error) {
    console.error('Failed to decode deeplink:', error);
    throw error;
  }
}

export async function generateDeepLink(recipe: Recipe): Promise<string> {
  const encoded = await encodeRecipe(recipe);
  return `goose://recipe?config=${encoded}`;
}
