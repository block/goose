import React, { createContext, useContext, useState, useEffect, useMemo, useCallback } from 'react';
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
import type {
  ConfigResponse,
  UpsertConfigQuery,
  ConfigKeyQuery,
  ExtensionResponse,
  ProviderDetails,
  ExtensionQuery,
  ExtensionConfig,
} from '../api';
import { removeShims } from './settings/extensions/utils';

export type { ExtensionConfig } from '../api/types.gen';

// Define a local version that matches the structure of the imported one
export type FixedExtensionEntry = ExtensionConfig & {
  enabled: boolean;
};

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
  disableAllExtensions: () => Promise<void>;
  enableBotExtensions: (extensions: ExtensionConfig[]) => Promise<void>;
}

interface ConfigProviderProps {
  children: React.ReactNode;
}

export class MalformedConfigError extends Error {
  constructor() {
    super('Check contents of ~/.config/goose/config.yaml');
    this.name = 'MalformedConfigError';
    Object.setPrototypeOf(this, MalformedConfigError.prototype);
  }
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export const ConfigProvider: React.FC<ConfigProviderProps> = ({ children }) => {
  const [config, setConfig] = useState<ConfigResponse['config']>({});
  const [providersList, setProvidersList] = useState<ProviderDetails[]>([]);
  const [extensionsList, setExtensionsList] = useState<FixedExtensionEntry[]>([]);

  const reloadConfig = useCallback(async () => {
    const response = await readAllConfig();
    setConfig(response.data?.config || {});
  }, []);

  const upsert = useCallback(
    async (key: string, value: unknown, isSecret: boolean = false) => {
      const query: UpsertConfigQuery = {
        key: key,
        value: value,
        is_secret: isSecret,
      };
      await upsertConfig({
        body: query,
      });
      await reloadConfig();
    },
    [reloadConfig]
  );

  const read = useCallback(async (key: string, is_secret: boolean = false) => {
    const query: ConfigKeyQuery = { key: key, is_secret: is_secret };
    const response = await readConfig({
      body: query,
    });
    return response.data;
  }, []);

  const remove = useCallback(
    async (key: string, is_secret: boolean) => {
      const query: ConfigKeyQuery = { key: key, is_secret: is_secret };
      await removeConfig({
        body: query,
      });
      await reloadConfig();
    },
    [reloadConfig]
  );

  const refreshExtensions = useCallback(async () => {
    const result = await apiGetExtensions();

    if (result.response.status === 422) {
      throw new MalformedConfigError();
    }

    if (result.error && !result.data) {
      console.log(result.error);
      return extensionsList;
    }

    const extensionResponse: ExtensionResponse = result.data!;
    setExtensionsList(extensionResponse.extensions);
    return extensionResponse.extensions;
  }, [extensionsList]);

  const addExtension = useCallback(
    async (name: string, config: ExtensionConfig, enabled: boolean) => {
      // remove shims if present
      if (config.type === 'stdio') {
        config.cmd = removeShims(config.cmd);
      }
      const query: ExtensionQuery = { name, config, enabled };
      await apiAddExtension({
        body: query,
      });
      await reloadConfig();
      // Refresh extensions list after successful addition
      await refreshExtensions();
    },
    [reloadConfig, refreshExtensions]
  );

  const removeExtension = useCallback(
    async (name: string) => {
      await apiRemoveExtension({ path: { name: name } });
      await reloadConfig();
      // Refresh extensions list after successful removal
      await refreshExtensions();
    },
    [reloadConfig, refreshExtensions]
  );

  const getExtensions = useCallback(
    async (forceRefresh = false): Promise<FixedExtensionEntry[]> => {
      if (forceRefresh || extensionsList.length === 0) {
        return await refreshExtensions();
      }
      return extensionsList;
    },
    [extensionsList, refreshExtensions]
  );

  const toggleExtension = useCallback(
    async (name: string) => {
      const exts = await getExtensions(true);
      const extension = exts.find((ext) => ext.name === name);

      if (extension) {
        await addExtension(name, extension, !extension.enabled);
      }
    },
    [addExtension, getExtensions]
  );

  const getProviders = useCallback(
    async (forceRefresh = false): Promise<ProviderDetails[]> => {
      if (forceRefresh || providersList.length === 0) {
        const response = await providers();
        setProvidersList(response.data || []);
        return response.data || [];
      }
      return providersList;
    },
    [providersList]
  );

  useEffect(() => {
    // Load all configuration data and providers on mount
    (async () => {
      // Load config
      const configResponse = await readAllConfig();
      setConfig(configResponse.data?.config || {});

      // Load providers
      try {
        const providersResponse = await providers();
        setProvidersList(providersResponse.data || []);
      } catch (error) {
        console.error('Failed to load providers:', error);
      }

      // Load extensions
      try {
        const extensionsResponse = await apiGetExtensions();
        setExtensionsList(extensionsResponse.data?.extensions || []);
      } catch (error) {
        console.error('Failed to load extensions:', error);
      }
    })();
  }, []);

  const contextValue = useMemo(() => {
    const disableAllExtensions = async () => {
      const currentExtensions = await getExtensions(true);
      for (const ext of currentExtensions) {
        if (ext.enabled) {
          await addExtension(ext.name, ext, false);
        }
      }
      await reloadConfig();
    };

    const enableBotExtensions = async (extensions: ExtensionConfig[]) => {
      for (const ext of extensions) {
        await addExtension(ext.name, ext, true);
      }
      await reloadConfig();
    };

    return {
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
      disableAllExtensions,
      enableBotExtensions,
    };
  }, [
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
    reloadConfig,
  ]);

  return <ConfigContext.Provider value={contextValue}>{children}</ConfigContext.Provider>;
};

export const useConfig = () => {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
};
