import { getApiUrl, getSecretKey } from '../config';
import { Recipe } from '../recipe';

interface InitializeAgentProps {
  model: string;
  provider: string;
  recipeConfig?: Recipe;
  recipeParams?: Record<string, string>;
}

export async function initializeAgent({ model, provider, recipeConfig, recipeParams }: InitializeAgentProps) {
  const response = await fetch(getApiUrl('/agent/update_provider'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model: model,
      recipe_config: recipeConfig ? {
        config: recipeConfig,
        parameters: recipeParams || {}
      } : undefined
    }),
  });
  return response;
}
