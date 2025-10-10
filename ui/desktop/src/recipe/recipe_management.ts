import { Recipe, saveRecipe as saveRecipeApi, listRecipes, RecipeManifestResponse } from '../api';

export async function saveRecipe(recipe: Recipe, recipeId?: string | null): Promise<void> {
  try {
    await saveRecipeApi({
      body: {
        recipe,
        id: recipeId,
      },
      throwOnError: true,
    });
  } catch (error) {
    let error_message = 'unknown error';
    if (typeof error === 'object' && error !== null && 'message' in error) {
      error_message = error.message as string;
    }
    throw new Error(error_message);
  }
}

export async function listSavedRecipes(): Promise<RecipeManifestResponse[]> {
  try {
    const listRecipeResponse = await listRecipes();
    return listRecipeResponse?.data?.recipe_manifest_responses ?? [];
  } catch (error) {
    console.warn('Failed to list saved recipes:', error);
    return [];
  }
}

function parseLastModified(val: string | Date): Date {
  return val instanceof Date ? val : new Date(val);
}

export function convertToLocaleDateString(lastModified: string): string {
  if (lastModified) {
    return parseLastModified(lastModified).toLocaleDateString();
  }
  return '';
}

export function getStorageDirectory(isGlobal: boolean): string {
  if (isGlobal) {
    return '~/.config/goose/recipes';
  } else {
    // For directory recipes, build absolute path using working directory
    const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;
    return `${workingDir}/.goose/recipes`;
  }
}
