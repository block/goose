import { Recipe, saveRecipeToFile } from '../api';

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
    });
  } catch (error) {
    throw new Error(
      `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}
