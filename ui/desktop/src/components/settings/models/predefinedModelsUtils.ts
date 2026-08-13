import Model from './modelInterface';

let curatedModelsCache: Model[] = [];

/** Update in-memory curated models from ACP inventory (server catalog). */
export function setCuratedModels(models: Model[]): void {
  curatedModelsCache = models;
}

export function getCuratedModels(): Model[] {
  if (curatedModelsCache.length > 0) {
    return curatedModelsCache;
  }
  return getPredefinedModelsFromEnv();
}

// Helper functions for predefined models - shared across components
export function getPredefinedModelsFromEnv(): Model[] {
  try {
    const envModels = window.appConfig.get('GOOSE_PREDEFINED_MODELS'); // process.env.GOOSE_PREDEFINED_MODELS
    if (envModels && typeof envModels === 'string') {
      return JSON.parse(envModels) as Model[];
    }
  } catch (error) {
    console.warn('Failed to parse GOOSE_PREDEFINED_MODELS environment variable:', error);
  }
  return [];
}

export function shouldShowPredefinedModels(): boolean {
  return getCuratedModels().length > 0;
}

export function getModelDisplayName(modelName: string): string {
  const models = getCuratedModels();
  const matchingModel = models.find((model) => model.name === modelName);
  return matchingModel?.alias || modelName;
}

export function getProviderDisplayName(modelName: string): string {
  const models = getCuratedModels();
  const matchingModel = models.find((model) => model.name === modelName);
  return matchingModel?.subtext || '';
}
