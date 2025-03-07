import React, { useState, useEffect } from 'react';
import { useRecentModels } from './RecentModels';
import { useModel } from './ModelContext';
import { useHandleModelSelection } from './utils';
import type { View } from '@/src/App';
import { SettingsViewOptions } from '@/src/components/settings/SettingsView';

export interface Model {
  id?: number; // Make `id` optional to allow user-defined models
  name: string;
  provider: string;
  lastUsed?: string;
}

interface ModelRadioListProps {
  renderItem: (props: {
    model: Model;
    isSelected: boolean;
    onSelect: () => void;
  }) => React.ReactNode;
  className?: string;
}

export function SeeMoreModelsButtons({ setView }: { setView: (view: View) => void }) {
  return (
    <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
      <h2 className="text-xl font-medium text-textStandard">Models</h2>
      <button
        onClick={() => {
          setView('moreModels');
        }}
        className="text-indigo-500 hover:text-indigo-600 text-sm"
      >
        Browse
      </button>
    </div>
  );
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
