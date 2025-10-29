import { Recipe, saveRecipe as saveRecipeApi, listRecipes, RecipeManifestResponse } from '../api';

export const saveRecipe = async (recipe: Recipe, recipeId?: string | null): Promise<string> => {
  try {
    let response = await saveRecipeApi({
      body: {
        recipe,
        id: recipeId,
      },
      throwOnError: true,
    });
    return response.data.id;
  } catch (error) {
    let error_message = 'unknown error';
    if (typeof error === 'object' && error !== null && 'message' in error) {
      error_message = error.message as string;
    }
    throw new Error(error_message);
  }
};

export const listSavedRecipes = async (): Promise<RecipeManifestResponse[]> => {
  try {
    const listRecipeResponse = await listRecipes();
    return listRecipeResponse?.data?.recipe_manifest_responses ?? [];
  } catch (error) {
    console.warn('Failed to list saved recipes:', error);
    return [];
  }
};

const parseLastModified = (val: string | Date): Date => {
  return val instanceof Date ? val : new Date(val);
};

export const convertToLocaleDateString = (lastModified: string): string => {
  if (lastModified) {
    return parseLastModified(lastModified).toLocaleDateString();
  }
  return '';
};

export const getStorageDirectory = (isGlobal: boolean): string => {
  if (isGlobal) {
    return '~/.config/goose/recipes';
  } else {
    // For directory recipes, build absolute path using working directory
    const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;
    return `${workingDir}/.goose/recipes`;
  }
};
