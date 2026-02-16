import { createContext, useContext, useState, useCallback, useMemo, ReactNode } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

export type NavigationZone = 'home' | 'chat' | 'workflows' | 'observatory' | 'platform';

export interface ZoneConfig {
  zone: NavigationZone;
  placeholder: string;
  hint: string;
  actions: SlashCommand[];
}

export interface SlashCommand {
  command: string;
  description: string;
  icon?: string;
  action: (args: string) => void;
}

const ZONE_MAP: Record<string, NavigationZone> = {
  '/': 'home',
  '/pair': 'chat',
  '/recipes': 'workflows',
  '/apps': 'workflows',
  '/schedules': 'workflows',
  '/analytics': 'observatory',
  '/tools': 'observatory',
  '/agents': 'observatory',
  '/extensions': 'platform',
  '/settings': 'platform',
  '/sessions': 'home',
};

function getZoneFromPath(pathname: string): NavigationZone {
  // Exact match first
  if (ZONE_MAP[pathname]) return ZONE_MAP[pathname];
  // Prefix match for nested routes
  for (const [path, zone] of Object.entries(ZONE_MAP)) {
    if (pathname.startsWith(path) && path !== '/') return zone;
  }
  return 'home';
}

interface PromptBarContextValue {
  zone: NavigationZone;
  config: ZoneConfig;
  isChatActive: boolean;
  showPromptBar: boolean;
  submitPrompt: (text: string) => void;
  slashCommands: SlashCommand[];
}

const PromptBarContext = createContext<PromptBarContextValue | null>(null);

export function usePromptBar() {
  const ctx = useContext(PromptBarContext);
  if (!ctx) throw new Error('usePromptBar must be used within PromptBarProvider');
  return ctx;
}

interface PromptBarProviderProps {
  children: ReactNode;
  onCreateSession?: (message: string) => void;
}

export function PromptBarProvider({ children, onCreateSession }: PromptBarProviderProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const [, setLastCommand] = useState('');

  const zone = useMemo(() => getZoneFromPath(location.pathname), [location.pathname]);
  const isChatActive = location.pathname === '/pair';

  // Slash commands available everywhere
  const globalCommands: SlashCommand[] = useMemo(() => [
    {
      command: '/new',
      description: 'Start a new chat session',
      action: () => {
        window.dispatchEvent(new CustomEvent('TRIGGER_NEW_CHAT'));
      },
    },
    {
      command: '/recipe',
      description: 'Browse and run recipes',
      action: (args: string) => {
        if (args) {
          // Search for recipe
          navigate(`/recipes?search=${encodeURIComponent(args)}`);
        } else {
          navigate('/recipes');
        }
      },
    },
    {
      command: '/settings',
      description: 'Open settings',
      action: () => navigate('/settings'),
    },
    {
      command: '/model',
      description: 'Change model configuration',
      action: () => navigate('/settings'),
    },
    {
      command: '/project',
      description: 'Switch project directory',
      action: () => navigate('/sessions'),
    },
    {
      command: '/help',
      description: 'Show available commands',
      action: () => setLastCommand('/help'),
    },
  ], [navigate]);

  // Zone-specific commands
  const zoneCommands: SlashCommand[] = useMemo(() => {
    switch (zone) {
      case 'observatory':
        return [
          {
            command: '/eval',
            description: 'Run evaluation on a dataset',
            action: () => navigate('/analytics'),
          },
          {
            command: '/tools',
            description: 'Check tool health status',
            action: () => navigate('/tools'),
          },
        ];
      case 'workflows':
        return [
          {
            command: '/schedule',
            description: 'Create a new schedule',
            action: () => navigate('/schedules'),
          },
        ];
      case 'platform':
        return [
          {
            command: '/install',
            description: 'Install an extension',
            action: () => navigate('/extensions'),
          },
        ];
      default:
        return [];
    }
  }, [zone, navigate]);

  const slashCommands = useMemo(
    () => [...zoneCommands, ...globalCommands],
    [zoneCommands, globalCommands]
  );

  const config: ZoneConfig = useMemo(() => {
    switch (zone) {
      case 'home':
        return {
          zone,
          placeholder: 'Ask anything or type / for commands...',
          hint: 'Start a conversation or use /recipe to run a workflow',
          actions: slashCommands,
        };
      case 'chat':
        return {
          zone,
          placeholder: '', // ChatInput handles this
          hint: '',
          actions: slashCommands,
        };
      case 'workflows':
        return {
          zone,
          placeholder: 'Describe a workflow or type / for commands...',
          hint: 'Try: "Create a recipe that reviews PRs" or /schedule',
          actions: slashCommands,
        };
      case 'observatory':
        return {
          zone,
          placeholder: 'Ask about performance or type / for commands...',
          hint: 'Try: "Show me accuracy trends" or /eval',
          actions: slashCommands,
        };
      case 'platform':
        return {
          zone,
          placeholder: 'Search catalogs or type / for commands...',
          hint: 'Try: "Find a GitHub extension" or /install',
          actions: slashCommands,
        };
    }
  }, [zone, slashCommands]);

  const submitPrompt = useCallback((text: string) => {
    const trimmed = text.trim();

    // Handle slash commands
    if (trimmed.startsWith('/')) {
      const parts = trimmed.split(' ');
      const cmd = parts[0].toLowerCase();
      const args = parts.slice(1).join(' ');

      const command = slashCommands.find(c => c.command === cmd);
      if (command) {
        command.action(args);
        return;
      }
    }

    // Default: create a new session with this message
    if (onCreateSession) {
      onCreateSession(trimmed);
    } else {
      // Dispatch event for Hub to handle
      window.dispatchEvent(new CustomEvent('PROMPT_BAR_SUBMIT', { detail: { message: trimmed } }));
    }
  }, [slashCommands, onCreateSession]);

  const showPromptBar = !isChatActive;

  const value = useMemo(() => ({
    zone,
    config,
    isChatActive,
    showPromptBar,
    submitPrompt,
    slashCommands,
  }), [zone, config, isChatActive, showPromptBar, submitPrompt, slashCommands]);

  return (
    <PromptBarContext.Provider value={value}>
      {children}
    </PromptBarContext.Provider>
  );
}
