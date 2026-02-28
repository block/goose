/**
 * UnifiedInputContext — single context for the app-wide input bar.
 *
 * Merges the zone-aware slash-command system (from PromptBarContext)
 * with session-level state (from ChatInput props) so that ONE input
 * component can adapt its rendering mode based on route context.
 *
 * Rendering modes:
 *   - compact  : non-chat routes — single-line input + slash commands + Cmd+K
 *   - full     : /pair route — textarea, file drag/drop, voice, tokens, queue
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import type { Message, Recipe } from '@/api';
import type { DroppedFile } from '@/hooks/useFileDrop';
import { ChatState } from '@/types/chatState';
import type { UserInput } from '@/types/message';
import type { View, ViewOptions } from '@/utils/navigationUtils';

// ─── Zone & Slash Command Types ───────────────────────────────────

export type NavigationZone = 'home' | 'chat' | 'workflows' | 'observatory' | 'platform';

export interface SlashCommand {
  command: string;
  description: string;
  action: (args?: string) => void;
}

export interface ZoneConfig {
  zone: NavigationZone;
  placeholder: string;
  hint: string;
  actions: SlashCommand[];
}

// ─── Session-Level Props (for full mode) ──────────────────────────

export interface SessionInputState {
  sessionId: string | null;
  handleSubmit: (input: UserInput) => void;
  chatState: ChatState;
  setChatState?: (state: ChatState) => void;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
  droppedFiles?: DroppedFile[];
  onFilesProcessed?: () => void;
  setView: (view: View, options?: ViewOptions) => void;
  totalTokens?: number;
  accumulatedInputTokens?: number;
  accumulatedOutputTokens?: number;
  messages?: Message[];
  sessionCosts?: Record<string, { inputTokens: number; outputTokens: number; totalCost: number }>;
  disableAnimation?: boolean;
  recipe?: Recipe | null;
  recipeId?: string | null;
  recipeAccepted?: boolean;
  initialPrompt?: string;
  toolCount: number;
  append?: (message: Message) => void;
  onWorkingDirChange?: (newDir: string) => void;
  inputRef?: React.RefObject<HTMLTextAreaElement | null>;
}

// ─── Unified Context Value ────────────────────────────────────────

export type InputMode = 'compact' | 'full';

export interface UnifiedInputContextValue {
  mode: InputMode;
  zone: NavigationZone;
  config: ZoneConfig;
  slashCommands: SlashCommand[];
  session: SessionInputState | null;
  submitPrompt: (text: string) => void;
  setSessionState: (
    state:
      | SessionInputState
      | null
      | ((prev: SessionInputState | null) => SessionInputState | null)
  ) => void;
}

const UnifiedInputContext = createContext<UnifiedInputContextValue | null>(null);

export function useUnifiedInput() {
  const ctx = useContext(UnifiedInputContext);
  if (!ctx) throw new Error('useUnifiedInput must be used within UnifiedInputProvider');
  return ctx;
}

// Hook for session components (BaseChat) to register their session into the unified context.
// Uses a ref for the context setter to avoid re-running the effect when the context value changes.
export function useRegisterSession(
  state: Partial<SessionInputState> & { sessionId: string | null }
) {
  const ctx = useContext(UnifiedInputContext);
  const stateRef = useRef(state);
  stateRef.current = state;

  const setSessionStateRef = useRef(ctx?.setSessionState);
  setSessionStateRef.current = ctx?.setSessionState;

  const stableSubmit = useCallback((input: UserInput) => {
    stateRef.current.handleSubmit?.(input);
  }, []);

  const stableSetChatState = useCallback((nextState: ChatState) => {
    stateRef.current.setChatState?.(nextState);
  }, []);

  const stableOnStop = useCallback(() => {
    stateRef.current.onStop?.();
  }, []);

  const stableOnFilesProcessed = useCallback(() => {
    stateRef.current.onFilesProcessed?.();
  }, []);

  const stableAppend = useCallback((message: Message) => {
    stateRef.current.append?.(message);
  }, []);

  const stableOnWorkingDirChange = useCallback((newDir: string) => {
    stateRef.current.onWorkingDirChange?.(newDir);
  }, []);

  const stableSetView = useCallback((view: View, options?: ViewOptions) => {
    stateRef.current.setView?.(view, options);
  }, []);

  // 1) Register/unregister ONLY when sessionId changes or on unmount.
  //    The cleanup setter(null) runs only here — never on field updates.
  useEffect(() => {
    const setter = setSessionStateRef.current;
    const sessionId = state.sessionId;
    if (!setter || !sessionId) return;

    setter({
      sessionId,
      chatState: stateRef.current.chatState ?? ChatState.Idle,
      handleSubmit: stableSubmit,
      setView: stableSetView,
      toolCount: stateRef.current.toolCount ?? 0,
      setChatState: stableSetChatState,
      onStop: stableOnStop,
      onFilesProcessed: stableOnFilesProcessed,
      append: stableAppend,
      onWorkingDirChange: stableOnWorkingDirChange,
    });

    return () => {
      setter(null);
    };
  }, [state.sessionId, stableSubmit, stableSetView, stableSetChatState, stableOnStop, stableOnFilesProcessed, stableAppend, stableOnWorkingDirChange]);

  // 2) Update session fields without unregistering.
  //    Uses functional update to avoid overwriting a different session.
  useEffect(() => {
    const setter = setSessionStateRef.current;
    const sessionId = state.sessionId;
    if (!setter || !sessionId) return;

    setter((prev) => {
      if (!prev || prev.sessionId !== sessionId) return prev;

      const next = {
        ...prev,
        chatState: state.chatState ?? ChatState.Idle,
        toolCount: state.toolCount ?? 0,
        commandHistory: state.commandHistory,
        droppedFiles: state.droppedFiles,
        totalTokens: state.totalTokens,
        accumulatedInputTokens: state.accumulatedInputTokens,
        accumulatedOutputTokens: state.accumulatedOutputTokens,
        messages: state.messages,
        sessionCosts: state.sessionCosts,
        recipe: state.recipe,
        recipeId: state.recipeId,
        recipeAccepted: state.recipeAccepted,
        initialPrompt: state.initialPrompt,
        inputRef: state.inputRef,
      };

      // Avoid infinite render loops by only updating context when something actually changed.
      const changed =
        next.chatState !== prev.chatState ||
        next.toolCount !== prev.toolCount ||
        next.commandHistory !== prev.commandHistory ||
        next.droppedFiles !== prev.droppedFiles ||
        next.totalTokens !== prev.totalTokens ||
        next.accumulatedInputTokens !== prev.accumulatedInputTokens ||
        next.accumulatedOutputTokens !== prev.accumulatedOutputTokens ||
        next.messages !== prev.messages ||
        next.sessionCosts !== prev.sessionCosts ||
        next.recipe !== prev.recipe ||
        next.recipeId !== prev.recipeId ||
        next.recipeAccepted !== prev.recipeAccepted ||
        next.initialPrompt !== prev.initialPrompt ||
        next.inputRef !== prev.inputRef ||
        false;

      return changed ? next : prev;
    });
  }, [
    state.sessionId,
    state.chatState,
    state.toolCount,
    state.totalTokens,
    state.messages,
    state.accumulatedInputTokens,
    state.accumulatedOutputTokens,
    state.droppedFiles,
    state.commandHistory,
    state.recipe,
    state.recipeId,
    state.recipeAccepted,
    state.initialPrompt,
    state.sessionCosts,
    state.inputRef,
  ]);
}

// ─── Zone Detection ───────────────────────────────────────────────

const ZONE_MAP: Record<string, NavigationZone> = {
  '/': 'home',
  '/pair': 'chat',
  '/recipes': 'workflows',
  '/apps': 'workflows',
  '/schedules': 'workflows',
  '/analytics': 'observatory',
  '/monitoring': 'observatory',
  '/evaluate': 'observatory',
  '/tools': 'observatory',
  '/agents': 'observatory',
  '/extensions': 'platform',
  '/catalogs': 'platform',
  '/settings': 'platform',
  '/sessions': 'home',
};

function getZoneFromPath(pathname: string): NavigationZone {
  if (ZONE_MAP[pathname]) return ZONE_MAP[pathname];
  for (const [path, zone] of Object.entries(ZONE_MAP)) {
    if (pathname.startsWith(path) && path !== '/') return zone;
  }
  return 'home';
}

// ─── Provider ─────────────────────────────────────────────────────

interface UnifiedInputProviderProps {
  children: ReactNode;
  onCreateSession?: (message: string) => void;
}

export function UnifiedInputProvider({ children, onCreateSession }: UnifiedInputProviderProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const [, setLastCommand] = useState('');
  const [session, setSessionState] = useState<SessionInputState | null>(null);

  const zone = useMemo(() => getZoneFromPath(location.pathname), [location.pathname]);
  const isOnPairRoute = location.pathname === '/pair';
  const mode: InputMode = isOnPairRoute ? 'full' : 'compact';

  // Global slash commands (available everywhere)
  const globalCommands: SlashCommand[] = useMemo(
    () => [
      {
        command: '/new',
        description: 'Start a new chat session',
        action: () => window.dispatchEvent(new CustomEvent('TRIGGER_NEW_CHAT')),
      },
      {
        command: '/recipe',
        description: 'Browse and run recipes',
        action: (args?: string) => {
          if (args) {
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
        action: () => navigate('/sessions/history'),
      },
      {
        command: '/help',
        description: 'Show available commands',
        action: () => setLastCommand('/help'),
      },
    ],
    [navigate]
  );

  // Zone-specific commands
  const zoneCommands: SlashCommand[] = useMemo(() => {
    switch (zone) {
      case 'observatory':
        return [
          {
            command: '/eval',
            description: 'Run an evaluation',
            action: () => navigate('/evaluate'),
          },
          {
            command: '/monitor',
            description: 'View live monitoring',
            action: () => navigate('/monitoring'),
          },
        ];
      case 'workflows':
        return [
          {
            command: '/schedule',
            description: 'Schedule a recipe',
            action: () => navigate('/schedules'),
          },
          {
            command: '/create',
            description: 'Create a new recipe',
            action: () => navigate('/recipes'),
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
          placeholder: '',
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

  // Use refs for values that change frequently to keep submitPrompt stable
  const sessionRef = useRef(session);
  sessionRef.current = session;
  const slashCommandsRef = useRef(slashCommands);
  slashCommandsRef.current = slashCommands;
  const onCreateSessionRef = useRef(onCreateSession);
  onCreateSessionRef.current = onCreateSession;

  const submitPrompt = useCallback((text: string) => {
    const trimmed = text.trim();

    // Handle slash commands first
    if (trimmed.startsWith('/')) {
      const parts = trimmed.split(' ');
      const cmd = parts[0].toLowerCase();
      const args = parts.slice(1).join(' ');

      const command = slashCommandsRef.current.find((c) => c.command === cmd);
      if (command) {
        command.action(args);
        return;
      }
    }

    // If we have a session submit handler (full mode), delegate to it
    if (sessionRef.current?.handleSubmit) {
      sessionRef.current.handleSubmit({ msg: trimmed, images: [] });
      return;
    }

    // Default: create a new session with this message
    if (onCreateSessionRef.current) {
      onCreateSessionRef.current(trimmed);
    } else {
      window.dispatchEvent(new CustomEvent('PROMPT_BAR_SUBMIT', { detail: { message: trimmed } }));
    }
  }, []); // stable — reads everything from refs

  const value = useMemo<UnifiedInputContextValue>(
    () => ({
      mode,
      zone,
      config,
      slashCommands,
      session,
      submitPrompt,
      setSessionState,
    }),
    [mode, zone, config, slashCommands, session, submitPrompt]
  );

  return <UnifiedInputContext.Provider value={value}>{children}</UnifiedInputContext.Provider>;
}
