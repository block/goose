import { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
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
      // Update the recipe config with parameter values
      const enhancedConfig = {
        ...config,
        _paramValues: paramValues,
      };

      // Store the enhanced config in appConfig
      window.appConfig.set('recipeConfig', enhancedConfig);

      // Re-initialize the system with the parameter values
      try {
        const windowConfig = window.electron.getConfig();
        const provider =
          (await read('GOOSE_PROVIDER', false)) ?? windowConfig.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? windowConfig.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          await initializeSystem(provider as string, model as string, {
            getExtensions: async (_: boolean): Promise<FixedExtensionEntry[]> => [],
            addExtension: async (
              _name: string,
              _config: ExtensionConfig,
              _enabled: boolean
            ): Promise<void> => {},
          });
        } else {
          console.error('RecipeParametersView: Missing provider or model configuration');
        }
      } catch (error) {
        console.error(
          'RecipeParametersView: Failed to re-initialize system with parameters:',
          error
        );
      }

      // Add a small delay to ensure the config is saved before redirecting
      setTimeout(() => {
        // Redirect to chat view where the agent will use the parameterized prompt
        onClose();
      }, 100);
    }
  };

  const handleCancel = async () => {
    // When user confirms cancellation, create a recipe config without parameters
    // This makes it behave exactly like a recipe that never had parameters
    if (config) {
      // Create a new config without the parameters field
      const { parameters: _parameters, ...configWithoutParams } = config;

      // Store the config without parameters in appConfig
      window.appConfig.set('recipeConfig', configWithoutParams);

      // Re-initialize the system to start the chat (same flow as recipes without parameters)
      try {
        const windowConfig = window.electron.getConfig();
        const provider =
          (await read('GOOSE_PROVIDER', false)) ?? windowConfig.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? windowConfig.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          await initializeSystem(provider as string, model as string, {
            getExtensions: async (_: boolean): Promise<FixedExtensionEntry[]> => [],
            addExtension: async (
              _name: string,
              _config: ExtensionConfig,
              _enabled: boolean
            ): Promise<void> => {},
          });
        } else {
          console.error('RecipeParametersView: Missing provider or model configuration');
        }
      } catch (error) {
        console.error('RecipeParametersView: Failed to re-initialize system:', error);
      }
    }

    setIsModalOpen(false);
    // Redirect to chat view
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
        onSubmit={handleSubmit}
        onCancel={handleCancel}
        recipeConfig={config}
      />
    </div>
  );
}
