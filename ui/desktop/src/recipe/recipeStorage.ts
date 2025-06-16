import { Recipe } from './index';

export interface SaveRecipeOptions {
  name: string;
  global?: boolean; // true for global (~/.config/goose/recipes/), false for project-specific (.goose/recipes/)
}

export interface SavedRecipe {
  name: string;
  path: string;
  recipe: Recipe;
  isGlobal: boolean;
  lastModified: Date;
}

/**
 * Save a recipe to local storage.
 *
 * IMPORTANT: This is a temporary implementation using localStorage.
 * In a production system, this should save actual YAML files to the filesystem.
 * The returned path is a mock path for UI purposes only.
 *
 * TODO: Integrate with the Rust backend recipe system (or local filesystem) to save actual files.
 */
export async function saveRecipe(recipe: Recipe, options: SaveRecipeOptions): Promise<string> {
  const { name, global = true } = options;

  // Sanitize filename
  const sanitizedName = name.replace(/[^a-zA-Z0-9-_\s]/g, '').trim();
  if (!sanitizedName) {
    throw new Error('Invalid recipe name');
  }

  // Validate recipe has required fields
  if (!recipe.title || !recipe.description || !recipe.instructions) {
    throw new Error('Recipe is missing required fields (title, description, instructions)');
  }

  const recipeData = {
    name: sanitizedName,
    recipe: recipe,
    savedAt: new Date().toISOString(),
    isGlobal: global,
  };

  try {
    // Store in localStorage with a unique key
    const storageKey = `goose_saved_recipe_${sanitizedName}_${global ? 'global' : 'local'}`;
    localStorage.setItem(storageKey, JSON.stringify(recipeData));

    // Return a mock path that represents where the file would be saved
    const mockPath = global
      ? `~/.config/goose/recipes/${sanitizedName}.yaml`
      : `.goose/recipes/${sanitizedName}.yaml`;

    return mockPath;
  } catch (error) {
    throw new Error(
      `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Load a recipe from storage (not yet implemented for file system)
 */
export async function loadRecipe(_filePath: string): Promise<Recipe> {
  // TODO: Implement file system loading when integrated with Rust backend
  throw new Error('Recipe loading from files not yet implemented');
}

/**
 * List all saved recipes from localStorage.
 *
 * IMPORTANT: This reads from localStorage, not actual files.
 * TODO: Integrate with filesystem when Rust backend is available.
 */
export async function listSavedRecipes(): Promise<SavedRecipe[]> {
  const recipes: SavedRecipe[] = [];

  try {
    // Get all saved recipes from localStorage
    const keys = Object.keys(localStorage).filter((key) => key.startsWith('goose_saved_recipe_'));

    for (const key of keys) {
      try {
        const data = JSON.parse(localStorage.getItem(key) || '{}');
        if (data.recipe && data.name && data.savedAt) {
          recipes.push({
            name: data.name,
            path: key, // Use the localStorage key as identifier
            recipe: data.recipe,
            isGlobal: data.isGlobal,
            lastModified: new Date(data.savedAt),
          });
        }
      } catch (error) {
        console.warn(`Failed to parse saved recipe ${key}:`, error);
        // Continue processing other recipes instead of failing completely
      }
    }
  } catch (error) {
    console.warn('Failed to load saved recipes:', error);
  }

  // Sort by last modified (newest first)
  return recipes.sort((a, b) => b.lastModified.getTime() - a.lastModified.getTime());
}

/**
 * Delete a saved recipe from localStorage.
 *
 * @param storageKey The localStorage key (not a file path)
 */
export async function deleteRecipe(storageKey: string): Promise<void> {
  try {
    if (!storageKey.startsWith('goose_saved_recipe_')) {
      throw new Error('Invalid storage key');
    }
    localStorage.removeItem(storageKey);
  } catch (error) {
    throw new Error(
      `Failed to delete recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Generate a suggested filename for a recipe based on its title.
 *
 * @param recipe The recipe to generate a filename for
 * @returns A sanitized filename suitable for use as a recipe name
 */
export function generateRecipeFilename(recipe: Recipe): string {
  const baseName = recipe.title
    .toLowerCase()
    .replace(/[^a-zA-Z0-9\s-]/g, '')
    .replace(/\s+/g, '-')
    .trim();

  return baseName || 'untitled-recipe';
}
