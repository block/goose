import React, { useState, useEffect } from 'react';
import { Clock } from 'lucide-react';
import { Model } from './ModelContext';
import { ModelRadioList, SeeMoreModelsButtons } from './ModelRadioList';
import { useModel } from './ModelContext';
import { useHandleModelSelection } from './utils';
import type { View } from '../../../App';

const MAX_RECENT_MODELS = 3;

export function useRecentModels() {
  const [recentModels, setRecentModels] = useState<Model[]>([]);

  useEffect(() => {
    const storedModels = localStorage.getItem('recentModels');
    if (storedModels) {
      setRecentModels(JSON.parse(storedModels));
    }
  }, []);

  const addRecentModel = (model: Model) => {
    const modelWithTimestamp = { ...model, lastUsed: new Date().toISOString() }; // Add lastUsed field
    setRecentModels((prevModels) => {
      const updatedModels = [
        modelWithTimestamp,
        ...prevModels.filter((m) => m.name !== model.name),
      ].slice(0, MAX_RECENT_MODELS);

      localStorage.setItem('recentModels', JSON.stringify(updatedModels));
      return updatedModels;
    });
  };

  return { recentModels, addRecentModel };
}

function getRelativeTimeString(date: string | Date): string {
  const now = new Date();
  const then = new Date(date);
  const diffInSeconds = Math.floor((now.getTime() - then.getTime()) / 1000);

  if (diffInSeconds < 60) {
    return 'Just now';
  }

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) {
    return `${diffInMinutes}m ago`;
  }

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) {
    return `${diffInHours}h ago`;
  }

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 7) {
    return `${diffInDays}d ago`;
  }

  if (diffInDays < 30) {
    const weeks = Math.floor(diffInDays / 7);
    return `${weeks}w ago`;
  }

  const months = Math.floor(diffInDays / 30);
  if (months < 12) {
    return `${months}mo ago`;
  }

  const years = Math.floor(months / 12);
  return `${years}y ago`;
}

export function RecentModels() {
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
    await handleModelSelection(model, 'RecentModels');
  };

  return (
    <div className="space-y-2">
      {recentModels.map((model) => (
        <label key={model.name} className="flex justify-between items-center py-2 cursor-pointer">
          <div className="relative" onClick={() => handleRadioChange(model)}>
            <div className="flex flex-row items-center">
              <input
                type="radio"
                name="recentModels"
                value={model.name}
                checked={selectedModel === model.name}
                onChange={() => handleRadioChange(model)}
                className="peer sr-only"
              />
              <div
                className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500 mr-4
                                peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                                peer-checked:bg-white dark:peer-checked:bg-black
                                transition-all duration-200 ease-in-out"
              ></div>
              <div className="">
                <p className="text-sm text-textStandard">{model.name}</p>
                <p className="text-xs text-textSubtle">{model.provider}</p>
              </div>
            </div>
          </div>
          <div className="flex items-center text-sm text-textSubtle">
            <Clock className="w-4 h-4 mr-2" />
            {model.lastUsed ? getRelativeTimeString(model.lastUsed) : 'N/A'}
          </div>
        </label>
      ))}
    </div>
  );
}

export function RecentModelsRadio({ setView }: { setView: (view: View) => void }) {
  return (
    <div>
      <SeeMoreModelsButtons setView={setView} />
      <div className="px-8">
        <div className="space-y-2">
          <ModelRadioList
            renderItem={({ model, isSelected, onSelect }) => (
              <label key={model.name} className="flex items-center py-2 cursor-pointer">
                <div className="relative mr-4">
                  <input
                    type="radio"
                    name="recentModels"
                    value={model.name}
                    checked={isSelected}
                    onChange={onSelect}
                    className="peer sr-only"
                  />
                  <div
                    className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                                peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                                peer-checked:bg-white dark:peer-checked:bg-black
                                transition-all duration-200 ease-in-out"
                  ></div>
                </div>

                <div className="">
                  <p className="text-sm text-textStandard">{model.alias ?? model.name}</p>
                  <p className="text-xs text-textSubtle">{model.subtext ?? model.provider}</p>
                </div>
              </label>
            )}
          />
        </div>
      </div>
    </div>
  );
}
