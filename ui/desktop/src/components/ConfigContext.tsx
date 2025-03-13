import React, { createContext, useContext, useState, useEffect, useMemo } from 'react';
import {
  readAllConfig,
  readConfig,
  removeConfig,
  upsertConfig,
  getExtensions as apiGetExtensions,
  toggleExtension as apiToggleExtension,
  addExtension as apiAddExtension,
  removeExtension as apiRemoveExtension,
  updateExtension as apiUpdateExtension,
  providers,
} from '../api';
import { client } from '../api/client.gen';
import type {
  ConfigResponse,
  UpsertConfigQuery,
  ConfigKeyQuery,
  ExtensionResponse,
  ExtensionEntry,
  ProviderDetails,
  ExtensionQuery,
  ExtensionConfig,
} from '../api/types.gen';

// Initialize client configuration
client.setConfig({
  baseUrl: window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT'),
  headers: {
    'Content-Type': 'application/json',
    'X-Secret-Key': window.appConfig.get('secretKey'),
  },
});

interface ConfigContextType {
  config: ConfigResponse['config'];
  providersList: ProviderDetails[];
  upsert: (key: string, value: unknown, is_secret: boolean) => Promise<void>;
  read: (key: string, is_secret: boolean) => Promise<unknown>;
  remove: (key: string, is_secret: boolean) => Promise<void>;
  addExtension: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  updateExtension: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  toggleExtension: (name: string) => Promise<void>;
  removeExtension: (name: string) => Promise<void>;
  getProviders: (b: boolean) => Promise<ProviderDetails[]>;
  getExtensions: (b: boolean) => Promise<ExtensionEntry[]>
}

interface ConfigProviderProps {
  children: React.ReactNode;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export const ConfigProvider: React.FC<ConfigProviderProps> = ({ children }) => {
  const [config, setConfig] = useState<ConfigResponse['config']>({});
  const [providersList, setProvidersList] = useState<ProviderDetails[]>([]);
  const [extensionsList, setExtensionsList] = useState<ExtensionEntry[]>([]);

  useEffect(() => {
    // Load all configuration data and providers on mount
    (async () => {
      // Load config
      const configResponse = await readAllConfig();
      setConfig(configResponse.data.config || {});

      // Load providers
      try {
        const providersResponse = await providers();
        setProvidersList(providersResponse.data);
      } catch (error) {
        console.error('Failed to load providers:', error);
      }

      // Load extensions
      try {
        const extensionsResponse = await apiGetExtensions()
        setExtensionsList(extensionsResponse.data.extensions)
      } catch (error) {
        console.error('Failed to load extensions:', error)
      }

    })();
  }, []);

  const reloadConfig = async () => {
    const response = await readAllConfig();
    setConfig(response.data.config || {});
  };

  const upsert = async (key: string, value: unknown, isSecret: boolean = false) => {
    const query: UpsertConfigQuery = {
      key: key,
      value: value,
      is_secret: isSecret,
    };
    await upsertConfig({
      body: query,
    });
    await reloadConfig();
  };

  const read = async (key: string, is_secret: boolean = false) => {
    const query: ConfigKeyQuery = { key: key, is_secret: is_secret };
    const response = await readConfig({
      body: query,
    });
    return response.data;
  };

  const remove = async (key: string, is_secret: boolean) => {
    const query: ConfigKeyQuery = { key: key, is_secret: is_secret };
    await removeConfig({
      body: query,
    });
    await reloadConfig();
  };

  const addExtension = async (name: string, config: ExtensionConfig, enabled: boolean) => {
    const query: ExtensionQuery = { name, config, enabled };
    await apiAddExtension({
      body: query,
    });
    await reloadConfig();
  };

  const removeExtension = async (name: string) => {
    await apiRemoveExtension({path: {name: name}});
    await reloadConfig();
  };

  const updateExtension = async (name: string, config: ExtensionConfig, enabled: boolean) => {
    const query: ExtensionQuery = { name, config, enabled };
    await apiUpdateExtension({
      body: query,
      path: {name: name}
    });
    await reloadConfig();
  };

  const toggleExtension = async (name: string) => {
    await apiToggleExtension({path: {name: name}});
    await reloadConfig();
  };

  const getProviders = async (forceRefresh = false): Promise<ProviderDetails[]> => {
    if (forceRefresh || providersList.length === 0) {
      // If a refresh is forced or we don't have providers yet
      const response = await providers();
      setProvidersList(response.data);
      return response.data;
    }
    // Otherwise return the cached providers
    return providersList;
  };

  const getExtensions = async (forceRefresh = false): Promise<ExtensionEntry[]> => {
    if (forceRefresh || extensionsList.length === 0) {
      // If a refresh is forced, or we don't have providers yet
      const response = await apiGetExtensions();
      const extensionResponse: ExtensionResponse = response.data
      setExtensionsList(extensionResponse.extensions);
      return extensionResponse.extensions;
    }
    // Otherwise return the cached providers
    return extensionsList;
  };

  const contextValue = useMemo(
    () => ({
      config,
      providersList,
      upsert,
      read,
      remove,
      addExtension,
      updateExtension,
      removeExtension,
      toggleExtension,
      getProviders,
      getExtensions,
    }),
    [config, providersList]
  ); // Functions don't need to be dependencies as they don't change

  return <ConfigContext.Provider value={contextValue}>{children}</ConfigContext.Provider>;
};

export const useConfig = () => {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
};
