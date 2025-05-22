import React, { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { type View, ViewOptions } from '../App';
import { RecipeParametersModal } from './RecipeParametersModal';
import { initializeSystem } from '../utils/providerUtils';
import { useConfig } from './ConfigContext';
import type { ExtensionConfig, FixedExtensionEntry } from './ConfigContext';

interface RecipeParametersViewProps {
  config?: Recipe;
  onClose: () => void;
}

export function RecipeParametersView({ config, onClose }: RecipeParametersViewProps) {
  const [isModalOpen, setIsModalOpen] = useState(true);
  const { read } = useConfig();
  
  // If no config or no parameters, redirect to the chat view
  useEffect(() => {
    if (!config || !config.parameters || config.parameters.length === 0) {
      onClose();
    }
  }, [config, onClose]);

  const handleSubmit = async (paramValues: Record<string, string>) => {
    if (config) {
      // Log the collected parameter values
      console.log('Recipe parameters collected:', paramValues);
      
      // Update the recipe config with parameter values
      const enhancedConfig = {
        ...config,
        _paramValues: paramValues
      };
      
      // Log the enhanced config for debugging
      console.log('Storing enhanced recipe config:', enhancedConfig);
      
      // Store the enhanced config in appConfig
      window.appConfig.set('recipeConfig', enhancedConfig);
      
      // Re-initialize the system with the parameter values
      try {
        const windowConfig = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? windowConfig.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? windowConfig.GOOSE_DEFAULT_MODEL;
        
        if (provider && model) {
          console.log('Re-initializing system with recipe parameters...');
          await initializeSystem(provider, model, {
            getExtensions: async (): Promise<FixedExtensionEntry[]> => [],
            addExtension: async (_name: string, _config: ExtensionConfig, _enabled: boolean): Promise<void> => {}
          });
          console.log('System re-initialized successfully with parameters');
        } else {
          console.error('Missing provider or model configuration');
        }
      } catch (error) {
        console.error('Failed to re-initialize system with parameters:', error);
      }
      
      // Add a small delay to ensure the config is saved before redirecting
      setTimeout(() => {
        console.log('Redirecting to chat view...');
        // Redirect to chat view where the agent will use the parameterized prompt
        onClose();
      }, 100);
    }
  };

  const handleClose = () => {
    setIsModalOpen(false);
    // Redirect to chat view even if the user cancels
    onClose();
  };

  // If no config, don't render anything
  if (!config) {
    return null;
  }

  return (
    <div className="h-screen bg-gray-50">
      <RecipeParametersModal
        isOpen={isModalOpen}
        onClose={handleClose}
        onSubmit={handleSubmit}
        recipeConfig={config}
      />
    </div>
  );
} 