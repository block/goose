import { useEffect, useState } from 'react';
import { IpcRendererEvent } from 'electron';
import {
  HashRouter,
  Routes,
  Route,
  useNavigate,
  useLocation,
  useSearchParams,
} from 'react-router-dom';
import { openSharedSessionFromDeepLink } from './sessionLinks';
import { type SharedSessionDetails } from './sharedSessions';
import { ErrorUI } from './components/ErrorBoundary';
import { ExtensionInstallModal } from './components/ExtensionInstallModal';
import { ToastContainer } from 'react-toastify';
import AnnouncementModal from './components/AnnouncementModal';
import TelemetryOptOutModal from './components/TelemetryOptOutModal';
import ProviderGuard from './components/ProviderGuard';
import { createSession } from './sessions';

import { ChatType } from './types/chat';
import Hub from './components/Hub';
import Pair, { PairRouteState } from './components/Pair';
import SettingsView, { SettingsViewOptions } from './components/settings/SettingsView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import SchedulesView from './components/schedule/SchedulesView';
import ProviderSettings from './components/settings/providers/ProviderSettingsPage';
import { AppLayout } from './components/Layout/AppLayout';
import { ChatProvider } from './contexts/ChatContext';
import LauncherView from './components/LauncherView';

import 'react-toastify/dist/ReactToastify.css';
import { useConfig } from './components/ConfigContext';
import { ModelAndProviderProvider } from './components/ModelAndProviderContext';
import PermissionSettingsView from './components/settings/permission/PermissionSetting';

import ExtensionsView, { ExtensionsViewOptions } from './components/extensions/ExtensionsView';
import RecipesView from './components/recipes/RecipesView';
import { View, ViewOptions } from './utils/navigationUtils';
import { NoProviderOrModelError, useAgent } from './hooks/useAgent';
import { useNavigation } from './hooks/useNavigation';
import { errorMessage } from './utils/conversionUtils';
import { getInitialWorkingDir } from './utils/workingDir';
import { usePageViewTracking } from './hooks/useAnalytics';
import { trackOnboardingCompleted } from './utils/analytics';

function PageViewTracker() {
  usePageViewTracking();
  return null;
}

// Route Components
const HubRouteWrapper = () => {
  const setView = useNavigation();
  return <Hub setView={setView} />;
};

const PairRouteWrapper = ({
  chat,
  setChat,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
}) => {
  const { extensionsList } = useConfig();
  const location = useLocation();
  const navigate = useNavigate();
  const routeState = (location.state as PairRouteState) || {};
  const [searchParams] = useSearchParams();
  const resumeSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const recipeId = searchParams.get('recipeId') ?? undefined;
  const recipeDeeplinkFromConfig = window.appConfig?.get('recipeDeeplink') as string | undefined;

  // Session ID and initialMessage come from route state (Hub, fork) or URL params (refresh, deeplink)
  const sessionIdFromState = routeState.resumeSessionId;
  const initialMessage = routeState.initialMessage;
  const sessionId = sessionIdFromState || resumeSessionId || chat.sessionId || undefined;

  // Handle recipe deeplinks - create session if needed
  useEffect(() => {
    if ((recipeId || recipeDeeplinkFromConfig) && !sessionId) {
      (async () => {
        try {
          const newSession = await createSession(getInitialWorkingDir(), {
            recipeId,
            recipeDeeplink: recipeDeeplinkFromConfig,
            allExtensions: extensionsList,
          });
          navigate(`/pair?resumeSessionId=${newSession.id}`, {
            replace: true,
            state: { resumeSessionId: newSession.id },
          });
        } catch (error) {
          console.error('Failed to create session for recipe:', error);
        }
      })();
    }
  }, [recipeId, recipeDeeplinkFromConfig, sessionId, extensionsList, navigate]);

  // Sync URL with session ID for refresh support (only if not already in URL)
  useEffect(() => {
    if (sessionId && sessionId !== resumeSessionId) {
      navigate(`/pair?resumeSessionId=${sessionId}`, {
        replace: true,
        state: { resumeSessionId: sessionIdFromState, initialMessage }, // Preserve state
      });
    }
  }, [sessionId, resumeSessionId, navigate, sessionIdFromState, initialMessage]);

  return (
    <Pair
      key={sessionId}
      setChat={setChat}
      sessionId={sessionId ?? ''}
      initialMessage={initialMessage}
    />
  );
};

const SettingsRoute = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const setView = useNavigation();

  // Get viewOptions from location.state, history.state, or URL search params
  const viewOptions =
    (location.state as SettingsViewOptions) || (window.history.state as SettingsViewOptions) || {};

  // If section is provided via URL search params, add it to viewOptions
  const sectionFromUrl = searchParams.get('section');
  if (sectionFromUrl) {
    viewOptions.section = sectionFromUrl;
  }

  return <SettingsView onClose={() => navigate('/')} setView={setView} viewOptions={viewOptions} />;
};

const SessionsRoute = () => {
  return <SessionsView />;
};

const SchedulesRoute = () => {
  const navigate = useNavigate();
  return <SchedulesView onClose={() => navigate('/')} />;
};

const RecipesRoute = () => {
  return <RecipesView />;
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

interface WelcomeRouteProps {
  onSelectProvider: () => void;
}

const WelcomeRoute = ({ onSelectProvider }: WelcomeRouteProps) => {
  const navigate = useNavigate();

  return (
    <div className="w-screen h-screen bg-background-default">
      <ProviderSettings
        onClose={() => {
          navigate('/', { replace: true });
        }}
        isOnboarding={true}
        onProviderLaunched={(model?: string) => {
          trackOnboardingCompleted('other', model);
          onSelectProvider();
          navigate('/', { replace: true });
        }}
      />
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
  const setView = useNavigation();

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
            await openSharedSessionFromDeepLink(`goose://sessions/${shareToken}`, setView, baseUrl);
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

export function AppInner() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [agentWaitingMessage, setAgentWaitingMessage] = useState<string | null>(null);
  const [isLoadingSharedSession, setIsLoadingSharedSession] = useState(false);
  const [sharedSessionError, setSharedSessionError] = useState<string | null>(null);
  const [didSelectProvider, setDidSelectProvider] = useState<boolean>(false);

  const navigate = useNavigate();
  const setView = useNavigation();
  const location = useLocation();

  const [chat, setChat] = useState<ChatType>({
    sessionId: '',
    name: 'Pair Chat',
    messages: [],
    messageHistoryIndex: 0,
    recipe: null,
  });

  const { addExtension } = useConfig();
  const { loadCurrentChat } = useAgent();

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

  // Handle URL parameters and deeplinks on app startup
  const loadingHub = location.pathname === '/';
  useEffect(() => {
    if (loadingHub) {
      (async () => {
        try {
          const loadedChat = await loadCurrentChat({
            setAgentWaitingMessage,
          });
          setChat(loadedChat);
        } catch (e) {
          if (e instanceof NoProviderOrModelError) {
            // the onboarding flow will trigger
          } else {
            throw e;
          }
        }
      })();
    }
  }, [loadCurrentChat, setAgentWaitingMessage, navigate, loadingHub, setChat]);

  useEffect(() => {
    const handleOpenSharedSession = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const link = args[0] as string;
      window.electron.logInfo(`Opening shared session from deep link ${link}`);
      setIsLoadingSharedSession(true);
      setSharedSessionError(null);
      try {
        await openSharedSessionFromDeepLink(link, (_view: View, options?: ViewOptions) => {
          navigate('/shared-session', { state: options });
        });
      } catch (error) {
        console.error('Unexpected error opening shared session:', error);
        // Navigate to shared session view with error
        const shareToken = link.replace('goose://sessions/', '');
        const options = {
          sessionDetails: null,
          error: error instanceof Error ? error.message : 'Unknown error',
          shareToken,
        };
        navigate('/shared-session', { state: options });
      } finally {
        setIsLoadingSharedSession(false);
      }
    };
    window.electron.on('open-shared-session', handleOpenSharedSession);
    return () => {
      window.electron.off('open-shared-session', handleOpenSharedSession);
    };
  }, [navigate]);

  useEffect(() => {
    console.log('Setting up keyboard shortcuts');
    const handleKeyDown = (event: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';
      if ((isMac ? event.metaKey : event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        try {
          window.electron.createChatWindow(undefined, getInitialWorkingDir());
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
    const preventDefaults = (e: globalThis.DragEvent) => {
      // Only prevent default if we're not over a designated drop zone
      const target = e.target as HTMLElement;
      const isOverDropZone = target.closest('[data-drop-zone="true"]') !== null;

      if (!isOverDropZone) {
        e.preventDefault();
        e.stopPropagation();
      }
    };

    const handleDragOver = (e: globalThis.DragEvent) => {
      // Always prevent default for dragover to allow dropping
      e.preventDefault();
      e.stopPropagation();
    };

    const handleDrop = (e: globalThis.DragEvent) => {
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
    const handleFatalError = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const errorMessage = args[0] as string;
      console.error('Encountered a fatal error:', errorMessage);
      setFatalError(errorMessage);
    };
    window.electron.on('fatal-error', handleFatalError);
    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, []);

  useEffect(() => {
    const handleSetView = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const newView = args[0] as View;
      const section = args[1] as string | undefined;
      console.log(
        `Received view change request to: ${newView}${section ? `, section: ${section}` : ''}`
      );

      if (section && newView === 'settings') {
        navigate(`/settings?section=${section}`);
      } else {
        navigate(`/${newView}`);
      }
    };

    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, [navigate]);

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

  useEffect(() => {
    if (!window.electron) return;

    const handleThemeChanged = (_event: unknown, ...args: unknown[]) => {
      const themeData = args[0] as { mode: string; useSystemTheme: boolean; theme: string };

      if (themeData.useSystemTheme) {
        localStorage.setItem('use_system_theme', 'true');
      } else {
        localStorage.setItem('use_system_theme', 'false');
        localStorage.setItem('theme', themeData.theme);
      }

      const isDark = themeData.useSystemTheme
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
        : themeData.mode === 'dark';

      if (isDark) {
        document.documentElement.classList.add('dark');
        document.documentElement.classList.remove('light');
      } else {
        document.documentElement.classList.remove('dark');
        document.documentElement.classList.add('light');
      }

      const storageEvent = new Event('storage') as Event & {
        key: string | null;
        newValue: string | null;
      };
      storageEvent.key = themeData.useSystemTheme ? 'use_system_theme' : 'theme';
      storageEvent.newValue = themeData.useSystemTheme ? 'true' : themeData.theme;
      window.dispatchEvent(storageEvent);
    };

    window.electron.on('theme-changed', handleThemeChanged);

    return () => {
      window.electron.off('theme-changed', handleThemeChanged);
    };
  }, []);

  // Handle initial message from launcher
  useEffect(() => {
    const handleSetInitialMessage = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const initialMessage = args[0] as string;
      if (initialMessage) {
        console.log('Received initial message from launcher:', initialMessage);
        try {
          const session = await createSession(getInitialWorkingDir(), {});
          navigate('/pair', {
            state: {
              initialMessage,
              resumeSessionId: session.id,
            },
          });
        } catch (error) {
          console.error('Failed to create session for launcher message:', error);
        }
      }
    };
    window.electron.on('set-initial-message', handleSetInitialMessage);
    return () => {
      window.electron.off('set-initial-message', handleSetInitialMessage);
    };
  }, [navigate]);

  if (fatalError) {
    return <ErrorUI error={errorMessage(fatalError)} />;
  }

  return (
    <>
      <PageViewTracker />
      <ToastContainer
        aria-label="Toast notifications"
        toastClassName={() =>
          `relative min-h-16 mb-4 p-2 rounded-lg
               flex justify-between overflow-hidden cursor-pointer
               text-text-on-accent bg-background-inverse
              `
        }
        style={{ width: '450px' }}
        className="mt-6"
        position="top-right"
        autoClose={3000}
        closeOnClick
        pauseOnHover
      />
      <ExtensionInstallModal addExtension={addExtension} setView={setView} />
      <div className="relative w-screen h-screen overflow-hidden bg-background-muted flex flex-col">
        <div className="titlebar-drag-region" />
        <Routes>
          <Route path="launcher" element={<LauncherView />} />
          <Route
            path="welcome"
            element={<WelcomeRoute onSelectProvider={() => setDidSelectProvider(true)} />}
          />
          <Route path="configure-providers" element={<ConfigureProvidersRoute />} />
          <Route
            path="/"
            element={
              <ProviderGuard didSelectProvider={didSelectProvider}>
                <ChatProvider
                  chat={chat}
                  setChat={setChat}
                  contextKey="hub"
                  agentWaitingMessage={agentWaitingMessage}
                >
                  <AppLayout />
                </ChatProvider>
              </ProviderGuard>
            }
          >
            <Route index element={<HubRouteWrapper />} />
            <Route path="pair" element={<PairRouteWrapper chat={chat} setChat={setChat} />} />
            <Route path="settings" element={<SettingsRoute />} />
            <Route
              path="extensions"
              element={
                <ChatProvider
                  chat={chat}
                  setChat={setChat}
                  contextKey="extensions"
                  agentWaitingMessage={agentWaitingMessage}
                >
                  <ExtensionsRoute />
                </ChatProvider>
              }
            />
            <Route path="sessions" element={<SessionsRoute />} />
            <Route path="schedules" element={<SchedulesRoute />} />
            <Route path="recipes" element={<RecipesRoute />} />
            <Route
              path="shared-session"
              element={
                <SharedSessionRouteWrapper
                  isLoadingSharedSession={isLoadingSharedSession}
                  setIsLoadingSharedSession={setIsLoadingSharedSession}
                  sharedSessionError={sharedSessionError}
                />
              }
            />
            <Route path="permission" element={<PermissionRoute />} />
          </Route>
        </Routes>
      </div>
    </>
  );
}

export default function App() {
  return (
    <ModelAndProviderProvider>
      <HashRouter>
        <AppInner />
      </HashRouter>
      <AnnouncementModal />
      <TelemetryOptOutModal controlled={false} />
    </ModelAndProviderProvider>
  );
}
