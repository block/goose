import type {
  RecipeDto,
  RecipeListEntryDto,
  SaveRecipeResponse_unstable,
  ScanRecipeResponse_unstable,
} from '@aaif/goose-sdk';
import type { Recipe, RecipeManifest } from '../api';
import { getAcpClient } from './acpConnection';

function asAcpRecipe(recipe: Recipe): RecipeDto {
  return recipe as unknown as RecipeDto;
}

function asDesktopRecipe(recipe: RecipeDto): Recipe {
  return recipe as unknown as Recipe;
}

function asDesktopRecipeManifest(entry: RecipeListEntryDto): RecipeManifest {
  return {
    ...entry,
    recipe: asDesktopRecipe(entry.recipe),
  };
}

function acpErrorMessage(error: unknown): string | null {
  if (typeof error !== 'object' || error === null) {
    return null;
  }

  const candidate = 'error' in error && isRecord(error.error) ? error.error : error;
  return isRecord(candidate) && typeof candidate.message === 'string' ? candidate.message : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function normalizeAcpError(error: unknown, fallback: string): Error {
  if (error instanceof Error) {
    return error;
  }
  return new Error(acpErrorMessage(error) ?? fallback);
}

export async function encodeRecipe(recipe: Recipe): Promise<string> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.recipesEncode_unstable({ recipe: asAcpRecipe(recipe) });
    return response.deeplink;
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to encode recipe');
  }
}

export async function decodeRecipe(deeplink: string): Promise<Recipe> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.recipesDecode_unstable({ deeplink });
    return asDesktopRecipe(response.recipe);
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to decode recipe');
  }
}

export async function scanRecipe(recipe: Recipe): Promise<ScanRecipeResponse_unstable> {
  try {
    const client = await getAcpClient();
    return await client.goose.recipesScan_unstable({ recipe: asAcpRecipe(recipe) });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to scan recipe');
  }
}

export async function parseRecipe(content: string): Promise<Recipe> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.recipesParse_unstable({ content });
    return asDesktopRecipe(response.recipe);
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to parse recipe');
  }
}

export async function saveRecipe(
  recipe: Recipe,
  id?: string | null
): Promise<SaveRecipeResponse_unstable> {
  try {
    const client = await getAcpClient();
    return await client.goose.recipesSave_unstable({
      recipe: asAcpRecipe(recipe),
      id,
    });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to save recipe');
  }
}

export async function listRecipes(): Promise<RecipeManifest[]> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.recipesList_unstable({});
    return response.recipes.map(asDesktopRecipeManifest);
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to list recipes');
  }
}

export async function deleteRecipe(id: string): Promise<void> {
  try {
    const client = await getAcpClient();
    await client.goose.recipesDelete_unstable({ id });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to delete recipe');
  }
}

export async function scheduleRecipe(id: string, cronSchedule?: string | null): Promise<void> {
  try {
    const client = await getAcpClient();
    await client.goose.recipesSchedule_unstable({ id, cron_schedule: cronSchedule });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to schedule recipe');
  }
}

export async function setRecipeSlashCommand(
  id: string,
  slashCommand?: string | null
): Promise<void> {
  try {
    const client = await getAcpClient();
    await client.goose.recipesSlashCommand_unstable({ id, slash_command: slashCommand });
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to set recipe slash command');
  }
}

export async function recipeToYaml(recipe: Recipe): Promise<string> {
  try {
    const client = await getAcpClient();
    const response = await client.goose.recipesToYaml_unstable({ recipe: asAcpRecipe(recipe) });
    return response.yaml;
  } catch (error) {
    throw normalizeAcpError(error, 'Failed to convert recipe to YAML');
  }
}
