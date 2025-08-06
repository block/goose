import { useEffect, useRef, useState } from 'react';
import { IpcRendererEvent } from 'electron';
import { HashRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { openSharedSessionFromDeepLink, type SessionLinksViewOptions } from './sessionLinks';
import { type SharedSessionDetails } from './sharedSessions';
import { initializeSystem } from './utils/providerUtils';
import { initializeCostDatabase } from './utils/costDatabase';
import { ErrorUI } from './components/ErrorBoundary';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';
import { type ExtensionConfig } from './extensions';
import AnnouncementModal from './components/AnnouncementModal';
import { generateSessionId } from './sessions';
import ProviderGuard from './components/ProviderGuard';

import { ChatType } from './types/chat';
import Hub from './components/hub';
import Pair from './components/pair';
import SettingsView, { SettingsViewOptions } from './components/settings/SettingsView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import SchedulesView from './components/schedule/SchedulesView';
import ProviderSettings from './components/settings/providers/ProviderSettingsPage';
import { useChat } from './hooks/useChat';
import { AppLayout } from './components/Layout/AppLayout';
import { ChatProvider } from './contexts/ChatContext';
import { DraftProvider } from './contexts/DraftContext';

import 'react-toastify/dist/ReactToastify.css';
import { useConfig, MalformedConfigError } from './components/ConfigContext';
import { ModelAndProviderProvider } from './components/ModelAndProviderContext';
import { addExtensionFromDeepLink as addExtensionFromDeepLinkV2 } from './components/settings/extensions';
import {
  backupConfig,
  initConfig,
  readAllConfig,
  recoverConfig,
  validateConfig,
} from './api/sdk.gen';
import PermissionSettingsView from './components/settings/permission/PermissionSetting';
import { COST_TRACKING_ENABLED } from './updates';

import { type SessionDetails } from './sessions';
import ExtensionsView, { ExtensionsViewOptions } from './components/extensions/ExtensionsView';
// import ProjectsContainer from './components/projects/ProjectsContainer';
import { Recipe } from './recipe';
import RecipesView from './components/RecipesView';
import RecipeEditor from './components/RecipeEditor';

export type View =
  | 'welcome'
  | 'chat'
  | 'pair'
  | 'settings'
  | 'extensions'
  | 'moreModels'
  | 'configureProviders'
  | 'configPage'
  | 'ConfigureProviders'
  | 'settingsV2'
  | 'sessions'
  | 'schedules'
  | 'sharedSession'
  | 'loading'
  | 'recipeEditor'
  | 'recipes'
  | 'permission';
// | 'projects';

export type ViewOptions = {
  // Settings view options
  extensionId?: string;
  showEnvVars?: boolean;
  deepLinkConfig?: ExtensionConfig;

  // Session view options
  resumedSession?: SessionDetails;
  sessionDetails?: SessionDetails;
  error?: string;
  shareToken?: string;
  baseUrl?: string;

  // Recipe editor options
  config?: unknown;

  // Permission view options
  parentView?: View;

  // Generic options
  [key: string]: unknown;
};

export type ViewConfig = {
  view: View;
  viewOptions?: ViewOptions;
};

// Route Components
const HubRouteWrapper = ({
  chat,
  setChat,
  setPairChat,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setPairChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) => {
  const navigate = useNavigate();

  return (
    <Hub
      readyForAutoUserPrompt={true}
      chat={chat}
      setChat={setChat}
      setPairChat={setPairChat}
      setView={(view: View, options?: ViewOptions) => {
        // Convert view to route navigation
        switch (view) {
          case 'chat':
            navigate('/');
            break;
          case 'pair':
            navigate('/pair', { state: options });
            break;
          case 'settings':
            navigate('/settings', { state: options });
            break;
          case 'sessions':
            navigate('/sessions');
            break;
          case 'schedules':
            navigate('/schedules');
            break;
          case 'recipes':
            navigate('/recipes');
            break;
          case 'permission':
            navigate('/permission', { state: options });
            break;
          case 'ConfigureProviders':
            navigate('/configure-providers');
            break;
          case 'sharedSession':
            navigate('/shared-session', { state: options });
            break;
          case 'recipeEditor':
            navigate('/recipe-editor', { state: options });
            break;
          case 'welcome':
            navigate('/welcome');
            break;
          default:
            navigate('/');
        }
      }}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  );
};

const PairRouteWrapper = ({
  chat,
  setChat,
  setPairChat,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setPairChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) => {
  const navigate = useNavigate();
  const location = useLocation();
  const chatRef = useRef(chat);

  // Keep the ref updated with the current chat state
  useEffect(() => {
    chatRef.current = chat;
  }, [chat]);

  // Check if we have a resumed session or recipe config from navigation state
  useEffect(() => {
    // Only process if we actually have navigation state
    if (!location.state) {
      console.log('No navigation state, preserving existing chat state');
      return;
    }

    const resumedSession = location.state?.resumedSession as SessionDetails | undefined;
    const recipeConfig = location.state?.recipeConfig as Recipe | undefined;
    const resetChat = location.state?.resetChat as boolean | undefined;

    if (resumedSession) {
      console.log('Loading resumed session in pair view:', resumedSession.session_id);
      console.log('Current chat before resume:', chatRef.current);

      // Convert session to chat format - this clears any existing recipe config
      const sessionChat: ChatType = {
        id: resumedSession.session_id,
        title: resumedSession.metadata?.description || `ID: ${resumedSession.session_id}`,
        messages: resumedSession.messages,
        messageHistoryIndex: resumedSession.messages.length,
        recipeConfig: null, // Clear recipe config when resuming a session
      };

      // Update both the local chat state and the app-level pairChat state
      setChat(sessionChat);
      setPairChat(sessionChat);

      // Clear the navigation state to prevent reloading on navigation
      window.history.replaceState({}, document.title);
    } else if (recipeConfig && resetChat) {
      console.log('Loading new recipe config in pair view:', recipeConfig.title);

      const updatedChat: ChatType = {
        id: chatRef.current.id, // Keep the same ID
        title: recipeConfig.title || 'Recipe Chat',
        messages: [], // Clear messages to start fresh
        messageHistoryIndex: 0,
        recipeConfig: recipeConfig,
        recipeParameters: null, // Clear parameters for new recipe
      };

      // Update both the local chat state and the app-level pairChat state
      setChat(updatedChat);
      setPairChat(updatedChat);

      // Clear the navigation state to prevent reloading on navigation
      window.history.replaceState({}, document.title);
    } else if (recipeConfig && !chatRef.current.recipeConfig) {
      // Only set recipe config if we don't already have one (e.g., from deeplinks)

      const updatedChat: ChatType = {
        ...chatRef.current,
        recipeConfig: recipeConfig,
        title: recipeConfig.title || chatRef.current.title,
      };

      // Update both the local chat state and the app-level pairChat state
      setChat(updatedChat);
      setPairChat(updatedChat);

      // Clear the navigation state to prevent reloading on navigation
      window.history.replaceState({}, document.title);
    } else if (location.state) {
      // We have navigation state but it doesn't match our conditions
      // Clear it to prevent future processing, but don't modify chat state
      console.log('Clearing unprocessed navigation state');
      window.history.replaceState({}, document.title);
    }
    // If we have a recipe config but resetChat is false and we already have a recipe,
    // do nothing - just continue with the existing chat state
  }, [location.state, setChat, setPairChat]);

  return (
    <Pair
      chat={chat}
      setChat={setChat}
      setView={(view: View, options?: ViewOptions) => {
        // Convert view to route navigation
        switch (view) {
          case 'chat':
            navigate('/');
            break;
          case 'pair':
            navigate('/pair', { state: options });
            break;
          case 'settings':
            navigate('/settings', { state: options });
            break;
          case 'sessions':
            navigate('/sessions');
            break;
          case 'schedules':
            navigate('/schedules');
            break;
          case 'recipes':
            navigate('/recipes');
            break;
          case 'permission':
            navigate('/permission', { state: options });
            break;
          case 'ConfigureProviders':
            navigate('/configure-providers');
            break;
          case 'sharedSession':
            navigate('/shared-session', { state: options });
            break;
          case 'recipeEditor':
            navigate('/recipe-editor', { state: options });
            break;
          case 'welcome':
            navigate('/welcome');
            break;
          default:
            navigate('/');
        }
      }}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  );
};

const SettingsRoute = () => {
  const location = useLocation();
  const navigate = useNavigate();

  // Get viewOptions from location.state or history.state
  const viewOptions =
    (location.state as SettingsViewOptions) || (window.history.state as SettingsViewOptions) || {};
  return (
    <SettingsView
      onClose={() => navigate('/')}
      setView={(view: View, options?: ViewOptions) => {
        // Convert view to route navigation
        switch (view) {
          case 'chat':
            navigate('/');
            break;
          case 'pair':
            navigate('/pair');
            break;
          case 'settings':
            navigate('/settings', { state: options });
            break;
          case 'sessions':
            navigate('/sessions');
            break;
          case 'schedules':
            navigate('/schedules');
            break;
          case 'recipes':
            navigate('/recipes');
            break;
          case 'permission':
            navigate('/permission', { state: options });
            break;
          case 'ConfigureProviders':
            navigate('/configure-providers');
            break;
          case 'sharedSession':
            navigate('/shared-session', { state: options });
            break;
          case 'recipeEditor':
            navigate('/recipe-editor', { state: options });
            break;
          case 'welcome':
            navigate('/welcome');
            break;
          default:
            navigate('/');
        }
      }}
      viewOptions={viewOptions}
    />
  );
};

const SessionsRoute = () => {
  const navigate = useNavigate();

  return (
    <SessionsView
      setView={(view: View, options?: ViewOptions) => {
        // Convert view to route navigation
        switch (view) {
          case 'chat':
            navigate('/', { state: options });
            break;
          case 'pair':
            navigate('/pair', { state: options });
            break;
          case 'settings':
            navigate('/settings', { state: options });
            break;
          case 'sessions':
            navigate('/sessions');
            break;
          case 'schedules':
            navigate('/schedules');
            break;
          case 'recipes':
            navigate('/recipes');
            break;
          case 'permission':
            navigate('/permission', { state: options });
            break;
          case 'ConfigureProviders':
            navigate('/configure-providers');
            break;
          case 'sharedSession':
            navigate('/shared-session', { state: options });
            break;
          case 'recipeEditor':
            navigate('/recipe-editor', { state: options });
            break;
          case 'welcome':
            navigate('/welcome');
            break;
          default:
            navigate('/');
        }
      }}
    />
  );
};

const SchedulesRoute = () => {
  const navigate = useNavigate();
  return <SchedulesView onClose={() => navigate('/')} />;
};

const RecipesRoute = () => {
  const navigate = useNavigate();

  return (
    <RecipesView
      onLoadRecipe={(recipe) => {
        // Navigate to pair view with the recipe configuration in state
        navigate('/pair', {
          state: {
            recipeConfig: recipe,
            // Reset the pair chat to start fresh with the recipe
            resetChat: true,
          },
        });
      }}
    />
  );
};

const RecipeEditorRoute = () => {
  const location = useLocation();

  // Check for config from multiple sources:
  // 1. Location state (from navigation)
  // 2. localStorage (from "View Recipe" button)
  // 3. Window electron config (from deeplinks)
  let config = location.state?.config;

  if (!config) {
    const storedConfig = localStorage.getItem('viewRecipeConfig');
    if (storedConfig) {
      try {
        config = JSON.parse(storedConfig);
        // Clear the stored config after using it
        localStorage.removeItem('viewRecipeConfig');
      } catch (error) {
        console.error('Failed to parse stored recipe config:', error);
      }
    }
  }

  if (!config) {
    const electronConfig = window.electron.getConfig();
    config = electronConfig.recipe;
  }

  return <RecipeEditor config={config} />;
};

const PermissionRoute = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const parentView = location.state?.parentView as View;
  const parentViewOptions = location.state?.parentViewOptions as ViewOptions;

  return (
    <PermissionSettingsView
      onClose={() => {
        // Navigate back to parent view with options
        switch (parentView) {
          case 'chat':
            navigate('/');
            break;
          case 'pair':
            navigate('/pair');
            break;
          case 'settings':
            navigate('/settings', { state: parentViewOptions });
            break;
          case 'sessions':
            navigate('/sessions');
            break;
          case 'schedules':
            navigate('/schedules');
            break;
          case 'recipes':
            navigate('/recipes');
            break;
          default:
            navigate('/');
        }
      }}
    />
  );
};

const ConfigureProvidersRoute = () => {
  const navigate = useNavigate();

  return (
    <div className="w-screen h-screen bg-background-default">
      <ProviderSettings
        onClose={() => navigate('/settings', { state: { section: 'models' } })}
        isOnboarding={false}
      />
    </div>
  );
};

const WelcomeRoute = () => {
  const navigate = useNavigate();

  return (
    <div className="w-screen h-screen bg-background-default">
      <ProviderSettings onClose={() => navigate('/')} isOnboarding={true} />
    </div>
  );
};

// Wrapper component for SharedSessionRoute to access parent state
const SharedSessionRouteWrapper = ({
  isLoadingSharedSession,
  setIsLoadingSharedSession,
  sharedSessionError,
}: {
  isLoadingSharedSession: boolean;
  setIsLoadingSharedSession: (loading: boolean) => void;
  sharedSessionError: string | null;
}) => {
  const location = useLocation();
  const navigate = useNavigate();

  const historyState = window.history.state;
  const sessionDetails = (location.state?.sessionDetails ||
    historyState?.sessionDetails) as SharedSessionDetails | null;
  const error = location.state?.error || historyState?.error || sharedSessionError;
  const shareToken = location.state?.shareToken || historyState?.shareToken;
  const baseUrl = location.state?.baseUrl || historyState?.baseUrl;

  return (
    <SharedSessionView
      session={sessionDetails}
      isLoading={isLoadingSharedSession}
      error={error}
      onRetry={async () => {
        if (shareToken && baseUrl) {
          setIsLoadingSharedSession(true);
          try {
            await openSharedSessionFromDeepLink(
              `goose://sessions/${shareToken}`,
              (view: View, _options?: SessionLinksViewOptions) => {
                // Convert view to route navigation
                switch (view) {
                  case 'chat':
                    navigate('/', { state: _options });
                    break;
                  case 'pair':
                    navigate('/pair', { state: _options });
                    break;
                  case 'settings':
                    navigate('/settings', { state: _options });
                    break;
                  case 'sessions':
                    navigate('/sessions');
                    break;
                  case 'schedules':
                    navigate('/schedules');
                    break;
                  case 'recipes':
                    navigate('/recipes');
                    break;
                  case 'permission':
                    navigate('/permission', { state: _options });
                    break;
                  case 'ConfigureProviders':
                    navigate('/configure-providers');
                    break;
                  case 'sharedSession':
                    navigate('/shared-session', { state: _options });
                    break;
                  case 'recipeEditor':
                    navigate('/recipe-editor', { state: _options });
                    break;
                  default:
                    navigate('/');
                }
              },
              baseUrl
            );
          } catch (error) {
            console.error('Failed to retry loading shared session:', error);
          } finally {
            setIsLoadingSharedSession(false);
          }
        }
      }}
    />
  );
};

const ExtensionsRoute = () => {
  const navigate = useNavigate();
  const location = useLocation();

  // Get viewOptions from location.state or history.state (for deep link extensions)
  const viewOptions =
    (location.state as ExtensionsViewOptions) ||
    (window.history.state as ExtensionsViewOptions) ||
    {};

  return (
    <ExtensionsView
      onClose={() => navigate(-1)}
      setView={(view, options) => {
        switch (view) {
          case 'chat':
            navigate('/');
            break;
          case 'pair':
            navigate('/pair', { state: options });
            break;
          case 'settings':
            navigate('/settings', { state: options });
            break;
          default:
            navigate('/');
        }
      }}
      viewOptions={viewOptions}
    />
  );
};

// const ProjectsRoute = () => {
//   const navigate = useNavigate();
//
//   const setView = (view: View, viewOptions?: ViewOptions) => {
//     // Convert view to route navigation
//     switch (view) {
//       case 'chat':
//         navigate('/');
//         break;
//       case 'pair':
//         navigate('/pair', { state: viewOptions });
//         break;
//       case 'settings':
//         navigate('/settings', { state: viewOptions });
//         break;
//       case 'sessions':
//         navigate('/sessions');
//         break;
//       case 'schedules':
//         navigate('/schedules');
//         break;
//       case 'recipes':
//         navigate('/recipes');
//         break;
//       case 'permission':
//         navigate('/permission', { state: viewOptions });
//         break;
//       case 'ConfigureProviders':
//         navigate('/configure-providers');
//         break;
//       case 'sharedSession':
//         navigate('/shared-session', { state: viewOptions });
//         break;
//       case 'recipeEditor':
//         navigate('/recipe-editor', { state: viewOptions });
//         break;
//       case 'welcome':
//         navigate('/welcome');
//         break;
//       default:
//         navigate('/');
//     }
//   };
//
//   return (
//     <React.Suspense fallback={<div>Loading projects...</div>}>
//       <ProjectsContainer setView={setView} />
//     </React.Suspense>
//   );
// };

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [modalVisible, setModalVisible] = useState(false);
  const [pendingLink, setPendingLink] = useState<string | null>(null);
  const [modalMessage, setModalMessage] = useState<string>('');
  const [extensionConfirmLabel, setExtensionConfirmLabel] = useState<string>('');
  const [extensionConfirmTitle, setExtensionConfirmTitle] = useState<string>('');
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [isGoosehintsModalOpen, setIsGoosehintsModalOpen] = useState(false);
  const [isLoadingSharedSession, setIsLoadingSharedSession] = useState(false);
  const [sharedSessionError, setSharedSessionError] = useState<string | null>(null);

  // Add separate state for pair chat to maintain its own conversation
  const [pairChat, setPairChat] = useState<ChatType>({
    id: generateSessionId(),
    title: 'Pair Chat',
    messages: [],
    messageHistoryIndex: 0,
    recipeConfig: null, // Initialize with no recipe
  });

  const { getExtensions, addExtension, read } = useConfig();
  const initAttemptedRef = useRef(false);

  // Create a setView function for useChat hook - we'll use window.history instead of navigate
  const setView = (view: View, viewOptions: ViewOptions = {}) => {
    console.log(`Setting view to: ${view}`, viewOptions);
    console.trace('setView called from:'); // This will show the call stack
    // Convert view to route navigation using hash routing
    switch (view) {
      case 'chat':
        window.location.hash = '#/';
        break;
      case 'pair':
        window.location.hash = '#/pair';
        break;
      case 'settings':
        window.location.hash = '#/settings';
        break;
      case 'extensions':
        window.location.hash = '#/extensions';
        break;
      case 'sessions':
        window.location.hash = '#/sessions';
        break;
      case 'schedules':
        window.location.hash = '#/schedules';
        break;
      case 'recipes':
        window.location.hash = '#/recipes';
        break;
      case 'permission':
        window.location.hash = '#/permission';
        break;
      case 'ConfigureProviders':
        window.location.hash = '#/configure-providers';
        break;
      case 'sharedSession':
        window.location.hash = '#/shared-session';
        break;
      case 'recipeEditor':
        window.location.hash = '#/recipe-editor';
        break;
      case 'welcome':
        window.location.hash = '#/welcome';
        break;
      default:
        console.error(`Unknown view: ${view}, not navigating anywhere. This is likely a bug.`);
        console.trace('Invalid setView call stack:');
        // Don't navigate anywhere for unknown views to avoid unexpected redirects
        break;
    }
  };

  const { chat, setChat } = useChat({ setIsLoadingSession, setView, setPairChat });

  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim();
  }

  function extractRemoteUrl(link: string): string | null {
    const url = new URL(link);
    return url.searchParams.get('url');
  }

  useEffect(() => {
    if (initAttemptedRef.current) {
      console.log('Initialization already attempted, skipping...');
      return;
    }
    initAttemptedRef.current = true;

    console.log(`Initializing app`);

    const urlParams = new URLSearchParams(window.location.search);
    const viewType = urlParams.get('view');
    const resumeSessionId = urlParams.get('resumeSessionId');
    const recipeConfig = window.appConfig.get('recipe');

    // Check for session resume first - this takes priority over other navigation
    if (resumeSessionId) {
      console.log('Session resume detected, letting useChat hook handle navigation');

      // Even when resuming a session, we need to initialize the system
      const initializeForSessionResume = async () => {
        try {
          await initConfig();
          await readAllConfig({ throwOnError: true });

          const config = window.electron.getConfig();
          const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
          const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

          if (provider && model) {
            await initializeSystem(provider as string, model as string, {
              getExtensions,
              addExtension,
            });
          } else {
            throw new Error('No provider/model configured for session resume');
          }
        } catch (error) {
          console.error('Failed to initialize system for session resume:', error);
          setFatalError(
            `Failed to initialize system for session resume: ${error instanceof Error ? error.message : 'Unknown error'}`
          );
        }
      };

      initializeForSessionResume();
      return;
    }

    if (viewType) {
      if (viewType === 'recipeEditor' && recipeConfig) {
        // Handle recipe editor deep link - use hash routing
        window.location.hash = '#/recipe-editor';
        window.history.replaceState({ config: recipeConfig }, '', '#/recipe-editor');
      } else {
        // Handle other deep links by redirecting to appropriate route
        const routeMap: Record<string, string> = {
          chat: '#/',
          pair: '#/pair',
          settings: '#/settings',
          sessions: '#/sessions',
          schedules: '#/schedules',
          recipes: '#/recipes',
          permission: '#/permission',
          ConfigureProviders: '#/configure-providers',
          sharedSession: '#/shared-session',
          recipeEditor: '#/recipe-editor',
          welcome: '#/welcome',
        };

        const route = routeMap[viewType];
        if (route) {
          window.location.hash = route;
          window.history.replaceState({}, '', route);
        }
      }
      return;
    }

    const initializeApp = async () => {
      try {
        // Start cost database initialization early (non-blocking) - only if cost tracking is enabled
        const costDbPromise = COST_TRACKING_ENABLED
          ? initializeCostDatabase().catch((error) => {
              console.error('Failed to initialize cost database:', error);
            })
          : (() => {
              console.log('Cost tracking disabled, skipping cost database initialization');
              return Promise.resolve();
            })();

        await initConfig();

        try {
          await readAllConfig({ throwOnError: true });
        } catch (error) {
          console.warn('Initial config read failed, attempting recovery:', error);

          const configVersion = localStorage.getItem('configVersion');
          const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;

          if (shouldMigrateExtensions) {
            console.log('Performing extension migration...');
            try {
              await backupConfig({ throwOnError: true });
              await initConfig();
            } catch (migrationError) {
              console.error('Migration failed:', migrationError);
              // Continue with recovery attempts
            }
          }

          // Try recovery if migration didn't work or wasn't needed
          console.log('Attempting config recovery...');
          try {
            // Try to validate first (faster than recovery)
            await validateConfig({ throwOnError: true });
            // If validation passes, try reading again
            await readAllConfig({ throwOnError: true });
          } catch (validateError) {
            console.log('Config validation failed, attempting recovery...');
            try {
              await recoverConfig({ throwOnError: true });
              await readAllConfig({ throwOnError: true });
            } catch (recoverError) {
              console.warn('Config recovery failed, reinitializing...');
              await initConfig();
            }
          }
        }

        const config = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          try {
            // Initialize system in parallel with cost database (if enabled)
            const initPromises = [
              initializeSystem(provider as string, model as string, {
                getExtensions,
                addExtension,
              }),
            ];

            if (COST_TRACKING_ENABLED) {
              initPromises.push(costDbPromise);
            }

            await Promise.all(initPromises);

            const recipeConfig = window.appConfig.get('recipe');
            if (
              recipeConfig &&
              typeof recipeConfig === 'object' &&
              !window.sessionStorage.getItem('ignoreRecipeConfigChanges')
            ) {
              console.log(
                'Recipe deeplink detected, navigating to pair view with config:',
                recipeConfig
              );
              // Set the recipe config in the pair chat state
              setPairChat((prevChat) => ({
                ...prevChat,
                recipeConfig: recipeConfig as Recipe,
                title: (recipeConfig as Recipe).title || 'Recipe Chat',
                messages: [], // Start fresh for recipe
                messageHistoryIndex: 0,
              }));
              // Navigate to pair view with recipe config using hash routing
              window.location.hash = '#/pair';
              window.history.replaceState(
                {
                  recipeConfig: recipeConfig,
                  resetChat: true,
                },
                '',
                '#/pair'
              );
            } else if (window.sessionStorage.getItem('ignoreRecipeConfigChanges')) {
              console.log(
                'Ignoring recipe config changes to prevent navigation conflicts with new window creation'
              );
            } else {
              // Only navigate to chat route if we're not already on a valid route
              const currentHash = window.location.hash;
              const validRoutes = [
                '#/',
                '#/pair',
                '#/settings',
                '#/sessions',
                '#/schedules',
                '#/recipes',
                '#/permission',
                '#/configure-providers',
                '#/shared-session',
                '#/recipe-editor',
                '#/extensions',
              ];

              if (!validRoutes.includes(currentHash)) {
                console.log('No valid route detected, navigating to chat route (hub)');
                window.location.hash = '#/';
                window.history.replaceState({}, '', '#/');
              }
            }
          } catch (error) {
            console.error('Error in system initialization:', error);
            if (error instanceof MalformedConfigError) {
              throw error;
            }
            window.location.hash = '#/welcome';
            window.history.replaceState({}, '', '#/welcome');
          }
        } else {
          window.location.hash = '#/welcome';
          window.history.replaceState({}, '', '#/welcome');
        }
      } catch (error) {
        console.error('Fatal error during initialization:', error);
        setFatalError(error instanceof Error ? error.message : 'Unknown error occurred');
      }
    };

    initializeApp();
  }, [getExtensions, addExtension, read, setPairChat]);

  useEffect(() => {
    console.log('Sending reactReady signal to Electron');
    try {
      window.electron.reactReady();
    } catch (error) {
      console.error('Error sending reactReady:', error);
      setFatalError(
        `React ready notification failed: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }, []);

  // Handle navigation to pair view for recipe deeplinks after router is ready
  useEffect(() => {
    const recipeConfig = window.appConfig.get('recipe');
    if (
      recipeConfig &&
      typeof recipeConfig === 'object' &&
      window.location.hash === '#/' &&
      !window.sessionStorage.getItem('ignoreRecipeConfigChanges')
    ) {
      console.log('Router ready - navigating to pair view for recipe deeplink:', recipeConfig);
      // Small delay to ensure router is fully initialized
      setTimeout(() => {
        window.location.hash = '#/pair';
      }, 100);
    } else if (window.sessionStorage.getItem('ignoreRecipeConfigChanges')) {
      console.log('Router ready - ignoring recipe config navigation due to new window creation');
    }
  }, []);

  useEffect(() => {
    const handleOpenSharedSession = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const link = args[0] as string;
      window.electron.logInfo(`Opening shared session from deep link ${link}`);
      setIsLoadingSharedSession(true);
      setSharedSessionError(null);
      try {
        await openSharedSessionFromDeepLink(
          link,
          (_view: View, _options?: SessionLinksViewOptions) => {
            // Navigate to shared session view with the session data
            window.location.hash = '#/shared-session';
            if (_options) {
              window.history.replaceState(_options, '', '#/shared-session');
            }
          }
        );
      } catch (error) {
        console.error('Unexpected error opening shared session:', error);
        // Navigate to shared session view with error
        window.location.hash = '#/shared-session';
        const shareToken = link.replace('goose://sessions/', '');
        const options = {
          sessionDetails: null,
          error: error instanceof Error ? error.message : 'Unknown error',
          shareToken,
        };
        window.history.replaceState(options, '', '#/shared-session');
      } finally {
        setIsLoadingSharedSession(false);
      }
    };
    window.electron.on('open-shared-session', handleOpenSharedSession);
    return () => {
      window.electron.off('open-shared-session', handleOpenSharedSession);
    };
  }, [setSharedSessionError]);

  // Handle recipe decode events from main process
  useEffect(() => {
    const handleRecipeDecoded = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const decodedRecipe = args[0] as Recipe;
      console.log('[App] Recipe decoded successfully:', decodedRecipe);

      // Update the pair chat with the decoded recipe
      setPairChat((prevChat) => ({
        ...prevChat,
        recipeConfig: decodedRecipe,
        title: decodedRecipe.title || 'Recipe Chat',
        messages: [], // Start fresh for recipe
        messageHistoryIndex: 0,
      }));

      // Navigate to pair view if not already there
      if (window.location.hash !== '#/pair') {
        window.location.hash = '#/pair';
      }
    };

    const handleRecipeDecodeError = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const errorMessage = args[0] as string;
      console.error('[App] Recipe decode error:', errorMessage);

      // Show error to user - you could add a toast notification here
      // For now, just log the error and navigate to recipes page
      window.location.hash = '#/recipes';
    };

    window.electron.on('recipe-decoded', handleRecipeDecoded);
    window.electron.on('recipe-decode-error', handleRecipeDecodeError);

    return () => {
      window.electron.off('recipe-decoded', handleRecipeDecoded);
      window.electron.off('recipe-decode-error', handleRecipeDecodeError);
    };
  }, [setPairChat]);

  useEffect(() => {
    console.log('Setting up keyboard shortcuts');
    const handleKeyDown = (event: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      if ((isMac ? event.metaKey : event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        try {
          const workingDir = window.appConfig.get('GOOSE_WORKING_DIR');
          console.log(`Creating new chat window with working dir: ${workingDir}`);
          window.electron.createChatWindow(undefined, workingDir as string);
        } catch (error) {
          console.error('Error creating new window:', error);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Prevent default drag and drop behavior globally to avoid opening files in new windows
  // but allow our React components to handle drops in designated areas
  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const preventDefaults = (e: any) => {
      // Only prevent default if we're not over a designated drop zone
      const target = e.target as HTMLElement;
      const isOverDropZone = target.closest('[data-drop-zone="true"]') !== null;

      if (!isOverDropZone) {
        e.preventDefault();
        e.stopPropagation();
      }
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const handleDragOver = (e: any) => {
      // Always prevent default for dragover to allow dropping
      e.preventDefault();
      e.stopPropagation();
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const handleDrop = (e: any) => {
      // Only prevent default if we're not over a designated drop zone
      const target = e.target as HTMLElement;
      const isOverDropZone = target.closest('[data-drop-zone="true"]') !== null;

      if (!isOverDropZone) {
        e.preventDefault();
        e.stopPropagation();
      }
    };

    // Add event listeners to document to catch drag events
    document.addEventListener('dragenter', preventDefaults, false);
    document.addEventListener('dragleave', preventDefaults, false);
    document.addEventListener('dragover', handleDragOver, false);
    document.addEventListener('drop', handleDrop, false);

    return () => {
      document.removeEventListener('dragenter', preventDefaults, false);
      document.removeEventListener('dragleave', preventDefaults, false);
      document.removeEventListener('dragover', handleDragOver, false);
      document.removeEventListener('drop', handleDrop, false);
    };
  }, []);

  useEffect(() => {
    console.log('Setting up fatal error handler');
    const handleFatalError = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const errorMessage = args[0] as string;
      console.error('Encountered a fatal error: ', errorMessage);
      console.error('Is loading session:', isLoadingSession);
      setFatalError(errorMessage);
    };
    window.electron.on('fatal-error', handleFatalError);
    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, [isLoadingSession]);

  useEffect(() => {
    console.log('Setting up view change handler');
    const handleSetView = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const newView = args[0] as View;
      const section = args[1] as string | undefined;
      console.log(
        `Received view change request to: ${newView}${section ? `, section: ${section}` : ''}`
      );

      if (section && newView === 'settings') {
        window.location.hash = `#/settings?section=${section}`;
      } else {
        window.location.hash = `#/${newView}`;
      }
    };
    const urlParams = new URLSearchParams(window.location.search);
    const viewFromUrl = urlParams.get('view');
    if (viewFromUrl) {
      const windowConfig = window.electron.getConfig();
      if (viewFromUrl === 'recipeEditor') {
        const initialViewOptions = {
          recipeConfig: JSON.stringify(windowConfig?.recipeConfig),
          view: viewFromUrl,
        };
        window.history.replaceState(
          {},
          '',
          `/recipe-editor?${new URLSearchParams(initialViewOptions).toString()}`
        );
      } else {
        window.history.replaceState({}, '', `/${viewFromUrl}`);
      }
    }
    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, []);

  const config = window.electron.getConfig();
  const STRICT_ALLOWLIST = config.GOOSE_ALLOWLIST_WARNING !== true;

  useEffect(() => {
    console.log('Setting up extension handler');
    const handleAddExtension = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const link = args[0] as string;
      try {
        console.log(`Received add-extension event with link: ${link}`);
        const command = extractCommand(link);
        const remoteUrl = extractRemoteUrl(link);
        const extName = extractExtensionName(link);
        window.electron.logInfo(`Adding extension from deep link ${link}`);
        setPendingLink(link);
        let warningMessage = '';
        let label = 'OK';
        let title = 'Confirm Extension Installation';
        let isBlocked = false;
        let useDetailedMessage = false;
        if (remoteUrl) {
          useDetailedMessage = true;
        } else {
          try {
            const allowedCommands = await window.electron.getAllowedExtensions();
            if (allowedCommands && allowedCommands.length > 0) {
              const isCommandAllowed = allowedCommands.some((allowedCmd) =>
                command.startsWith(allowedCmd)
              );
              if (!isCommandAllowed) {
                useDetailedMessage = true;
                title = '⛔️ Untrusted Extension ⛔️';
                if (STRICT_ALLOWLIST) {
                  isBlocked = true;
                  label = 'Extension Blocked';
                  warningMessage =
                    '\n\n⛔️ BLOCKED: This extension command is not in the allowed list. ' +
                    'Installation is blocked by your administrator. ' +
                    'Please contact your administrator if you need this extension.';
                } else {
                  label = 'Override and install';
                  warningMessage =
                    '\n\n⚠️ WARNING: This extension command is not in the allowed list. ' +
                    'Installing extensions from untrusted sources may pose security risks. ' +
                    'Please contact an admin if you are unsure or want to allow this extension.';
                }
              }
            }
          } catch (error) {
            console.error('Error checking allowlist:', error);
          }
        }
        if (useDetailedMessage) {
          const detailedMessage = remoteUrl
            ? `You are about to install the ${extName} extension which connects to:\n\n${remoteUrl}\n\nThis extension will be able to access your conversations and provide additional functionality.`
            : `You are about to install the ${extName} extension which runs the command:\n\n${command}\n\nThis extension will be able to access your conversations and provide additional functionality.`;
          setModalMessage(`${detailedMessage}${warningMessage}`);
        } else {
          const messageDetails = `Command: ${command}`;
          setModalMessage(
            `Are you sure you want to install the ${extName} extension?\n\n${messageDetails}`
          );
        }
        setExtensionConfirmLabel(label);
        setExtensionConfirmTitle(title);
        if (isBlocked) {
          setPendingLink(null);
        }
        setModalVisible(true);
      } catch (error) {
        console.error('Error handling add-extension event:', error);
      }
    };
    window.electron.on('add-extension', handleAddExtension);
    return () => {
      window.electron.off('add-extension', handleAddExtension);
    };
  }, [STRICT_ALLOWLIST]);

  useEffect(() => {
    const handleFocusInput = (_event: IpcRendererEvent, ..._args: unknown[]) => {
      const inputField = document.querySelector('input[type="text"], textarea') as HTMLInputElement;
      if (inputField) {
        inputField.focus();
      }
    };
    window.electron.on('focus-input', handleFocusInput);
    return () => {
      window.electron.off('focus-input', handleFocusInput);
    };
  }, []);

  const handleConfirm = async () => {
    if (pendingLink) {
      console.log(`Confirming installation of extension from: ${pendingLink}`);
      setModalVisible(false);
      try {
        await addExtensionFromDeepLinkV2(pendingLink, addExtension, (view: string, options) => {
          console.log('Extension deep link handler called with view:', view, 'options:', options);
          switch (view) {
            case 'settings':
              window.location.hash = '#/extensions';
              // Store the config for the extensions route
              window.history.replaceState(options, '', '#/extensions');
              break;
            default:
              window.location.hash = `#/${view}`;
          }
        });
        console.log('Extension installation successful');
      } catch (error) {
        console.error('Failed to add extension:', error);
      } finally {
        setPendingLink(null);
      }
    } else {
      console.log('Extension installation blocked by allowlist restrictions');
      setModalVisible(false);
    }
  };

  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null);
  };

  if (fatalError) {
    return <ErrorUI error={new Error(fatalError)} />;
  }

  if (isLoadingSession)
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
      </div>
    );

  return (
    <DraftProvider>
      <ModelAndProviderProvider>
        <HashRouter>
          <ToastContainer
            aria-label="Toast notifications"
            toastClassName={() =>
              `relative min-h-16 mb-4 p-2 rounded-lg
               flex justify-between overflow-hidden cursor-pointer
               text-text-on-accent bg-background-inverse
              `
            }
            style={{ width: '380px' }}
            className="mt-6"
            position="top-right"
            autoClose={3000}
            closeOnClick
            pauseOnHover
          />
          {modalVisible && (
            <ConfirmationModal
              isOpen={modalVisible}
              message={modalMessage}
              confirmLabel={extensionConfirmLabel}
              title={extensionConfirmTitle}
              onConfirm={handleConfirm}
              onCancel={handleCancel}
            />
          )}
          <div className="relative w-screen h-screen overflow-hidden bg-background-muted flex flex-col">
            <div className="titlebar-drag-region" />
            <Routes>
              <Route path="welcome" element={<WelcomeRoute />} />
              <Route path="configure-providers" element={<ConfigureProvidersRoute />} />
              <Route
                path="/"
                element={
                  <ChatProvider chat={chat} setChat={setChat} contextKey="hub">
                    <AppLayout setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
                  </ChatProvider>
                }
              >
                <Route
                  index
                  element={
                    <ProviderGuard>
                      <HubRouteWrapper
                        chat={chat}
                        setChat={setChat}
                        setPairChat={setPairChat}
                        setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                      />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="pair"
                  element={
                    <ProviderGuard>
                      <ChatProvider
                        chat={pairChat}
                        setChat={setPairChat}
                        contextKey={`pair-${pairChat.id}`}
                        key={pairChat.id} // Add key prop to force re-render when chat ID changes
                      >
                        <PairRouteWrapper
                          chat={pairChat}
                          setChat={setPairChat}
                          setPairChat={setPairChat}
                          setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                        />
                      </ChatProvider>
                    </ProviderGuard>
                  }
                />
                <Route
                  path="settings"
                  element={
                    <ProviderGuard>
                      <SettingsRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="extensions"
                  element={
                    <ProviderGuard>
                      <ExtensionsRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="sessions"
                  element={
                    <ProviderGuard>
                      <SessionsRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="schedules"
                  element={
                    <ProviderGuard>
                      <SchedulesRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="recipes"
                  element={
                    <ProviderGuard>
                      <RecipesRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="recipe-editor"
                  element={
                    <ProviderGuard>
                      <RecipeEditorRoute />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="shared-session"
                  element={
                    <ProviderGuard>
                      <SharedSessionRouteWrapper
                        isLoadingSharedSession={isLoadingSharedSession}
                        setIsLoadingSharedSession={setIsLoadingSharedSession}
                        sharedSessionError={sharedSessionError}
                      />
                    </ProviderGuard>
                  }
                />
                <Route
                  path="permission"
                  element={
                    <ProviderGuard>
                      <PermissionRoute />
                    </ProviderGuard>
                  }
                />
                {/*<Route*/}
                {/*  path="projects"*/}
                {/*  element={*/}
                {/*    <ProviderGuard>*/}
                {/*      <ChatProvider chat={chat} setChat={setChat}>*/}
                {/*        <ProjectsRoute />*/}
                {/*      </ChatProvider>*/}
                {/*    </ProviderGuard>  */}
                {/*  }*/}
                {/*/>*/}
              </Route>
            </Routes>
          </div>
          {isGoosehintsModalOpen && (
            <GoosehintsModal
              directory={window.appConfig.get('GOOSE_WORKING_DIR') as string}
              setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            />
          )}
        </HashRouter>
        <AnnouncementModal />
      </ModelAndProviderProvider>
    </DraftProvider>
  );
}
