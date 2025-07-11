import Model from './modelInterface';

// Helper functions for predefined models - shared across components
export function getPredefinedModelsFromEnv(): Model[] {
  // TODO: remove test data and use environment variable
  const models_for_test = [
    {
      id: 1,
      name: 'goose-claude-4-sonnet',
      provider: 'databricks',
      alias: 'claude-4-sonnet (recommended)',
      subtext: 'Anthropic',
    },
    {
      id: 2,
      name: 'goose-claude-3-5-sonnet',
      provider: 'databricks',
      alias: 'claude-3.5-sonnet',
      subtext: 'Anthropic',
    },
    {
      id: 3,
      name: 'goose-claude-4-opus',
      provider: 'databricks',
      alias: 'claude-4-opus',
      subtext: 'Anthropic',
    },
  ];

  try {
    // For testing: use hardcoded models
    const envModels = models_for_test; // TODO: replace with process.env.GOOSE_PREDEFINED_MODELS;
    if (envModels) {
      // When using real environment variable, it will be a JSON string that needs parsing:
      // return JSON.parse(envModels) as Model[];

      // For testing with hardcoded array:
      return envModels as Model[];
    }
  } catch (error) {
    console.warn('Failed to parse GOOSE_PREDEFINED_MODELS environment variable:', error);
  }
  return [];
}

export function shouldShowPredefinedModels(): boolean {
  return true; // process.env.GOOSE_PREDEFINED_MODELS !== undefined;
}

export function getModelDisplayName(modelName: string): string {
  const predefinedModels = getPredefinedModelsFromEnv();
  const matchingModel = predefinedModels.find((model) => model.name === modelName);
  return matchingModel?.alias || modelName;
}

export function getProviderDisplayName(modelName: string): string {
  const predefinedModels = getPredefinedModelsFromEnv();
  const matchingModel = predefinedModels.find((model) => model.name === modelName);
  return matchingModel?.subtext || '';
}
