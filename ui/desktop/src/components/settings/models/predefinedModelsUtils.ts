import Model from './modelInterface';

function getAppConfigString(key: string): string | null {
  const value = window.appConfig.get(key);
  return typeof value === 'string' && value.trim() ? value : null;
}

export function getConfiguredDefaultProvider(): string | null {
  return getAppConfigString('GOOSE_DEFAULT_PROVIDER');
}

// Helper functions for predefined models - shared across components
export function getPredefinedModelsFromEnv(): Model[] {
  try {
    const envModels = getAppConfigString('GOOSE_PREDEFINED_MODELS'); // process.env.GOOSE_PREDEFINED_MODELS
    if (envModels && typeof envModels === 'string') {
      return JSON.parse(envModels) as Model[];
    }
  } catch (error) {
    console.warn('Failed to parse GOOSE_PREDEFINED_MODELS environment variable:', error);
  }
  return [];
}

export function shouldShowPredefinedModels(): boolean {
  return getPredefinedModelsFromEnv().length > 0;
}

export function getConfiguredDefaultPredefinedModel(): Model | null {
  const predefinedModels = getPredefinedModelsFromEnv();
  if (predefinedModels.length === 0) {
    return null;
  }

  const defaultProvider = getAppConfigString('GOOSE_DEFAULT_PROVIDER');
  const defaultModel = getAppConfigString('GOOSE_DEFAULT_MODEL');

  if (defaultModel) {
    const exactMatch = predefinedModels.find(
      (model) =>
        model.name === defaultModel && (!defaultProvider || model.provider === defaultProvider)
    );
    if (exactMatch) {
      return exactMatch;
    }

    const modelOnlyMatch = predefinedModels.find((model) => model.name === defaultModel);
    if (modelOnlyMatch) {
      return modelOnlyMatch;
    }
  }

  return predefinedModels[0];
}

export function isSingleProviderCatalogMode(): boolean {
  const defaultProvider = getConfiguredDefaultProvider();
  const predefinedModels = getPredefinedModelsFromEnv();

  return Boolean(
    defaultProvider &&
      predefinedModels.length > 0 &&
      predefinedModels.every((model) => model.provider === defaultProvider)
  );
}

export function getSingleProviderCatalogLabel(): string {
  const defaultProvider = getConfiguredDefaultProvider();
  const predefinedModels = getPredefinedModelsFromEnv();
  const autoModel = predefinedModels.find(
    (model) => model.provider === defaultProvider && model.name === 'auto'
  );

  return autoModel?.subtext || autoModel?.alias || defaultProvider || '';
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
