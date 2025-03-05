import React, { createContext, useContext, useState, useEffect } from 'react';
import {
  readAllConfig,
  readConfig,
  removeConfig,
  upsertConfig,
  addExtension as apiAddExtension,
  removeExtension as apiRemoveExtension,
  updateExtension as apiUpdateExtension,
} from '../api';
import { client } from '../api/client.gen';

// Initialize client configuration
client.setConfig({
  baseUrl: window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT'),
  headers: {
    'Content-Type': 'application/json',
    'X-Secret-Key': window.appConfig.get('secretKey'),
  },
});

interface ConfigContextType {
  config: Record<string, any>;
  upsert: (key: string, value: any, isSecret?: boolean) => Promise<void>;
  read: (key: string) => Promise<any>;
  remove: (key: string) => Promise<void>;
  addExtension: (name: string, config: any) => Promise<void>;
  updateExtension: (name: string, config: any) => Promise<void>;
  removeExtension: (name: string) => Promise<void>;
}

interface ConfigProviderProps {
  children: React.ReactNode;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export const ConfigProvider: React.FC<ConfigProviderProps> = ({ children }) => {
  const [config, setConfig] = useState<Record<string, any>>({});

  useEffect(() => {
    // Load all configuration data on mount
    (async () => {
      const response = await readAllConfig();
      setConfig(response.data.config || {});
    })();
  }, []);

  const reloadConfig = async () => {
    const response = await readAllConfig();
    setConfig(response.data.config || {});
  };

  const upsert = async (key: string, value: any, isSecret?: boolean) => {
    await upsertConfig({
      body: {
        key,
        value,
        is_secret: isSecret,
      },
    });
    await reloadConfig();
  };

  const read = async (key: string) => {
    return await readConfig({
      body: { key },
    });
  };

  const remove = async (key: string) => {
    await removeConfig({
      body: { key },
    });
    await reloadConfig();
  };

  const addExtension = async (name: string, config: any) => {
    await apiAddExtension({
      body: { name, config },
    });
    await reloadConfig();
  };

  const removeExtension = async (name: string) => {
    await apiRemoveExtension({
      body: { key: name },
    });
    await reloadConfig();
  };

  const updateExtension = async (name: string, config: any) => {
    await apiUpdateExtension({
      body: { name, config },
    });
    await reloadConfig();
  };

  return (
    <ConfigContext.Provider
      value={{ config, upsert, read, remove, addExtension, updateExtension, removeExtension }}
    >
      {children}
    </ConfigContext.Provider>
  );
};

export const useConfig = () => {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
};
