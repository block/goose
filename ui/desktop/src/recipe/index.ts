import {
  encodeRecipe as apiEncodeRecipe,
  decodeRecipe as apiDecodeRecipe,
  scanRecipe as apiScanRecipe,
} from '../api';
import type { RecipeParameter } from '../api';

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

export async function scanRecipe(recipe: Recipe): Promise<{ has_security_warnings: boolean }> {
  try {
    const response = await apiScanRecipe({
      body: { recipe },
    });

    if (!response.data) {
      throw new Error('No data returned from API');
    }

    return response.data;
  } catch (error) {
    console.error('Failed to scan recipe:', error);
    throw error;
  }
}

export async function generateDeepLink(recipe: Recipe): Promise<string> {
  const encoded = await encodeRecipe(recipe);
  return `goose://recipe?config=${encoded}`;
}
