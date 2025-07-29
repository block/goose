import React, { createContext, useContext, useState, useEffect, useMemo, useCallback } from 'react';
import { initializeAgent } from '../agent';
import { toastError, toastSuccess } from '../toasts';
import Model, { getProviderMetadata } from './settings/models/modelInterface';
import { ProviderMetadata } from '../api';
import { useConfig } from './ConfigContext';
import {
  getModelDisplayName,
  getProviderDisplayName,
} from './settings/models/predefinedModelsUtils';

// titles
export const UNKNOWN_PROVIDER_TITLE = 'Provider name lookup';

// errors
const CHANGE_MODEL_ERROR_TITLE = 'Change failed';
const SWITCH_MODEL_AGENT_ERROR_MSG =
  'Failed to start agent with selected model -- please try again';
const CONFIG_UPDATE_ERROR_MSG = 'Failed to update configuration settings -- please try again';
export const UNKNOWN_PROVIDER_MSG = 'Unknown provider in config -- please inspect your config.yaml';

// success
const CHANGE_MODEL_TOAST_TITLE = 'Model changed';
const SWITCH_MODEL_SUCCESS_MSG = 'Successfully switched models';

interface ModelAndProviderContextType {
  currentModel: string | null;
  currentProvider: string | null;
  changeModel: (model: Model) => Promise<void>;
  getCurrentModelAndProvider: () => Promise<{ model: string; provider: string }>;
  getFallbackModelAndProvider: () => Promise<{ model: string; provider: string }>;
  getCurrentModelAndProviderForDisplay: () => Promise<{ model: string; provider: string }>;
  getCurrentModelDisplayName: () => Promise<string>;
  getCurrentProviderDisplayName: () => Promise<string>; // Gets provider display name from subtext
  refreshCurrentModelAndProvider: () => Promise<void>;
}

interface ModelAndProviderProviderProps {
  children: React.ReactNode;
}

const ModelAndProviderContext = createContext<ModelAndProviderContextType | undefined>(undefined);

export const ModelAndProviderProvider: React.FC<ModelAndProviderProviderProps> = ({ children }) => {
  const [currentModel, setCurrentModel] = useState<string | null>(null);
  const [currentProvider, setCurrentProvider] = useState<string | null>(null);
  const { read, upsert, getProviders, config } = useConfig();

  const changeModel = useCallback(
    async (model: Model) => {
      const modelName = model.name;
      const providerName = model.provider;
      try {
        await initializeAgent({
          model: model.name,
          provider: model.provider,
        });
      } catch (error) {
        console.error(`Failed to change model at agent step -- ${modelName} ${providerName}`);
        toastError({
          title: CHANGE_MODEL_ERROR_TITLE,
          msg: SWITCH_MODEL_AGENT_ERROR_MSG,
          traceback: error instanceof Error ? error.message : String(error),
        });
        // don't write to config
        return;
      }

      try {
        await upsert('GOOSE_PROVIDER', providerName, false);
        await upsert('GOOSE_MODEL', modelName, false);

        // Update local state
        setCurrentProvider(providerName);
        setCurrentModel(modelName);
      } catch (error) {
        console.error(`Failed to change model at config step -- ${modelName} ${providerName}}`);
        toastError({
          title: CHANGE_MODEL_ERROR_TITLE,
          msg: CONFIG_UPDATE_ERROR_MSG,
          traceback: error instanceof Error ? error.message : String(error),
        });
        // agent and config will be out of sync at this point
        // TODO: reset agent to use current config settings
      } finally {
        // show toast
        toastSuccess({
          title: CHANGE_MODEL_TOAST_TITLE,
          msg: `${SWITCH_MODEL_SUCCESS_MSG} -- using ${model.alias ?? modelName} from ${model.subtext ?? providerName}`,
        });
      }
    },
    [upsert]
  );

  const getFallbackModelAndProvider = useCallback(async () => {
    const provider = window.appConfig.get('GOOSE_DEFAULT_PROVIDER') as string;
    const model = window.appConfig.get('GOOSE_DEFAULT_MODEL') as string;
    if (provider && model) {
      try {
        await upsert('GOOSE_MODEL', model, false);
        await upsert('GOOSE_PROVIDER', provider, false);
      } catch (error) {
        console.error('[getFallbackModelAndProvider] Failed to write to config', error);
      }
    }
    return { model: model, provider: provider };
  }, [upsert]);

  const getCurrentModelAndProvider = useCallback(async () => {
    let model: string;
    let provider: string;

    // read from config
    try {
      model = (await read('GOOSE_MODEL', false)) as string;
      provider = (await read('GOOSE_PROVIDER', false)) as string;
    } catch (error) {
      console.error(`Failed to read GOOSE_MODEL or GOOSE_PROVIDER from config`);
      throw error;
    }
    if (!model || !provider) {
      console.log('[getCurrentModelAndProvider] Checking app environment as fallback');
      return getFallbackModelAndProvider();
    }
    return { model: model, provider: provider };
  }, [read, getFallbackModelAndProvider]);

  const getCurrentModelAndProviderForDisplay = useCallback(async () => {
    const modelProvider = await getCurrentModelAndProvider();
    const gooseModel = modelProvider.model;
    const gooseProvider = modelProvider.provider;

    // lookup display name
    let metadata: ProviderMetadata;

    try {
      metadata = await getProviderMetadata(String(gooseProvider), getProviders);
    } catch (error) {
      return { model: gooseModel, provider: gooseProvider };
    }
    const providerDisplayName = metadata.display_name;

    return { model: gooseModel, provider: providerDisplayName };
  }, [getCurrentModelAndProvider, getProviders]);

  const getCurrentModelDisplayName = useCallback(async () => {
    try {
      const currentModelName = (await read('GOOSE_MODEL', false)) as string;
      return getModelDisplayName(currentModelName);
    } catch (error) {
      return 'Select Model';
    }
  }, [read]);

  const getCurrentProviderDisplayName = useCallback(async () => {
    try {
      const currentModelName = (await read('GOOSE_MODEL', false)) as string;
      const providerDisplayName = getProviderDisplayName(currentModelName);
      if (providerDisplayName) {
        return providerDisplayName;
      }
      // Fall back to regular provider display name lookup
      const { provider } = await getCurrentModelAndProviderForDisplay();
      return provider;
    } catch (error) {
      return '';
    }
  }, [read, getCurrentModelAndProviderForDisplay]);

  const refreshCurrentModelAndProvider = useCallback(async () => {
    try {
      const { model, provider } = await getCurrentModelAndProvider();
      setCurrentModel(model);
      setCurrentProvider(provider);
    } catch (error) {
      console.error('Failed to refresh current model and provider:', error);
    }
  }, [getCurrentModelAndProvider]);

  // Load initial model and provider on mount
  useEffect(() => {
    refreshCurrentModelAndProvider();
  }, [refreshCurrentModelAndProvider]);

  // Extract config values for dependency array
  const configObj = config as Record<string, unknown>;
  const gooseModel = configObj?.GOOSE_MODEL;
  const gooseProvider = configObj?.GOOSE_PROVIDER;

  // Listen for config changes and refresh when GOOSE_MODEL or GOOSE_PROVIDER changes
  useEffect(() => {
    // Only refresh if the config has loaded and model/provider values exist
    if (config && Object.keys(config).length > 0 && (gooseModel || gooseProvider)) {
      refreshCurrentModelAndProvider();
    }
  }, [config, gooseModel, gooseProvider, refreshCurrentModelAndProvider]);

  const contextValue = useMemo(
    () => ({
      currentModel,
      currentProvider,
      changeModel,
      getCurrentModelAndProvider,
      getFallbackModelAndProvider,
      getCurrentModelAndProviderForDisplay,
      getCurrentModelDisplayName,
      getCurrentProviderDisplayName,
      refreshCurrentModelAndProvider,
    }),
    [
      currentModel,
      currentProvider,
      changeModel,
      getCurrentModelAndProvider,
      getFallbackModelAndProvider,
      getCurrentModelAndProviderForDisplay,
      getCurrentModelDisplayName,
      getCurrentProviderDisplayName,
      refreshCurrentModelAndProvider,
    ]
  );

  return (
    <ModelAndProviderContext.Provider value={contextValue}>
      {children}
    </ModelAndProviderContext.Provider>
  );
};

export const useModelAndProvider = () => {
  const context = useContext(ModelAndProviderContext);
  if (context === undefined) {
    throw new Error('useModelAndProvider must be used within a ModelAndProviderProvider');
  }
  return context;
};
