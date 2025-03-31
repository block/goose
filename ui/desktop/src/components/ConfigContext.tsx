import React, { createContext, useContext, useState, useEffect, useMemo } from 'react';
import {
  readAllConfig,
  readConfig,
  removeConfig,
  upsertConfig,
  getExtensions as apiGetExtensions,
  addExtension as apiAddExtension,
  removeExtension as apiRemoveExtension,
  providers,
} from '../api';
import { client } from '../api/client.gen';
import type {
  ConfigResponse,
  UpsertConfigQuery,
  ConfigKeyQuery,
  ExtensionResponse,
  ProviderDetails,
  ExtensionQuery,
  ExtensionConfig,
} from '../api/types.gen';
import { removeShims } from './settings_v2/extensions/utils';

export type { ExtensionConfig } from '../api/types.gen';

// Define a local version that matches the structure of the imported one
export type FixedExtensionEntry = ExtensionConfig & {
  enabled: boolean;
};

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
  extensionsList: FixedExtensionEntry[];
  upsert: (key: string, value: unknown, is_secret: boolean) => Promise<void>;
  read: (key: string, is_secret: boolean) => Promise<unknown>;
  remove: (key: string, is_secret: boolean) => Promise<void>;
  addExtension: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  toggleExtension: (name: string) => Promise<void>;
  removeExtension: (name: string) => Promise<void>;
  getProviders: (b: boolean) => Promise<ProviderDetails[]>;
  getExtensions: (b: boolean) => Promise<FixedExtensionEntry[]>;
}

interface ConfigProviderProps {
  children: React.ReactNode;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export const ConfigProvider: React.FC<ConfigProviderProps> = ({ children }) => {
  const [config, setConfig] = useState<ConfigResponse['config']>({});
  const [providersList, setProvidersList] = useState<ProviderDetails[]>([]);
  const [extensionsList, setExtensionsList] = useState<FixedExtensionEntry[]>([]);

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
        const extensionsResponse = await apiGetExtensions();
        setExtensionsList(extensionsResponse.data.extensions);
      } catch (error) {
        console.error('Failed to load extensions:', error);
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
    // remove shims if present
    if (config.type == 'stdio') {
      config.cmd = removeShims(config.cmd);
    }
    const query: ExtensionQuery = { name, config, enabled };
    await apiAddExtension({
      body: query,
    });
    await reloadConfig();
  };

  const removeExtension = async (name: string) => {
    await apiRemoveExtension({ path: { name: name } });
    await reloadConfig();
  };

  const toggleExtension = async (name: string) => {
    // Get current extensions to find the one we need to toggle
    const exts = await getExtensions(true);
    const extension = exts.find((ext) => ext.name === name);

    if (extension) {
      // Toggle the enabled state and update using addExtension
      await addExtension(name, extension, !extension.enabled);
    }
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

  const getExtensions = async (forceRefresh = false): Promise<FixedExtensionEntry[]> => {
    if (forceRefresh || extensionsList.length === 0) {
      // If a refresh is forced, or we don't have providers yet
      const response = await apiGetExtensions();
      const extensionResponse: ExtensionResponse = response.data;
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
      extensionsList,
      upsert,
      read,
      remove,
      addExtension,
      removeExtension,
      toggleExtension,
      getProviders,
      getExtensions,
    }),
    [
      config,
      providersList,
      extensionsList,
      upsert,
      read,
      remove,
      addExtension,
      removeExtension,
      toggleExtension,
      getProviders,
      getExtensions,
    ]
  );

  return <ConfigContext.Provider value={contextValue}>{children}</ConfigContext.Provider>;
};

export const useConfig = () => {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
};
