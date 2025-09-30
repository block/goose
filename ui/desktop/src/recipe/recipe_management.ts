import { parseRecipe, Recipe, saveRecipeToFile } from '../api';

export async function saveRecipe(
  recipe: Recipe,
  isGlobal: boolean | null,
  recipeId: string | null
): Promise<void> {
  try {
    await saveRecipeToFile({
      body: {
        recipe,
        id: recipeId,
        is_global: isGlobal,
      },
      throwOnError: true,
    });
  } catch (error) {
    let error_message = 'unknown error';
    if (typeof error === 'object' && error !== null && 'message' in error) {
      error_message = error.message as string;
    }
    throw new Error(`Failed to save recipe: ${error_message}`);
  }
}

export async function parseRecipeFromFile(fileContent: string): Promise<Recipe> {
  let response = await parseRecipe({
    body: {
      content: fileContent,
    },
    throwOnError: true,
  });
  return response.data.recipe;
}
