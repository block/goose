import { Popover, PopoverContent, PopoverPortal, PopoverTrigger } from '../ui/popover';
import React, { useEffect, useState } from 'react';
import { ChatSmart, Idea, Refresh, Time, Send, Settings } from '../icons';
import { FolderOpen, Moon, Sliders, Sun, Save, FileText } from 'lucide-react';
import { useConfig } from '../ConfigContext';
import { ViewOptions, View } from '../../App';
import { saveRecipe, generateRecipeFilename } from '../../recipe/recipeStorage';
import { Recipe } from '../../recipe';

// RecipeConfig is used for window creation and should match Recipe interface
type RecipeConfig = Recipe;

interface MenuButtonProps {
  onClick: () => void;
  children: React.ReactNode;
  subtitle?: string;
  className?: string;
  danger?: boolean;
  icon?: React.ReactNode;
  testId?: string;
}

const MenuButton: React.FC<MenuButtonProps> = ({
  onClick,
  children,
  subtitle,
  className = '',
  danger = false,
  icon,
  testId = '',
}) => (
  <button
    onClick={onClick}
    data-testid={testId}
    className={`w-full text-left px-4 py-3 min-h-[64px] text-sm hover:bg-bgSubtle transition-[background] border-b border-borderSubtle ${
      danger ? 'text-red-400' : ''
    } ${className}`}
  >
    <div className="flex justify-between items-center">
      <div className="flex flex-col">
        <span>{children}</span>
        {subtitle && (
          <span className="text-xs font-regular text-textSubtle mt-0.5">{subtitle}</span>
        )}
      </div>
      {icon && <div className="ml-2">{icon}</div>}
    </div>
  </button>
);

interface ThemeSelectProps {
  themeMode: 'light' | 'dark' | 'system';
  onThemeChange: (theme: 'light' | 'dark' | 'system') => void;
}

const ThemeSelect: React.FC<ThemeSelectProps> = ({ themeMode, onThemeChange }) => {
  return (
    <div className="px-4 py-3 border-b border-borderSubtle">
      <div className="text-sm mb-2">Theme</div>
      <div className="grid grid-cols-3 gap-2">
        <button
          data-testid="light-mode-button"
          onClick={() => onThemeChange('light')}
          className={`flex items-center justify-center gap-2 p-2 rounded-md border transition-colors ${
            themeMode === 'light'
              ? 'border-borderStandard'
              : 'border-borderSubtle hover:border-borderStandard text-textSubtle hover:text-textStandard'
          }`}
        >
          <Sun className="h-4 w-4" />
          <span className="text-xs">Light</span>
        </button>

        <button
          data-testid="dark-mode-button"
          onClick={() => onThemeChange('dark')}
          className={`flex items-center justify-center gap-2 p-2 rounded-md border transition-colors ${
            themeMode === 'dark'
              ? 'border-borderStandard'
              : 'border-borderSubtle hover:border-borderStandard text-textSubtle hover:text-textStandard'
          }`}
        >
          <Moon className="h-4 w-4" />
          <span className="text-xs">Dark</span>
        </button>

        <button
          data-testid="system-mode-button"
          onClick={() => onThemeChange('system')}
          className={`flex items-center justify-center gap-2 p-2 rounded-md border transition-colors ${
            themeMode === 'system'
              ? 'border-borderStandard'
              : 'border-borderSubtle hover:border-borderStandard text-textSubtle hover:text-textStandard'
          }`}
        >
          <Sliders className="h-4 w-4" />
          <span className="text-xs">System</span>
        </button>
      </div>
    </div>
  );
};

export default function MoreMenu({
  setView,
  setIsGoosehintsModalOpen,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveRecipeName, setSaveRecipeName] = useState('');
  const [saveGlobal, setSaveGlobal] = useState(true);
  const [saving, setSaving] = useState(false);
  const { remove } = useConfig();
  const [themeMode, setThemeMode] = useState<'light' | 'dark' | 'system'>(() => {
    const savedUseSystemTheme = localStorage.getItem('use_system_theme') === 'true';
    if (savedUseSystemTheme) {
      return 'system';
    }
    const savedTheme = localStorage.getItem('theme');
    return savedTheme === 'dark' ? 'dark' : 'light';
  });

  const [isDarkMode, setDarkMode] = useState(() => {
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    if (themeMode === 'system') {
      return systemPrefersDark;
    }
    return themeMode === 'dark';
  });

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);

    if (themeMode === 'system') {
      setDarkMode(mediaQuery.matches);
      localStorage.setItem('use_system_theme', 'true');
    } else {
      setDarkMode(themeMode === 'dark');
      localStorage.setItem('use_system_theme', 'false');
      localStorage.setItem('theme', themeMode);
    }

    return () => mediaQuery.removeEventListener('change', handleThemeChange);
  }, [themeMode]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
      document.documentElement.classList.remove('light');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.classList.add('light');
    }
  }, [isDarkMode]);

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setThemeMode(newTheme);
  };

  const handleSaveRecipe = async () => {
    if (!saveRecipeName.trim()) {
      return;
    }

    setSaving(true);
    try {
      // Get the current recipe config from the window with proper validation
      const currentRecipeConfig = window.appConfig.get('recipeConfig');

      if (!currentRecipeConfig || typeof currentRecipeConfig !== 'object') {
        throw new Error('No recipe configuration found');
      }

      // Validate that it has the required Recipe properties
      const recipe = currentRecipeConfig as Recipe;
      if (!recipe.title || !recipe.description || !recipe.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      // Save the recipe
      const filePath = await saveRecipe(recipe, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      // Show success message (you might want to use a toast notification instead)
      console.log(`Recipe saved to: ${filePath}`);

      // Reset dialog state
      setShowSaveDialog(false);
      setSaveRecipeName('');
      setOpen(false);

      // Optional: Show a success notification
      window.electron.showNotification({
        title: 'Recipe Saved',
        body: `Recipe "${saveRecipeName}" has been saved successfully.`,
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      // Show error notification
      window.electron.showNotification({
        title: 'Save Failed',
        body: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
      });
    } finally {
      setSaving(false);
    }
  };

  const handleSaveRecipeClick = () => {
    const currentRecipeConfig = window.appConfig.get('recipeConfig');

    if (currentRecipeConfig && typeof currentRecipeConfig === 'object') {
      const recipe = currentRecipeConfig as Recipe;
      // Generate a suggested name from the recipe title
      const suggestedName = generateRecipeFilename(recipe);
      setSaveRecipeName(suggestedName);
      setShowSaveDialog(true);
      setOpen(false);
    }
  };

  const recipeConfig = window.appConfig.get('recipeConfig');
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          data-testid="more-options-button"
          className={`z-[100] w-7 h-7 p-1 rounded-full border border-borderSubtle transition-colors cursor-pointer no-drag hover:text-textStandard hover:border-borderStandard ${open ? 'text-textStandard' : 'text-textSubtle'}`}
          role="button"
        >
          <Settings />
        </button>
      </PopoverTrigger>

      <PopoverPortal>
        <>
          <div
            className={`z-[150] fixed inset-0 bg-black transition-all animate-in duration-500 fade-in-0 opacity-50`}
          />
          <PopoverContent
            className="z-[200] w-[375px] overflow-hidden rounded-lg bg-bgApp border border-borderSubtle text-textStandard !zoom-in-100 !slide-in-from-right-4 !slide-in-from-top-0"
            align="end"
            sideOffset={5}
          >
            <div className="flex flex-col rounded-md">
              <MenuButton
                onClick={() => {
                  setOpen(false);
                  window.electron.createChatWindow(
                    undefined,
                    window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
                  );
                }}
                subtitle="Start a new session in the current directory"
                icon={<ChatSmart className="w-4 h-4" />}
              >
                New session
                <span className="text-textSubtle ml-1">⌘N</span>
              </MenuButton>

              <MenuButton
                onClick={() => {
                  setOpen(false);
                  window.electron.directoryChooser();
                }}
                subtitle="Start a new session in a different directory"
                icon={<FolderOpen className="w-4 h-4" />}
              >
                Open directory
                <span className="text-textSubtle ml-1">⌘O</span>
              </MenuButton>

              <MenuButton
                onClick={() => setView('sessions')}
                subtitle="View and share previous sessions"
                icon={<Time className="w-4 h-4" />}
              >
                Session history
              </MenuButton>

              <MenuButton
                onClick={() => setView('schedules')}
                subtitle="Manage scheduled runs"
                icon={<Time className="w-4 h-4" />}
              >
                Scheduler
              </MenuButton>

              <MenuButton
                onClick={() => setIsGoosehintsModalOpen(true)}
                subtitle="Customize instructions"
                icon={<Idea className="w-4 h-4" />}
              >
                Configure .goosehints
              </MenuButton>

              {recipeConfig ? (
                <>
                  <MenuButton
                    onClick={() => {
                      setOpen(false);
                      window.electron.createChatWindow(
                        undefined, // query
                        undefined, // dir
                        undefined, // version
                        undefined, // resumeSessionId
                        recipeConfig as RecipeConfig, // recipe config
                        'recipeEditor' // view type
                      );
                    }}
                    subtitle="View the recipe you're using"
                    icon={<Send className="w-4 h-4" />}
                  >
                    View recipe
                  </MenuButton>

                  <MenuButton
                    onClick={handleSaveRecipeClick}
                    subtitle="Save this recipe for reuse"
                    icon={<Save className="w-4 h-4" />}
                  >
                    Save recipe
                  </MenuButton>
                </>
              ) : (
                <MenuButton
                  onClick={() => {
                    setOpen(false);
                    // Signal to ChatView that we want to make an agent from the current chat
                    window.electron.logInfo('Make recipe button clicked');
                    window.dispatchEvent(new CustomEvent('make-agent-from-chat'));
                  }}
                  subtitle="Make a custom agent recipe you can share or reuse with a link"
                  icon={<Send className="w-4 h-4" />}
                >
                  Make recipe from this session
                </MenuButton>
              )}
              <MenuButton
                onClick={() => {
                  setOpen(false);
                  setView('recipes');
                }}
                subtitle="Browse your saved recipes"
                icon={<FileText className="w-4 h-4" />}
              >
                Recipe Library
              </MenuButton>
              <MenuButton
                onClick={() => {
                  setOpen(false);
                  setView('settings');
                }}
                subtitle="View all settings and options"
                icon={<Sliders className="w-4 h-4 rotate-90" />}
                testId="advanced-settings-button"
              >
                Advanced settings
                <span className="text-textSubtle ml-1">⌘,</span>
              </MenuButton>

              <ThemeSelect themeMode={themeMode} onThemeChange={handleThemeChange} />

              <MenuButton
                data-testid="reset-provider-button"
                onClick={async () => {
                  await remove('GOOSE_PROVIDER', false);
                  await remove('GOOSE_MODEL', false);
                  setOpen(false);
                  setView('welcome');
                }}
                danger
                subtitle="Clear selected model and restart (alpha)"
                icon={<Refresh className="w-4 h-4 text-textStandard" />}
                className="border-b-0"
              >
                Reset provider and model
              </MenuButton>
            </div>
          </PopoverContent>
        </>
      </PopoverPortal>

      {/* Save Recipe Dialog */}
      {showSaveDialog && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black bg-opacity-50">
          <div className="bg-bgApp border border-borderSubtle rounded-lg p-6 w-96 max-w-[90vw]">
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
                  className="w-full p-3 border border-borderSubtle rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
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
                className="px-4 py-2 bg-borderProminent text-white rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? 'Saving...' : 'Save Recipe'}
              </button>
            </div>
          </div>
        </div>
      )}
    </Popover>
  );
}
