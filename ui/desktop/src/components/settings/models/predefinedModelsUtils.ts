import Model from './modelInterface';

// Helper functions for predefined models - shared across components
export function getPredefinedModelsFromEnv(): Model[] {
  try {
    // For testing: use hardcoded models
    const envModels = process.env.GOOSE_PREDEFINED_MODELS;
    if (envModels) {
      // When using real environment variable, it will be a JSON string that needs parsing:
      return JSON.parse(envModels) as Model[];
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
