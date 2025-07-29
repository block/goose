import { Recipe } from './index';
import * as yaml from 'yaml';

export interface SaveRecipeOptions {
  name: string;
  global?: boolean; // true for global (~/.config/goose/recipes/), false for project-specific (.goose/recipes/)
}

export interface SavedRecipe {
  name: string;
  recipe: Recipe;
  isGlobal: boolean;
  lastModified: Date;
  isArchived?: boolean;
}

/**
 * Sanitize a recipe name to be safe for use as a filename
 */
function sanitizeRecipeName(name: string): string {
  return name.replace(/[^a-zA-Z0-9-_\s]/g, '').trim();
}

/**
 * Parse a lastModified value that could be a string or Date
 */
function parseLastModified(val: string | Date): Date {
  return val instanceof Date ? val : new Date(val);
}

/**
 * Get the storage directory path for recipes
 */
export function getStorageDirectory(isGlobal: boolean): string {
  return isGlobal ? '~/.config/goose/recipes' : '.goose/recipes';
}

/**
 * Get the file path for a recipe based on its name
 */
function getRecipeFilePath(recipeName: string, isGlobal: boolean): string {
  const dir = getStorageDirectory(isGlobal);
  return `${dir}/${recipeName}.yaml`;
}

/**
 * Load recipe from file
 */
async function loadRecipeFromFile(
  recipeName: string,
  isGlobal: boolean
): Promise<SavedRecipe | null> {
  const filePath = getRecipeFilePath(recipeName, isGlobal);

  try {
    const result = await window.electron.readFile(filePath);
    if (!result.found || result.error) {
      return null;
    }

    const recipeData = yaml.parse(result.file) as SavedRecipe;

    // Convert lastModified string to Date if needed
    recipeData.lastModified = parseLastModified(recipeData.lastModified);

    return {
      ...recipeData,
      isGlobal: isGlobal,
    };
  } catch (error) {
    console.warn(`Failed to load recipe from ${filePath}:`, error);
    return null;
  }
}

/**
 * Save recipe to file
 */
async function saveRecipeToFile(recipe: SavedRecipe): Promise<boolean> {
  const filePath = getRecipeFilePath(recipe.name, recipe.isGlobal);

  // Ensure directory exists
  const dirPath = getStorageDirectory(recipe.isGlobal);
  await window.electron.ensureDirectory(dirPath);

  // Convert to YAML and save
  const yamlContent = yaml.stringify(recipe);
  return await window.electron.writeFile(filePath, yamlContent);
}
/**
 * Save a recipe to a file using IPC.
 */
export async function saveRecipe(recipe: Recipe, options: SaveRecipeOptions): Promise<string> {
  const { name, global = true } = options;

  // Sanitize name
  const sanitizedName = sanitizeRecipeName(name);
  if (!sanitizedName) {
    throw new Error('Invalid recipe name');
  }

  // Validate recipe has required fields
  if (!recipe.title || !recipe.description) {
    throw new Error('Recipe is missing required fields (title, description)');
  }

  if (!recipe.instructions && !recipe.prompt) {
    throw new Error('Recipe must have either instructions or prompt');
  }

  try {
    // Create saved recipe object
    const savedRecipe: SavedRecipe = {
      name: sanitizedName,
      recipe: recipe,
      isGlobal: global,
      lastModified: new Date(),
      isArchived: false,
    };

    // Save to file
    const success = await saveRecipeToFile(savedRecipe);

    if (!success) {
      throw new Error('Failed to save recipe file');
    }

    // Return identifier for the saved recipe
    return `${global ? 'global' : 'local'}:${sanitizedName}`;
  } catch (error) {
    throw new Error(
      `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Load a recipe by name from file.
 */
export async function loadRecipe(recipeName: string, isGlobal: boolean): Promise<Recipe> {
  try {
    const savedRecipe = await loadRecipeFromFile(recipeName, isGlobal);

    if (!savedRecipe) {
      throw new Error('Recipe not found');
    }

    // Validate the loaded recipe has required fields
    if (!savedRecipe.recipe.title || !savedRecipe.recipe.description) {
      throw new Error('Loaded recipe is missing required fields');
    }

    if (!savedRecipe.recipe.instructions && !savedRecipe.recipe.prompt) {
      throw new Error('Loaded recipe must have either instructions or prompt');
    }

    return savedRecipe.recipe;
  } catch (error) {
    throw new Error(
      `Failed to load recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * List all saved recipes from the recipes directories.
 *
 * Uses the listFiles API to find available recipe files.
 */
export async function listSavedRecipes(includeArchived: boolean = false): Promise<SavedRecipe[]> {
  try {
    // Check for global and local recipe directories
    const globalDir = getStorageDirectory(true);
    const localDir = getStorageDirectory(false);

    // Ensure directories exist
    await window.electron.ensureDirectory(globalDir);
    await window.electron.ensureDirectory(localDir);

    // Get list of recipe files with .yaml extension
    const globalFiles = await window.electron.listFiles(globalDir, 'yaml');
    const localFiles = await window.electron.listFiles(localDir, 'yaml');

    // Process global recipes in parallel
    const globalRecipePromises = globalFiles.map(async (file) => {
      const recipeName = file.replace(/\.yaml$/, '');
      return await loadRecipeFromFile(recipeName, true);
    });

    // Process local recipes in parallel
    const localRecipePromises = localFiles.map(async (file) => {
      const recipeName = file.replace(/\.yaml$/, '');
      return await loadRecipeFromFile(recipeName, false);
    });

    // Wait for all recipes to load in parallel
    const [globalRecipes, localRecipes] = await Promise.all([
      Promise.all(globalRecipePromises),
      Promise.all(localRecipePromises),
    ]);

    // Filter out null results and apply archived filter
    const recipes: SavedRecipe[] = [];

    for (const recipe of globalRecipes) {
      if (recipe && (includeArchived || !recipe.isArchived)) {
        recipes.push(recipe);
      }
    }

    for (const recipe of localRecipes) {
      if (recipe && (includeArchived || !recipe.isArchived)) {
        recipes.push(recipe);
      }
    }

    // Sort by last modified (newest first)
    return recipes.sort((a, b) => b.lastModified.getTime() - a.lastModified.getTime());
  } catch (error) {
    console.warn('Failed to list saved recipes:', error);
    return [];
  }
}

/**
 * Restore an archived recipe.
 *
 * @param recipeName The name of the recipe to restore
 * @param isGlobal Whether the recipe is in global or local storage
 */
export async function restoreRecipe(recipeName: string, isGlobal: boolean): Promise<void> {
  try {
    const savedRecipe = await loadRecipeFromFile(recipeName, isGlobal);

    if (!savedRecipe) {
      throw new Error('Archived recipe not found');
    }

    if (!savedRecipe.isArchived) {
      throw new Error('Recipe is not archived');
    }

    // Mark as not archived
    savedRecipe.isArchived = false;
    savedRecipe.lastModified = new Date();

    // Save back to file
    const success = await saveRecipeToFile(savedRecipe);

    if (!success) {
      throw new Error('Failed to save updated recipe');
    }
  } catch (error) {
    throw new Error(
      `Failed to restore recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Archive a recipe.
 *
 * @param recipeName The name of the recipe to archive
 * @param isGlobal Whether the recipe is in global or local storage
 */
export async function archiveRecipe(recipeName: string, isGlobal: boolean): Promise<void> {
  try {
    const savedRecipe = await loadRecipeFromFile(recipeName, isGlobal);

    if (!savedRecipe) {
      throw new Error('Recipe not found');
    }

    if (savedRecipe.isArchived) {
      throw new Error('Recipe is already archived');
    }

    // Mark as archived
    savedRecipe.isArchived = true;
    savedRecipe.lastModified = new Date();

    // Save back to file
    const success = await saveRecipeToFile(savedRecipe);

    if (!success) {
      throw new Error('Failed to save updated recipe');
    }
  } catch (error) {
    throw new Error(
      `Failed to archive recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Permanently delete a recipe file.
 *
 * @param recipeName The name of the recipe to permanently delete
 * @param isGlobal Whether the recipe is in global or local storage
 */
export async function permanentlyDeleteRecipe(
  recipeName: string,
  isGlobal: boolean
): Promise<void> {
  try {
    // TODO: Implement file deletion when available in the API
    // For now, we'll just mark it as archived as a fallback
    const savedRecipe = await loadRecipeFromFile(recipeName, isGlobal);

    if (!savedRecipe) {
      throw new Error('Recipe not found');
    }

    // Mark as archived with special flag
    savedRecipe.isArchived = true;
    savedRecipe.lastModified = new Date();

    // Save back to file
    const success = await saveRecipeToFile(savedRecipe);

    if (!success) {
      throw new Error('Failed to mark recipe as deleted');
    }
  } catch (error) {
    throw new Error(
      `Failed to delete recipe: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

/**
 * Delete a recipe (archives it by default for backward compatibility).
 *
 * @deprecated Use archiveRecipe instead
 * @param recipeName The name of the recipe to delete/archive
 * @param isGlobal Whether the recipe is in global or local storage
 */
export async function deleteRecipe(recipeName: string, isGlobal: boolean): Promise<void> {
  return archiveRecipe(recipeName, isGlobal);
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
