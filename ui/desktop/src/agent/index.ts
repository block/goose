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

export async function initializeAgent({ model, provider, recipeConfig, recipeParams }: InitializeAgentProps): Promise<Recipe | undefined> {
  console.log('initializeAgent: Starting with params:', { model, provider, recipeConfig: !!recipeConfig, recipeParams });
  
  const requestBody = {
    provider: provider.toLowerCase().replace(/ /g, '_'),
    model: model,
    recipe_config: recipeConfig ? {
      config: recipeConfig,
      parameters: recipeParams || {}
    } : undefined
  };
  
  console.log('initializeAgent: Request body:', requestBody);
  
  const response = await fetch(getApiUrl('/agent/update_provider'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify(requestBody),
  });

  console.log('initializeAgent: Response status:', response.status, response.statusText);

  if (!response.ok) {
    const errorText = await response.text();
    console.error('initializeAgent: Request failed:', errorText);
    throw new Error(`Failed to initialize agent: ${response.statusText}`);
  }

  const result: UpdateProviderResponse = await response.json();
  console.log('initializeAgent: Response result:', result);
  console.log('initializeAgent: Processed recipe:', result.processed_recipe);
  
  return result.processed_recipe;
}
