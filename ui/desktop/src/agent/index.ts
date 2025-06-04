import { getApiUrl, getSecretKey } from '../config';
import { Recipe } from '../recipe';

interface InitializeAgentProps {
  model: string;
  provider: string;
  recipeConfig?: Recipe;
  recipeParams?: Record<string, string>;
}

interface UpdateProviderResponse {
  success: boolean;
  processed_recipe?: Recipe;
}

export async function initializeAgent({
  model,
  provider,
  recipeConfig,
  recipeParams,
}: InitializeAgentProps): Promise<Recipe | undefined> {
  const requestBody = {
    provider: provider.toLowerCase().replace(/ /g, '_'),
    model: model,
    recipe_config: recipeConfig
      ? {
          config: recipeConfig,
          parameters: recipeParams || {},
        }
      : undefined,
  };

  const response = await fetch(getApiUrl('/agent/update_provider'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify(requestBody),
  });

  if (!response.ok) {
    const errorText = await response.text();
    console.error('initializeAgent: Request failed:', errorText);
    throw new Error(`Failed to initialize agent: ${response.statusText}`);
  }

  const result: UpdateProviderResponse = await response.json();
  return result.processed_recipe;
}
