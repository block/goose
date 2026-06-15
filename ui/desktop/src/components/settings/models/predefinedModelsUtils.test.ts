import { afterEach, describe, expect, it } from 'vitest';

import {
  getConfiguredDefaultPredefinedModel,
  getPredefinedModelsFromEnv,
  getProviderDisplayName,
} from './predefinedModelsUtils';

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

const PREDEFINED_MODELS = JSON.stringify([
  {
    id: 0,
    name: 'auto',
    provider: 'openai',
    alias: 'Auto',
    subtext: 'TokenPlan',
  },
  {
    id: 1,
    name: 'deepseek-v4-pro',
    provider: 'openai',
    alias: 'DeepSeek V4 Pro',
    subtext: '强推理',
  },
]);

describe('predefinedModelsUtils', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('keeps the configured predefined model order from appConfig', () => {
    mockAppConfig({
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });

    expect(getPredefinedModelsFromEnv().map((model) => model.name)).toEqual([
      'auto',
      'deepseek-v4-pro',
    ]);
  });

  it('returns the configured default predefined model from desktop defaults', () => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'auto',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });

    expect(getConfiguredDefaultPredefinedModel()).toMatchObject({
      name: 'auto',
      provider: 'openai',
      alias: 'Auto',
    });
  });

  it('falls back to the first predefined model when default env is missing', () => {
    mockAppConfig({
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });

    expect(getConfiguredDefaultPredefinedModel()).toMatchObject({
      name: 'auto',
      provider: 'openai',
    });
  });

  it('preserves provider subtext for the Auto option', () => {
    mockAppConfig({
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });

    expect(getProviderDisplayName('auto')).toBe('TokenPlan');
  });
});
