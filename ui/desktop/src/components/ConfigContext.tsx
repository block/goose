import React, { createContext, useContext, useState, useEffect } from 'react';
import { Config } from '../api/config';

interface ConfigContextType {
  config: Record<string, any>;
  upsert: (key: string, value: any, isSecret?: boolean) => Promise<void>;
  read: (key: string) => Promise<any>;
  remove: (key: string) => Promise<void>;
  addExtension: (name: string, config: any) => Promise<void>;
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
      const initialConfig = await Config.readAll();
      setConfig(initialConfig || {});
    })();
  }, []);

  const reloadConfig = async () => {
    const newConfig = await Config.readAll();
    setConfig(newConfig || {});
  };

  const upsert = async (key: string, value: any, isSecret?: boolean) => {
    await Config.upsert(key, value, isSecret);
    await reloadConfig();
  };

  const read = async (key: string) => {
    return Config.read(key);
  };

  const remove = async (key: string) => {
    await Config.remove(key);
    await reloadConfig();
  };

  const addExtension = async (name: string, config: any) => {
    await Config.addExtension(name, config);
    await reloadConfig();
  };

  const removeExtension = async (name: string) => {
    await Config.removeExtension(name);
    await reloadConfig();
  };

  return (
    <ConfigContext.Provider value={{ config, upsert, read, remove, addExtension, removeExtension }}>
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
