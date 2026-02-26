import type { Recipe, RecipeManifest } from '@/api';
import { listRecipes, saveRecipe as saveRecipeApi } from '@/api';

export const saveRecipe = async (recipe: Recipe, recipeId?: string | null): Promise<string> => {
  try {
    const response = await saveRecipeApi({
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

export const listSavedRecipes = async (): Promise<RecipeManifest[]> => {
  const listRecipeResponse = await listRecipes();
  return listRecipeResponse?.data?.manifests ?? [];
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
