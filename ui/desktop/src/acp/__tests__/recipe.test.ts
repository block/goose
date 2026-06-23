import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RecipeDto } from '@aaif/goose-sdk';
import { getAcpClient } from '../acpConnection';
import { encodeRecipe, saveRecipe } from '../recipe';

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
      recipesSave_unstable: vi.fn(),
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
