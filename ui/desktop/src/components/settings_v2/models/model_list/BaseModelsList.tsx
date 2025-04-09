import React, { useEffect, useState } from 'react';
import Model from '../modelInterface';
import { useRecentModels } from './recentModels';
import { changeModel, getCurrentModelAndProvider } from '../index';
import { useConfig } from '../../../ConfigContext';
import { toastInfo } from '../../../../toasts';

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
  const { read, upsert, getExtensions, addExtension } = useConfig();
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Load current model/provider once on component mount
  useEffect(() => {
    let isMounted = true;

    const initializeCurrentModel = async () => {
      try {
        const result = await getCurrentModelAndProvider({
          readFromConfig: read,
          writeToConfig: upsert,
        });
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

    initializeCurrentModel().then();

    return () => {
      isMounted = false;
    };
  }, [read]);

  const handleModelSelection = async (model: Model) => {
    await changeModel({ model: model, writeToConfig: upsert, getExtensions, addExtension });
  };

  const handleRadioChange = async (model: Model) => {
    // Check if the selected model is already active
    if (
      selectedModel &&
      selectedModel.name === model.name &&
      selectedModel.provider === model.provider
    ) {
      toastInfo({
        title: 'No change',
        msg: `Model "${model.name}" is already active.`,
      });
      return;
    }

    // OPTIMISTIC UPDATE: Update the UI immediately
    setSelectedModel(model);

    try {
      // Then perform the actual model change
      await handleModelSelection(model);
    } catch (error) {
      console.error('Error selecting model:', error);

      // If the operation fails, revert to the previous state by simply
      // re-calling the getCurrentModelAndProvider function
      try {
        const result = await getCurrentModelAndProvider({
          readFromConfig: read,
          writeToConfig: upsert,
        });

        const currentModel = modelList.find(
          (m) => m.name === result.model && m.provider === result.provider
        ) || { name: result.model, provider: result.provider };

        setSelectedModel(currentModel);
      } catch (secondError) {
        console.error('Failed to restore previous model:', secondError);
      }

      // Show an error toast
      toastInfo({
        title: 'Error',
        msg: `Failed to switch to model "${model.name}".`,
      });
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
          isSelected:
            selectedModel &&
            selectedModel.name === model.name &&
            selectedModel.provider === model.provider,
          onSelect: () => handleRadioChange(model),
        })
      )}
    </div>
  );
}
