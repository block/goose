import React, { useState, useEffect } from 'react';
import { Model } from './ModelContext';
import { useModel } from './ModelContext';
import { useHandleModelSelection } from './utils';
import { useRecentModels } from './RecentModels';

interface ModelRadioListProps {
  renderItem: (props: {
    model: Model;
    isSelected: boolean;
    onSelect: () => void;
  }) => React.ReactNode;
  className?: string;
}

export function ModelRadioList({ renderItem, className = '' }: ModelRadioListProps) {
  const { recentModels } = useRecentModels();
  const { currentModel } = useModel();
  const handleModelSelection = useHandleModelSelection();
  const [selectedModel, setSelectedModel] = useState<string | null>(null);

  useEffect(() => {
    if (currentModel) {
      setSelectedModel(currentModel.name);
    }
  }, [currentModel]);

  const handleRadioChange = async (model: Model) => {
    if (selectedModel === model.name) {
      console.log(`Model "${model.name}" is already active.`);
      return;
    }

    setSelectedModel(model.name);
    await handleModelSelection(model, 'ModelList');
  };

  return (
    <div className={className}>
      {recentModels.map((model) =>
        renderItem({
          model,
          isSelected: selectedModel === model.name,
          onSelect: () => handleRadioChange(model),
        })
      )}
    </div>
  );
}
