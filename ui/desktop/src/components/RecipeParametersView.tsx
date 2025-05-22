import React, { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { type View, ViewOptions } from '../App';
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

  const handleSubmit = (paramValues: Record<string, string>) => {
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
      
      // Redirect to chat view
      onClose();
    }
  };

  const handleClose = () => {
    setIsModalOpen(false);
    // Redirect to chat view even if the user cancels
    onClose();
  };

  if (!config) {
    return null;
  }

  return (
    <div className="flex items-center justify-center h-screen bg-bgApp">
      <RecipeParametersModal
        isOpen={isModalOpen}
        recipeConfig={config}
        onClose={handleClose}
        onSubmit={handleSubmit}
      />
    </div>
  );
} 