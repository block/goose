import React, { useEffect, useState } from 'react';
import Model from '../modelInterface';
import { useRecentModels } from './recentModels';
import { changeModel, getCurrentModelAndProvider } from '../index';
import { useConfig } from '../../../ConfigContext';

interface ModelRadioListProps {
  renderItem: (props: {
    model: Model;
    isSelected: boolean;
    onSelect: () => void;
  }) => React.ReactNode;
  className?: string;
  providedModelList?: Model[];
}

export function BaseModelsList({
  renderItem,
  className = '',
  providedModelList,
}: ModelRadioListProps) {
  const { recentModels } = useRecentModels();

  // allow for a custom model list to be passed if you don't want to use recent models
  let modelList: Model[];
  if (!providedModelList) {
    modelList = recentModels;
  } else {
    modelList = providedModelList;
  }
  const { read, upsert } = useConfig();
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Load current model/provider once on component mount
  useEffect(() => {
    let isMounted = true;

    const initializeCurrentModel = async () => {
      try {
        const result = await getCurrentModelAndProvider({ readFromConfig: read });
        if (isMounted) {
          setSelectedModel(result.model);
          setSelectedProvider(result.provider);
          setIsInitialized(true);
        }
      } catch (error) {
        console.error('Failed to load current model:', error);
        if (isMounted) {
          setIsInitialized(true); // Still mark as initialized even on error
        }
      }
    };

    initializeCurrentModel();

    return () => {
      isMounted = false;
    };
  }, [read]);

  const handleModelSelection = async (modelName: string, providerName: string) => {
    await changeModel({ model: modelName, provider: providerName, writeToConfig: upsert });
  };

  // Updated to work with CustomRadio
  const handleRadioChange = async (model: Model) => {
    if (selectedModel === model.name) {
      console.log(`Model "${model.name}" is already active.`);
      return;
    }

    // Update local state immediately for UI feedback
    setSelectedModel(model.name);
    setSelectedProvider(model.provider);

    try {
      await handleModelSelection(model.name, model.provider);
    } catch (error) {
      console.error('Error selecting model:', error);
    }
  };

  // Don't render until we've loaded the initial model/provider
  if (!isInitialized) {
    return <div>Loading models...</div>;
  }

  return (
    <div className={className}>
      {modelList.map((model) =>
        renderItem({
          model,
          isSelected: selectedModel === model.name,
          onSelect: () => handleRadioChange(model),
        })
      )}
    </div>
  );
}
