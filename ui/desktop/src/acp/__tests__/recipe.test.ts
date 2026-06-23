import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RecipeDto } from '@aaif/goose-sdk';
import { getAcpClient } from '../acpConnection';
import {
  decodeRecipe,
  deleteRecipe,
  encodeRecipe,
  listRecipes,
  parseRecipe,
  recipeToYaml,
  saveRecipe,
  scanRecipe,
  scheduleRecipe,
  setRecipeSlashCommand,
} from '../recipe';

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

const recipe = {
  title: 'Test Recipe',
  description: 'A recipe used by ACP tests',
  instructions: 'Follow these test instructions',
} as RecipeDto;

function createClient() {
  return {
    goose: {
      recipesEncode_unstable: vi.fn(),
      recipesDecode_unstable: vi.fn(),
      recipesScan_unstable: vi.fn(),
      recipesParse_unstable: vi.fn(),
      recipesSave_unstable: vi.fn(),
      recipesList_unstable: vi.fn(),
      recipesDelete_unstable: vi.fn(),
      recipesSchedule_unstable: vi.fn(),
      recipesSlashCommand_unstable: vi.fn(),
      recipesToYaml_unstable: vi.fn(),
    },
  };
}

describe('ACP recipe helpers', () => {
  let client: ReturnType<typeof createClient>;

  beforeEach(() => {
    vi.clearAllMocks();
    client = createClient();
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );
  });

  it('encodes a recipe using ACP', async () => {
    client.goose.recipesEncode_unstable.mockResolvedValue({ deeplink: 'encoded' });

    await expect(encodeRecipe(recipe)).resolves.toBe('encoded');

    expect(client.goose.recipesEncode_unstable).toHaveBeenCalledWith({ recipe });
  });

  it('decodes a recipe using ACP', async () => {
    client.goose.recipesDecode_unstable.mockResolvedValue({ recipe });

    await expect(decodeRecipe('encoded')).resolves.toEqual(recipe);

    expect(client.goose.recipesDecode_unstable).toHaveBeenCalledWith({ deeplink: 'encoded' });
  });

  it('scans a recipe using ACP', async () => {
    client.goose.recipesScan_unstable.mockResolvedValue({ has_security_warnings: true });

    await expect(scanRecipe(recipe)).resolves.toEqual({ has_security_warnings: true });

    expect(client.goose.recipesScan_unstable).toHaveBeenCalledWith({ recipe });
  });

  it('parses a recipe using ACP', async () => {
    client.goose.recipesParse_unstable.mockResolvedValue({ recipe });

    await expect(parseRecipe('title: Test')).resolves.toEqual(recipe);

    expect(client.goose.recipesParse_unstable).toHaveBeenCalledWith({ content: 'title: Test' });
  });

  it('saves a recipe using ACP', async () => {
    const response = {
      id: 'recipe-id',
      file_name: 'test.yaml',
      file_path: '/tmp/test.yaml',
    };
    client.goose.recipesSave_unstable.mockResolvedValue(response);

    await expect(saveRecipe(recipe, 'recipe-id')).resolves.toEqual(response);

    expect(client.goose.recipesSave_unstable).toHaveBeenCalledWith({
      recipe,
      id: 'recipe-id',
    });
  });

  it('lists recipes using ACP and returns desktop recipe manifests', async () => {
    client.goose.recipesList_unstable.mockResolvedValue({
      recipes: [
        {
          id: 'recipe-id',
          recipe,
          file_path: '/tmp/test.yaml',
          last_modified: '2026-06-23T00:00:00Z',
          schedule_cron: '0 0 * * * *',
          slash_command: 'test',
        },
      ],
    });

    await expect(listRecipes()).resolves.toEqual([
      {
        id: 'recipe-id',
        recipe,
        file_path: '/tmp/test.yaml',
        last_modified: '2026-06-23T00:00:00Z',
        schedule_cron: '0 0 * * * *',
        slash_command: 'test',
      },
    ]);

    expect(client.goose.recipesList_unstable).toHaveBeenCalledWith({});
  });

  it('runs recipe mutations using ACP', async () => {
    await deleteRecipe('recipe-id');
    await scheduleRecipe('recipe-id', '0 0 * * * *');
    await setRecipeSlashCommand('recipe-id', 'test');

    expect(client.goose.recipesDelete_unstable).toHaveBeenCalledWith({ id: 'recipe-id' });
    expect(client.goose.recipesSchedule_unstable).toHaveBeenCalledWith({
      id: 'recipe-id',
      cron_schedule: '0 0 * * * *',
    });
    expect(client.goose.recipesSlashCommand_unstable).toHaveBeenCalledWith({
      id: 'recipe-id',
      slash_command: 'test',
    });
  });

  it('converts a recipe to YAML using ACP', async () => {
    client.goose.recipesToYaml_unstable.mockResolvedValue({ yaml: 'title: Test Recipe' });

    await expect(recipeToYaml(recipe)).resolves.toBe('title: Test Recipe');

    expect(client.goose.recipesToYaml_unstable).toHaveBeenCalledWith({ recipe });
  });

  it('surfaces ACP JSON-RPC error messages', async () => {
    client.goose.recipesEncode_unstable.mockRejectedValue({
      error: { message: 'recipe is invalid' },
    });

    await expect(encodeRecipe(recipe)).rejects.toThrow('recipe is invalid');
  });

  it('prefers ACP JSON-RPC error data over generic messages', async () => {
    client.goose.recipesSave_unstable.mockRejectedValue({
      error: {
        message: 'Invalid params',
        data: 'save recipe validation failed at recipe.extensions[0]: missing field `cmd`',
      },
    });

    await expect(saveRecipe(recipe)).rejects.toThrow(
      'save recipe validation failed at recipe.extensions[0]: missing field `cmd`'
    );
  });
});
