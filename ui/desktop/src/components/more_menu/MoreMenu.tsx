import { Popover, PopoverContent, PopoverPortal, PopoverTrigger } from '../ui/popover';
import React, { useEffect, useState } from 'react';
import { ChatSmart, Idea, More, Refresh, Time, Send } from '../icons';
import { FolderOpen, Moon, Sliders, Sun } from 'lucide-react';
import { useConfig } from '../ConfigContext';
import { settingsV2Enabled } from '../../flags';
import { ViewOptions, View } from '../../App';

interface MenuButtonProps {
  onClick: () => void;
  children: React.ReactNode;
  subtitle?: string;
  className?: string;
  danger?: boolean;
  icon?: React.ReactNode;
}

const MenuButton: React.FC<MenuButtonProps> = ({
  onClick,
  children,
  subtitle,
  className = '',
  danger = false,
  icon,
}) => (
  <button
    onClick={onClick}
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

    // Handler for system theme changes
    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    // Add listener for system theme changes
    mediaQuery.addEventListener('change', handleThemeChange);

    // Initial setup
    if (themeMode === 'system') {
      setDarkMode(mediaQuery.matches);
      localStorage.setItem('use_system_theme', 'true');
    } else {
      setDarkMode(themeMode === 'dark');
      localStorage.setItem('use_system_theme', 'false');
      localStorage.setItem('theme', themeMode);
    }

    // Cleanup
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

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          data-testid="more-options-button"
          className={`z-[100] absolute top-2 right-4 w-[20px] h-[20px] transition-colors cursor-pointer no-drag hover:text-textProminent ${open ? 'text-textProminent' : 'text-textSubtle'}`}
          role="button"
        >
          <More />
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
                    window.appConfig.get('GOOSE_WORKING_DIR')
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
                onClick={() => setIsGoosehintsModalOpen(true)}
                subtitle="Customize instructions"
                icon={<Idea className="w-4 h-4" />}
              >
                Configure .goosehints
              </MenuButton>

              {/* Make Agent from Chat - disabled if already in a recipe */}
              <MenuButton
                onClick={() => {
                  const recipeConfig = window.appConfig.get('recipeConfig');
                  if (!recipeConfig) {
                    setOpen(false);
                    // Signal to ChatView that we want to make an agent from the current chat
                    window.electron.logInfo('Make Agent button clicked');
                    window.dispatchEvent(new CustomEvent('make-agent-from-chat'));
                  }
                }}
                subtitle="Make a custom agent you can share or reuse with a link"
                icon={<Send className="w-4 h-4" />}
                className={
                  window.appConfig.get('recipeConfig') ? 'opacity-50 cursor-not-allowed' : ''
                }
              >
                Make Agent from this session
                {window.appConfig.get('recipeConfig') && (
                  <div className="text-xs text-textSubtle mt-1">
                    (Not available while using a recipe/botling)
                  </div>
                )}
              </MenuButton>

              <MenuButton
                onClick={() => {
                  setOpen(false);
                  setView('settings');
                }}
                subtitle="View all settings and options"
                icon={<Sliders className="w-4 h-4 rotate-90" />}
              >
                Advanced settings
                <span className="text-textSubtle ml-1">⌘,</span>
              </MenuButton>

              <ThemeSelect themeMode={themeMode} onThemeChange={handleThemeChange} />

              {settingsV2Enabled && (
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
              )}

              {!settingsV2Enabled && (
                <MenuButton
                  data-testid="reset-provider-button"
                  onClick={() => {
                    localStorage.removeItem('GOOSE_PROVIDER');
                    setOpen(false);
                    window.electron.createChatWindow();
                  }}
                  danger
                  subtitle="Clear selected model and restart"
                  icon={<Refresh className="w-4 h-4 text-textStandard" />}
                  className="border-b-0"
                >
                  Reset provider and model
                </MenuButton>
              )}
            </div>
          </PopoverContent>
        </>
      </PopoverPortal>
    </Popover>
  );
}
