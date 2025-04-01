import {
  Popover,
  PopoverContent,
  PopoverPortal,
  PopoverTrigger,
} from '../../components/ui/popover';
import React, { useEffect, useState } from 'react';
import { ChatSmart, Idea, More, Refresh, Time, Send } from '../icons';
import { FolderOpen, Moon, Sliders, Sun } from 'lucide-react';
import { View } from '../../App';
import { useConfig } from '../ConfigContext';
import { toastService } from '../../toasts';

interface VersionInfo {
  current_version: string;
  available_versions: string[];
}

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

interface DarkModeToggleProps {
  isDarkMode: boolean;
  onToggle: () => void;
}

const DarkModeToggle: React.FC<DarkModeToggleProps> = ({ isDarkMode, onToggle }) => (
  <button
    className="flex items-center min-h-[64px] justify-between px-4 py-3 hover:bg-bgSubtle border-b border-borderSubtle"
    onClick={onToggle}
  >
    <div className="flex flex-col items-start">
      <span className="text-sm">{isDarkMode ? 'Light Mode' : 'Dark Mode'}</span>
      <span className="text-xs font-regular text-textSubtle mt-0.5">
        {isDarkMode ? 'Switch to light theme' : 'Switch to dark theme'}
      </span>
    </div>
    <div className="h-4 w-4 overflow-hidden relative rounded-full">
      <div className="absolute bg-bg flex h-4 w-4 flex-row items-center justify-center transition-transform rotate-180 dark:rotate-0 translate-x-[100%] dark:translate-x-[0%]">
        <Sun className="h-4 w-4 transition-all duration-[400ms]" />
      </div>

      <div className="absolute bg-bg flex h-4 w-4 flex-row items-center justify-center transition-transform dark:translate-x-[-100%] dark:-rotate-90">
        <Moon className="h-4 w-4 transition-all duration-[400ms]" />
      </div>
    </div>
  </button>
);

export default function MoreMenu({
  setView,
  setIsGoosehintsModalOpen,
}: {
  setView: (view: View) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const { remove } = useConfig();
  const [versions, setVersions] = useState<VersionInfo | null>(null);
  const [showVersions, setShowVersions] = useState(false);
  const [useSystemTheme, setUseSystemTheme] = useState(
    () => localStorage.getItem('use_system_theme') === 'true'
  );
  const [isDarkMode, setDarkMode] = useState(() => {
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    if (useSystemTheme) {
      return systemPrefersDark;
    }
    const savedTheme = localStorage.getItem('theme');
    return savedTheme ? savedTheme === 'dark' : systemPrefersDark;
  });

  useEffect(() => {
    // Fetch available versions when the menu opens
    const fetchVersions = async () => {
      try {
        const port = window.appConfig.get('GOOSE_PORT');
        const response = await fetch(`http://127.0.0.1:${port}/agent/versions`);
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setVersions(data);
      } catch (error) {
        console.error('Failed to fetch versions:', error);
      }
    };

    if (open) {
      fetchVersions();
    }
  }, [open]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    // Handler for system theme changes
    const handleThemeChange = (e: { matches: boolean }) => {
      if (useSystemTheme) {
        setDarkMode(e.matches);
      }
    };

    // Add listener for system theme changes
    mediaQuery.addEventListener('change', handleThemeChange);

    // Initial setup
    if (useSystemTheme) {
      setDarkMode(mediaQuery.matches);
    } else {
      const savedTheme = localStorage.getItem('theme');
      setDarkMode(savedTheme ? savedTheme === 'dark' : mediaQuery.matches);
    }

    // Cleanup
    return () => mediaQuery.removeEventListener('change', handleThemeChange);
  }, [useSystemTheme]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
    if (!useSystemTheme) {
      localStorage.setItem('theme', isDarkMode ? 'dark' : 'light');
    }
  }, [isDarkMode, useSystemTheme]);

  const toggleTheme = () => {
    if (!useSystemTheme) {
      setDarkMode(!isDarkMode);
    }
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          className={`z-[100] absolute top-2 right-4 w-[20px] h-[20px] transition-colors cursor-pointer no-drag hover:text-textProminent ${open ? 'text-textProminent' : 'text-textSubtle'}`}
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

              <DarkModeToggle isDarkMode={isDarkMode} onToggle={toggleTheme} />

              {/* Make Agent from Chat */}
              <MenuButton
                onClick={() => {
                  setOpen(false);
                  // Signal to ChatView that we want to make an agent from the current chat
                  window.electron.logInfo('Make Agent button clicked');
                  window.dispatchEvent(new CustomEvent('make-agent-from-chat'));
                }}
                subtitle="Make a custom agent you can share or reuse with a link"
                icon={<Send className="w-4 h-4" />}
              >
                Make Agent from this session
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

              {process.env.ALPHA && (
                <MenuButton
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

              {!process.env.ALPHA && (
                <MenuButton
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
