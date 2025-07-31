import { Sliders, ChefHat, Bot, Eye, Save } from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { AddModelModal } from '../subcomponents/AddModelModal';
import { LeadWorkerSettings } from '../subcomponents/LeadWorkerSettings';
import { View } from '../../../../App';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '../../../ui/dropdown-menu';
import { useCurrentModelInfo } from '../../../BaseChat';
import { useConfig } from '../../../ConfigContext';
import { getProviderMetadata } from '../modelInterface';
import { Alert } from '../../../alerts';
import BottomMenuAlertPopover from '../../../bottom_menu/BottomMenuAlertPopover';
import { Recipe } from '../../../../recipe';
import { saveRecipe, generateRecipeFilename } from '../../../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../../../toasts';
import ViewRecipeModal from '../../../ViewRecipeModal';

interface ModelsBottomBarProps {
  dropdownRef: React.RefObject<HTMLDivElement>;
  setView: (view: View) => void;
  alerts: Alert[];
  recipeConfig?: Recipe | null;
  hasMessages?: boolean; // Add prop to know if there are messages to create a recipe from
}
export default function ModelsBottomBar({
  dropdownRef,
  setView,
  alerts,
  recipeConfig,
  hasMessages = false,
}: ModelsBottomBarProps) {
  const {
    currentModel,
    currentProvider,
    getCurrentModelAndProviderForDisplay,
    getCurrentModelDisplayName,
    getCurrentProviderDisplayName,
  } = useModelAndProvider();
  const currentModelInfo = useCurrentModelInfo();
  const { read, getProviders } = useConfig();
  const [displayProvider, setDisplayProvider] = useState<string | null>(null);
  const [displayModelName, setDisplayModelName] = useState<string>('Select Model');
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const [isLeadWorkerModalOpen, setIsLeadWorkerModalOpen] = useState(false);
  const [isLeadWorkerActive, setIsLeadWorkerActive] = useState(false);
  const [providerDefaultModel, setProviderDefaultModel] = useState<string | null>(null);

  // Save recipe dialog state (like in RecipeEditor.tsx)
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveRecipeName, setSaveRecipeName] = useState('');
  const [saveGlobal, setSaveGlobal] = useState(true);
  const [saving, setSaving] = useState(false);

  // View recipe modal state
  const [showViewRecipeModal, setShowViewRecipeModal] = useState(false);

  // Check if lead/worker mode is active
  useEffect(() => {
    const checkLeadWorker = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        setIsLeadWorkerActive(!!leadModel);
      } catch (error) {
        console.error('Error checking lead model:', error);
        setIsLeadWorkerActive(false);
      }
    };
    checkLeadWorker();
  }, [read]);

  // Refresh lead/worker status when modal closes
  const handleLeadWorkerModalClose = () => {
    setIsLeadWorkerModalOpen(false);
    // Refresh the lead/worker status after modal closes
    const checkLeadWorker = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        const currentModel = await read('GOOSE_MODEL', false);
        setIsLeadWorkerActive(!!leadModel);
        setLeadModelName((leadModel as string) || '');
        setCurrentActiveModel((currentModel as string) || '');
      } catch (error) {
        console.error('Error checking lead model after modal close:', error);
        setIsLeadWorkerActive(false);
      }
    };
    checkLeadWorker();
  };

  // Since currentModelInfo.mode is not working, let's determine mode differently
  // We'll need to get the lead model and compare it with the current model
  const [leadModelName, setLeadModelName] = useState<string>('');
  const [currentActiveModel, setCurrentActiveModel] = useState<string>('');

  // Get lead model name and current model for comparison
  useEffect(() => {
    const getModelInfo = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        const currentModel = await read('GOOSE_MODEL', false);
        setLeadModelName((leadModel as string) || '');
        setCurrentActiveModel((currentModel as string) || '');
      } catch (error) {
        console.error('Error getting model info:', error);
      }
    };
    getModelInfo();
  }, [read]);

  // Determine the mode based on which model is currently active
  const modelMode = isLeadWorkerActive
    ? currentActiveModel === leadModelName
      ? 'lead'
      : 'worker'
    : undefined;

  // Determine which model to display - activeModel takes priority when lead/worker is active
  const displayModel =
    isLeadWorkerActive && currentModelInfo?.model
      ? currentModelInfo.model
      : currentModel || providerDefaultModel || displayModelName;

  // Update display provider when current provider changes
  useEffect(() => {
    if (currentProvider) {
      (async () => {
        const providerDisplayName = await getCurrentProviderDisplayName();
        if (providerDisplayName) {
          setDisplayProvider(providerDisplayName);
        } else {
          const modelProvider = await getCurrentModelAndProviderForDisplay();
          setDisplayProvider(modelProvider.provider);
        }
      })();
    }
  }, [currentProvider, getCurrentProviderDisplayName, getCurrentModelAndProviderForDisplay]);

  // Fetch provider default model when provider changes and no current model
  useEffect(() => {
    if (currentProvider && !currentModel) {
      (async () => {
        try {
          const metadata = await getProviderMetadata(currentProvider, getProviders);
          setProviderDefaultModel(metadata.default_model);
        } catch (error) {
          console.error('Failed to get provider default model:', error);
          setProviderDefaultModel(null);
        }
      })();
    } else if (currentModel) {
      // Clear provider default when we have a current model
      setProviderDefaultModel(null);
    }
  }, [currentProvider, currentModel, getProviders]);

  // Update display model name when current model changes
  useEffect(() => {
    (async () => {
      const displayName = await getCurrentModelDisplayName();
      setDisplayModelName(displayName);
    })();
  }, [currentModel, getCurrentModelDisplayName]);

  // Handle view recipe - open modal instead of navigating
  const handleViewRecipe = () => {
    if (recipeConfig) {
      setShowViewRecipeModal(true);
    }
  };

  // Handle save recipe - show save dialog (like in RecipeEditor.tsx)
  const handleSaveRecipeClick = () => {
    if (recipeConfig) {
      const suggestedName = generateRecipeFilename(recipeConfig);
      setSaveRecipeName(suggestedName);
      setShowSaveDialog(true);
    }
  };

  // Handle save recipe (like in RecipeEditor.tsx)
  const handleSaveRecipe = async () => {
    if (!saveRecipeName.trim() || !recipeConfig) {
      return;
    }

    setSaving(true);
    try {
      if (!recipeConfig.title || !recipeConfig.description || !recipeConfig.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      await saveRecipe(recipeConfig, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      // Reset dialog state
      setShowSaveDialog(false);
      setSaveRecipeName('');

      toastSuccess({
        title: saveRecipeName.trim(),
        msg: `Recipe saved successfully`,
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="relative flex items-center" ref={dropdownRef}>
      <BottomMenuAlertPopover alerts={alerts} />
      <DropdownMenu>
        <DropdownMenuTrigger className="flex items-center hover:cursor-pointer max-w-[180px] md:max-w-[200px] lg:max-w-[380px] min-w-0 text-text-default/70 hover:text-text-default transition-colors">
          <div className="flex items-center truncate max-w-[130px] md:max-w-[200px] lg:max-w-[360px] min-w-0">
            <Bot className="mr-1 h-4 w-4 flex-shrink-0" />
            <span className="truncate text-xs">
              {displayModel}
              {isLeadWorkerActive && modelMode && (
                <span className="ml-1 text-[10px] opacity-60">({modelMode})</span>
              )}
            </span>
          </div>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align="center" className="w-64 text-sm">
          <h6 className="text-xs text-textProminent mt-2 ml-2">Current model</h6>
          <p className="flex items-center justify-between text-sm mx-2 pb-2 border-b mb-2">
            {displayModelName}
            {displayProvider && ` — ${displayProvider}`}
          </p>
          <DropdownMenuItem onClick={() => setIsAddModelModalOpen(true)}>
            <span>Change Model</span>
            <Sliders className="ml-auto h-4 w-4 rotate-90" />
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setIsLeadWorkerModalOpen(true)}>
            <span>Lead/Worker Settings</span>
            <Sliders className="ml-auto h-4 w-4" />
          </DropdownMenuItem>

          {/* Recipe-specific menu items - only show when actively using a recipe */}
          {recipeConfig && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={handleViewRecipe}>
                <span>View/Edit Recipe</span>
                <Eye className="ml-auto h-4 w-4" />
              </DropdownMenuItem>
              <DropdownMenuItem onClick={handleSaveRecipeClick}>
                <span>Save Recipe</span>
                <Save className="ml-auto h-4 w-4" />
              </DropdownMenuItem>
            </>
          )}

          <DropdownMenuSeparator />
          {/* Only show "Create a recipe from this session" when there are messages to create from */}
          {hasMessages && (
            <DropdownMenuItem
              onClick={() => {
                // Signal to create an agent from the current chat
                window.dispatchEvent(new CustomEvent('make-agent-from-chat'));
              }}
            >
              <span>Create a recipe from this session</span>
              <ChefHat className="ml-auto h-4 w-4" />
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}

      {isLeadWorkerModalOpen ? (
        <LeadWorkerSettings isOpen={isLeadWorkerModalOpen} onClose={handleLeadWorkerModalClose} />
      ) : null}

      {/* Save Recipe Dialog - copied from RecipeEditor.tsx */}
      {showSaveDialog && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-borderSubtle rounded-lg p-6 w-96 max-w-[90vw]">
            <h3 className="text-lg font-medium text-textProminent mb-4">Save Recipe</h3>

            <div className="space-y-4">
              <div>
                <label
                  htmlFor="recipe-name"
                  className="block text-sm font-medium text-textStandard mb-2"
                >
                  Recipe Name
                </label>
                <input
                  id="recipe-name"
                  type="text"
                  value={saveRecipeName}
                  onChange={(e) => setSaveRecipeName(e.target.value)}
                  className="w-full p-3 border border-borderSubtle rounded-lg bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
                  placeholder="Enter recipe name"
                  autoFocus
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-textStandard mb-2">
                  Save Location
                </label>
                <div className="space-y-2">
                  <label className="flex items-center">
                    <input
                      type="radio"
                      name="save-location"
                      checked={saveGlobal}
                      onChange={() => setSaveGlobal(true)}
                      className="mr-2"
                    />
                    <span className="text-sm text-textStandard">
                      Global - Available across all Goose sessions
                    </span>
                  </label>
                  <label className="flex items-center">
                    <input
                      type="radio"
                      name="save-location"
                      checked={!saveGlobal}
                      onChange={() => setSaveGlobal(false)}
                      className="mr-2"
                    />
                    <span className="text-sm text-textStandard">
                      Directory - Available in the working directory
                    </span>
                  </label>
                </div>
              </div>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <button
                onClick={() => {
                  setShowSaveDialog(false);
                  setSaveRecipeName('');
                }}
                className="px-4 py-2 text-textSubtle hover:text-textStandard transition-colors"
                disabled={saving}
              >
                Cancel
              </button>
              <button
                onClick={handleSaveRecipe}
                disabled={!saveRecipeName.trim() || saving}
                className="px-4 py-2 bg-textProminent text-bgApp rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? 'Saving...' : 'Save Recipe'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* View Recipe Modal */}
      {recipeConfig && (
        <ViewRecipeModal
          isOpen={showViewRecipeModal}
          onClose={() => setShowViewRecipeModal(false)}
          config={recipeConfig}
        />
      )}
    </div>
  );
}
