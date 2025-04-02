import React, { useEffect, useState } from 'react';
import Model, { getProviderMetadata } from '../modelInterface';
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

// renders a model list and handles changing models when user clicks on them
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
  const { read, upsert, getProviders } = useConfig();
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Load current model/provider once on component mount
  useEffect(() => {
    let isMounted = true;

    const initializeCurrentModel = async () => {
      try {
        const result = await getCurrentModelAndProvider({ readFromConfig: read });
        if (isMounted) {
          // try to look up the model in the modelList
          let currentModel: Model;
          const match = modelList.find(
            (model) => model.name == result.model && model.provider == result.provider
          );
          // no matches so just create a model object (maybe user updated config.yaml from CLI usage, manual editing etc)
          if (!match) {
            currentModel = { name: result.model, provider: result.provider };
          } else {
            currentModel = match;
          }
          setSelectedModel(currentModel);
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
    await changeModel({ model: selectedModel, writeToConfig: upsert });
  };

  // Updated to work with CustomRadio
  const handleRadioChange = async (model: Model) => {
    if (selectedModel.name === model.name && selectedModel.provider === model.provider) {
      console.log(`Model "${model.name}" is already active.`);
      return;
    }

    const providerMetaData = await getProviderMetadata(model.provider, getProviders);
    const providerDisplayName = providerMetaData.display_name;

    // Update local state immediately for UI feedback and add in display name
    setSelectedModel({ ...model, alias: providerDisplayName });

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
          isSelected: selectedModel === model,
          onSelect: () => handleRadioChange(model),
        })
      )}
    </div>
  );
}
