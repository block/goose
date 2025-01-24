import React, { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Switch } from '../../ui/switch';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { model_docs_link, goose_models } from './hardcoded_stuff';
import { useModel } from './ModelContext';
import { useHandleModelSelection } from './utils';

// Create a mapping from provider name to href
const providerLinks = model_docs_link.reduce((acc, { name, href }) => {
  acc[name] = href;
  return acc;
}, {});

export function ProviderButtons() {
  const { activeKeys } = useActiveKeys();
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const { currentModel } = useModel();
  const handleModelSelection = useHandleModelSelection();

  // Handle Escape key press
  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setSelectedProvider(null);
      }
    };
    window.addEventListener('keydown', handleEsc);
    return () => window.removeEventListener('keydown', handleEsc);
  }, []);

  // Filter models by provider
  const providerModels = selectedProvider
    ? goose_models.filter((model) => model.provider === selectedProvider)
    : [];

  return (
    <div className="space-y-4">
      <div className="overflow-x-auto">
        <div className="flex items-center gap-2 min-w-min">
          {activeKeys.map((provider) => (
            <Button
              key={provider}
              variant="ghost"
              className={`text-sm whitespace-nowrap shrink-0
                                ${
                                  selectedProvider === provider
                                    ? 'bg-bgSubtle text-textStandard border-borderStandard'
                                    : 'bg-bgApp border-borderSubtle text-textSubtle'
                                }
                                rounded-full shadow-none border`}
              onClick={() => {
                setSelectedProvider(selectedProvider === provider ? null : provider);
              }}
            >
              {provider}
            </Button>
          ))}
        </div>
      </div>

      {/* Models List */}
      {selectedProvider && (
        <div className="mt-6">
          <div>
            {providerModels.map((model) => (
              <div
                key={model.id}
                className="py-2 px-1 cursor-pointer text-gray-600 
                                    dark:text-gray-300 hover:text-gray-900 
                                    dark:hover:text-white transition-colors
                                    flex justify-between items-center"
              >
                <span>{model.name}</span>
                <Switch
                  variant="mono"
                  checked={model.id === currentModel?.id}
                  onCheckedChange={() => handleModelSelection(model, 'ProviderButtons')}
                />
              </div>
            ))}
          </div>

          <a
            href={providerLinks[selectedProvider]}
            target="_blank"
            rel="noopener noreferrer"
            className="text-indigo-500 hover:text-indigo-600 text-sm"
          >
            Browse more {selectedProvider} models
          </a>
        </div>
      )}
    </div>
  );
}
