import React, { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import Select from 'react-select';
import { Plus } from 'lucide-react';
import { createSelectedModel, useHandleModelSelection } from './utils';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { gooseModels } from './GooseModels';
import { createDarkSelectStyles, darkSelectTheme } from '../../ui/select-styles';

export function AddModelInline() {
  const { activeKeys } = useActiveKeys(); // Access active keys from context

  // Convert active keys to dropdown options
  const providerOptions = activeKeys.map((key) => ({
    value: key.toLowerCase(),
    label: key,
  }));

  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [modelName, setModelName] = useState<string>('');
  const [filteredModels, setFilteredModels] = useState([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const handleModelSelection = useHandleModelSelection();

  // Filter models by selected provider and input text
  useEffect(() => {
    if (!selectedProvider || !modelName) {
      setFilteredModels([]);
      setShowSuggestions(false);
      return;
    }

    const filtered = gooseModels
      .filter(
        (model) =>
          model.provider.toLowerCase() === selectedProvider &&
          model.name.toLowerCase().includes(modelName.toLowerCase())
      )
      .slice(0, 5); // Limit suggestions to top 5
    setFilteredModels(filtered);
    setShowSuggestions(filtered.length > 0);
  }, [modelName, selectedProvider]);

  const handleSubmit = () => {
    if (!selectedProvider || !modelName) {
      console.error('Both provider and model name are required.');
      return;
    }

    // Find the selected model from the filtered models
    const selectedModel = createSelectedModel(selectedProvider, modelName);

    // Trigger the model selection logic
    handleModelSelection(selectedModel, 'AddModelInline');

    // Reset form state
    setSelectedProvider(null); // Clear the provider selection
    setModelName(''); // Clear the model name
    setFilteredModels([]);
    setShowSuggestions(false);
  };

  const handleSelectSuggestion = (suggestion) => {
    setModelName(suggestion.name);
    setShowSuggestions(false); // Hide suggestions after selection
  };

  const handleBlur = () => {
    setTimeout(() => setShowSuggestions(false), 150); // Delay to allow click to register
  };

  return (
    <div className="mb-6">
      <form className="grid grid-cols-[1.5fr_2fr_auto] gap-4 items-center">
        <Select
          options={providerOptions}
          value={providerOptions.find((option) => option.value === selectedProvider) || null}
          onChange={(option) => {
            setSelectedProvider(option?.value || null);
            setModelName(''); // Clear model name when provider changes
            setFilteredModels([]);
          }}
          placeholder="Select provider"
          isClearable
          styles={createDarkSelectStyles('200px')}
          theme={darkSelectTheme}
        />
        <div className="relative" style={{ minWidth: '150px' }}>
          <Input
            type="text"
            placeholder="Model name"
            value={modelName}
            onChange={(e) => setModelName(e.target.value)}
            onBlur={handleBlur}
          />
          {showSuggestions && (
            <div className="absolute z-10 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg">
              {filteredModels.map((model) => (
                <div
                  key={model.id}
                  className="p-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-white"
                  onClick={() => handleSelectSuggestion(model)}
                >
                  {model.name}
                </div>
              ))}
            </div>
          )}
        </div>
        <Button
          type="button"
          className="bg-black text-white hover:bg-black/90"
          onClick={handleSubmit}
        >
          <Plus className="mr-2 h-4 w-4" /> Add Model
        </Button>
      </form>
    </div>
  );
}
