import React, { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { RecipeParametersModal } from './RecipeParametersModal';

interface RecipeParametersViewProps {
  config?: Recipe;
  onClose: () => void;
}

export function RecipeParametersView({ config, onClose }: RecipeParametersViewProps) {
  const [isModalOpen, setIsModalOpen] = useState(true);

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

      // Add a small delay to ensure the config is saved before redirecting
      setTimeout(() => {
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
