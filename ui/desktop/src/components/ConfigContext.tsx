import React, { createContext, useContext, useState, useEffect, useMemo, useCallback } from 'react';
import { readAllConfig, readConfig, removeConfig, upsertConfig, providers } from '../api';
import {
  getConfiguredExtensions,
  addConfigExtension,
  removeConfigExtension,
  setConfigExtensionEnabled,
} from '../acp/extensions';
import { pruneDeprecatedBundledExtensions, syncBundledExtensions } from './settings/extensions';
import { nameToKey } from './settings/extensions/utils';
import type {
  ConfigResponse,
  UpsertConfigQuery,
  ConfigKeyQuery,
  ProviderDetails,
  ExtensionConfig,
} from '../api';

export type { ExtensionConfig } from '../api/types.gen';

// Define a local version that matches the structure of the imported one
export type FixedExtensionEntry = ExtensionConfig & {
  enabled: boolean;
  configKey?: string;
};

interface ConfigContextType {
  config: ConfigResponse['config'];
  providersList: ProviderDetails[];
  extensionsList: FixedExtensionEntry[];
  extensionWarnings: string[];
  upsert: (key: string, value: unknown, is_secret: boolean) => Promise<void>;
  read: (key: string, is_secret: boolean, options?: { throwOnError?: boolean }) => Promise<unknown>;
  remove: (key: string, is_secret: boolean) => Promise<void>;
  addExtension: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>;
  setExtensionEnabled: (configKey: string, enabled: boolean) => Promise<void>;
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
  const [extensionWarnings, setExtensionWarnings] = useState<string[]>([]);

  // Ref to access providersList in getProviders without recreating the callback
  const providersListRef = React.useRef<ProviderDetails[]>(providersList);
  providersListRef.current = providersList;

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

  const read = useCallback(
    async (key: string, is_secret: boolean = false, options?: { throwOnError?: boolean }) => {
      const query: ConfigKeyQuery = { key: key, is_secret: is_secret };
      const response = await readConfig({
        body: query,
      });
      if (options?.throwOnError && response.error) {
        throw response.error;
      }
      return response.data;
    },
    []
  );

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
    const { extensions, warnings } = await getConfiguredExtensions();
    setExtensionsList(extensions);
    setExtensionWarnings(warnings || []);
    return extensions;
  }, []);

  const addExtension = useCallback(
    async (_name: string, config: ExtensionConfig, enabled: boolean) => {
      await addConfigExtension(config, enabled);
      await reloadConfig();
      // Refresh extensions list after successful addition
      await refreshExtensions();
    },
    [reloadConfig, refreshExtensions]
  );

  const removeExtension = useCallback(
    async (name: string) => {
      const entry = extensionsList.find((ext) => ext.name === name);
      await removeConfigExtension(entry?.configKey ?? nameToKey(name));
      await reloadConfig();
      // Refresh extensions list after successful removal
      await refreshExtensions();
    },
    [extensionsList, reloadConfig, refreshExtensions]
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

  const setExtensionEnabled = useCallback(
    async (configKey: string, enabled: boolean) => {
      await setConfigExtensionEnabled(configKey, enabled);
      await reloadConfig();
      await refreshExtensions();
    },
    [reloadConfig, refreshExtensions]
  );

  const getProviders = useCallback(async (forceRefresh = false): Promise<ProviderDetails[]> => {
    if (forceRefresh || providersListRef.current.length === 0) {
      try {
        const response = await providers();
        const providersData = response.data || [];
        providersListRef.current = providersData;
        setProvidersList(providersData);
        return providersData;
      } catch (error) {
        console.error('Failed to fetch providers:', error);
        return providersListRef.current;
      }
    }
    return providersListRef.current;
  }, []);

  useEffect(() => {
    // Load all configuration data and providers on mount
    (async () => {
      // Load config
      const configResponse = await readAllConfig();
      setConfig(configResponse.data?.config || {});

      // Load providers
      try {
        const providersResponse = await providers();
        const providersData = providersResponse.data || [];
        providersListRef.current = providersData;
        setProvidersList(providersData);
      } catch (error) {
        console.error('Failed to load providers:', error);
        setProvidersList([]);
      }

      // Load extensions
      try {
        const extensionsResponse = await getConfiguredExtensions();
        let extensions = extensionsResponse.extensions;

        // Always sync bundled extensions from bundled-extensions.json
        // This ensures:
        // 1. Fresh installs get the default extensions (developer, computercontroller, etc.)
        // 2. Existing users get NEW bundled extensions added in subsequent releases
        // The syncBundledExtensions function skips extensions that already exist and are marked as bundled
        // Platform extensions (code_execution, todo, etc.) are handled by the backend
        const addExtensionForSync = async (
          _name: string,
          config: ExtensionConfig,
          enabled: boolean
        ) => {
          await addConfigExtension(config, enabled);
        };
        const removeExtensionForSync = async (configKey: string) => {
          await removeConfigExtension(configKey);
        };
        extensions = await pruneDeprecatedBundledExtensions(extensions, removeExtensionForSync);
        await syncBundledExtensions(extensions, addExtensionForSync);
        // Reload extensions after sync
        const refreshedResponse = await getConfiguredExtensions();
        extensions = refreshedResponse.extensions;

        setExtensionsList(extensions);
        setExtensionWarnings(extensionsResponse.warnings || []);
      } catch (error) {
        console.error('Failed to load extensions:', error);
      }
    })();
  }, []);

  const contextValue = useMemo(() => {
    return {
      config,
      providersList,
      extensionsList,
      extensionWarnings,
      upsert,
      read,
      remove,
      addExtension,
      removeExtension,
      setExtensionEnabled,
      getProviders,
      getExtensions,
    };
  }, [
    config,
    providersList,
    extensionsList,
    extensionWarnings,
    upsert,
    read,
    remove,
    addExtension,
    removeExtension,
    setExtensionEnabled,
    getProviders,
    getExtensions,
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
