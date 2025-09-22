import { listRecipes, RecipeManifestResponse } from '../api';
import { Recipe } from './index';
import * as yaml from 'yaml';
import { validateRecipe, getValidationErrorMessages } from './validation';

export interface SaveRecipeOptions {
  name: string;
  title?: string;
  global?: boolean; // true for global (~/.config/goose/recipes/), false for project-specific (.goose/recipes/)
}

export interface SavedRecipe {
  name: string;
  recipe: Recipe;
  isGlobal: boolean;
  lastModified: Date;
  isArchived?: boolean;
  filename: string; // The actual filename used
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
  if (isGlobal) {
    return '~/.config/goose/recipes';
  } else {
    // For directory recipes, build absolute path using working directory
    const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;
    return `${workingDir}/.goose/recipes`;
  }
}

/**
 * Get the file path for a recipe based on its name
 */
function getRecipeFilePath(recipeName: string, isGlobal: boolean): string {
  const dir = getStorageDirectory(isGlobal);
  return `${dir}/${recipeName}.yaml`;
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
  const { name, title, global = true } = options;

  let sanitizedName: string;

  if (title) {
    recipe.title = title.trim();
    sanitizedName = generateRecipeFilename(recipe);
    if (!sanitizedName) {
      throw new Error('Invalid recipe title - cannot generate filename');
    }
  } else {
    // This branch should now be considered deprecated and will be removed once the same functionality
    // is incorporated in CreateRecipeForm
    sanitizedName = sanitizeRecipeName(name);
    if (!sanitizedName) {
      throw new Error('Invalid recipe name');
    }
  }

  const validationResult = validateRecipe(recipe);
  if (!validationResult.success) {
    const errorMessages = getValidationErrorMessages(validationResult.errors);
    throw new Error(`Recipe validation failed: ${errorMessages.join(', ')}`);
  }

  try {
    // Create saved recipe object
    const savedRecipe: SavedRecipe = {
      name: sanitizedName,
      filename: sanitizedName,
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

export async function listSavedRecipes(): Promise<RecipeManifestResponse[]> {
  try {
    const listRecipeResponse = await listRecipes();
    return listRecipeResponse?.data?.recipe_manifest_responses ?? [];
  } catch (error) {
    console.warn('Failed to list saved recipes:', error);
    return [];
  }
}

export function convertToLocaleDateString(lastModified: string): string {
  if (lastModified) {
    return parseLastModified(lastModified).toLocaleDateString();
  }
  return '';
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
