import React, { createContext, useContext, useState, ReactNode } from 'react';
import { GOOSE_MODEL, GOOSE_PROVIDER } from '../../../env_vars';
import { gooseModels } from './GooseModels'; // Assuming hardcoded models are here

// TODO: API keys
export interface Model {
  id?: number; // Make `id` optional to allow user-defined models
  name: string;
  provider: string;
  lastUsed?: string;
  alias?: string; // optional model display name
  subtext?: string; // goes below model name if not the provider
}

interface ModelContextValue {
  currentModel: Model | null;
  setCurrentModel: (model: Model) => void;
  switchModel: (model: Model) => void; // Add the reusable switch function
}

const ModelContext = createContext<ModelContextValue | undefined>(undefined);

export const ModelProvider = ({ children }: { children: ReactNode }) => {
  const [currentModel, setCurrentModel] = useState<Model | null>(
    JSON.parse(localStorage.getItem(GOOSE_MODEL) || 'null')
  );

  const updateModel = (model: Model) => {
    setCurrentModel(model);
    localStorage.setItem(GOOSE_PROVIDER, model.provider.toLowerCase());
    localStorage.setItem(GOOSE_MODEL, JSON.stringify(model));
  };

  const switchModel = (model: Model) => {
    const newModel = model.id
      ? gooseModels.find((m) => m.id === model.id) || model
      : { id: Date.now(), ...model }; // Assign unique ID for user-defined models
    updateModel(newModel);
  };

  return (
    <ModelContext.Provider value={{ currentModel, setCurrentModel: updateModel, switchModel }}>
      {children}
    </ModelContext.Provider>
  );
};

export const useModel = () => {
  const context = useContext(ModelContext);
  if (!context) throw new Error('useModel must be used within a ModelProvider');
  return context;
};
