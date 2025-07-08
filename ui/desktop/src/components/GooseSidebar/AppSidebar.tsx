import React, { useEffect, useState } from 'react';
import { FileText, Clock, Home, Puzzle, History, FolderKanban } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  SidebarContent,
  SidebarFooter,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarGroup,
  SidebarGroupContent,
  SidebarSeparator,
} from '../ui/sidebar';
import { ChatSmart, Gear } from '../icons';
import { ViewOptions, View } from '../../App';
import { Recipe } from '../../recipe';
import { saveRecipe } from '../../recipe/recipeStorage';
import { useChatContext } from '../../contexts/ChatContext';

interface SidebarProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
  children?: React.ReactNode;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  currentPath?: string;
}

// Main Sidebar Component
const AppSidebar: React.FC<SidebarProps> = ({ currentPath }) => {
  const navigate = useNavigate();
  // const isVisible = useState(false);
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveRecipeName, setSaveRecipeName] = useState('');
  const [saveGlobal, setSaveGlobal] = useState(true);
  const [saving, setSaving] = useState(false);
  const { resetChat } = useChatContext();

  useEffect(() => {
    // Trigger animation after a small delay
    const timer = setTimeout(() => {
      // setIsVisible(true);
    }, 100);
    // eslint-disable-next-line no-undef
    return () => clearTimeout(timer);
  }, []);

  const handleSaveRecipe = async () => {
    if (!saveRecipeName.trim()) {
      return;
    }

    setSaving(true);
    try {
      const currentRecipeConfig = window.appConfig.get('recipeConfig');

      if (!currentRecipeConfig || typeof currentRecipeConfig !== 'object') {
        throw new Error('No recipe configuration found');
      }

      const recipe = currentRecipeConfig as Recipe;
      if (!recipe.title || !recipe.description || !recipe.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      const filePath = await saveRecipe(recipe, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      console.log(`Recipe saved to: ${filePath}`);

      setShowSaveDialog(false);
      setSaveRecipeName('');

      window.electron.showNotification({
        title: 'Recipe Saved',
        body: `Recipe "${saveRecipeName}" has been saved successfully.`,
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      window.electron.showNotification({
        title: 'Save Failed',
        body: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
      });
    } finally {
      setSaving(false);
    }
  };

  // Helper function to check if a path is active
  const isActivePath = (path: string) => {
    return currentPath === path;
  };

  return (
    <>
      <SidebarContent className="pt-16">
        {/* Menu */}
        <SidebarMenu>
          {/* Navigation Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.05 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => {
                      // Always reset chat and navigate to home when clicking Home button
                      // This ensures insights are displayed regardless of active session
                      resetChat();
                      navigate('/');
                    }}
                    isActive={isActivePath('/')}
                    tooltip="Go back to the main chat screen"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Home className="w-4 h-4" />
                    <span>Home</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Chat & Configuration Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.2 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/pair')}
                    isActive={isActivePath('/pair')}
                    tooltip="Start pairing with Goose"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <ChatSmart className="w-4 h-4" />
                    <span>Chat</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>

              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.25 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/extensions')}
                    isActive={isActivePath('/extensions')}
                    tooltip="Manage your extensions"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Puzzle className="w-4 h-4" />
                    <span>Extensions</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>

              {process.env.ALPHA && (
                <motion.div
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.3 }}
                >
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      onClick={() => navigate('/schedules')}
                      isActive={isActivePath('/schedules')}
                      tooltip="Manage scheduled runs"
                      className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                    >
                      <Clock className="w-4 h-4" />
                      <span>Scheduler</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </motion.div>
              )}
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Content Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.35 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/projects')}
                    isActive={isActivePath('/projects')}
                    tooltip="Manage your projects"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <FolderKanban className="w-4 h-4" />
                    <span>Projects</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>

              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.4 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/sessions')}
                    isActive={isActivePath('/sessions')}
                    tooltip="View your session history"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <History className="w-4 h-4" />
                    <span>Sessions</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>

              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.45 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/recipes')}
                    isActive={isActivePath('/recipes')}
                    tooltip="Browse your saved recipes"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <FileText className="w-4 h-4" />
                    <span>Recipes</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Settings Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ type: 'spring', stiffness: 300, damping: 25, delay: 0.5 }}
              >
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/settings')}
                    isActive={isActivePath('/settings')}
                    tooltip="Configure Goose settings"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Gear className="w-4 h-4" />
                    <span>Settings</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </motion.div>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarMenu>
      </SidebarContent>

      <SidebarFooter />

      {/* Save Recipe Dialog */}
      {showSaveDialog && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black bg-opacity-50">
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
                className="px-4 py-2 bg-borderProminent text-text-on-accent rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? 'Saving...' : 'Save Recipe'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};

export default AppSidebar;
