import { useCallback, useEffect, useState } from 'react';
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
import ProviderGuard from './components/ProviderGuard';
import { createSession } from './sessions';

import { ChatType } from './types/chat';
import Hub from './components/Hub';
import { PairRouteState } from './components/Pair';
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
import { SessionStatusProvider } from './contexts/SessionStatusContext';

// Route Components
const HubRouteWrapper = ({ isExtensionsLoading }: { isExtensionsLoading: boolean }) => {
  const setView = useNavigation();

  return <Hub setView={setView} isExtensionsLoading={isExtensionsLoading} />;
};

const PairRouteWrapper = ({
  activeSessions,
  setActiveSessions,
}: {
  activeSessions: Array<{ sessionId: string; initialMessage?: string }>;
  setActiveSessions: (sessions: Array<{ sessionId: string; initialMessage?: string }>) => void;
}) => {
  const location = useLocation();
  const routeState =
    (location.state as PairRouteState) || (window.history.state as PairRouteState) || {};
  const [searchParams, setSearchParams] = useSearchParams();
  const [isCreatingSession, setIsCreatingSession] = useState(false);

  const resumeSessionId = searchParams.get('resumeSessionId') ?? undefined;
  const recipeId = searchParams.get('recipeId') ?? undefined;
  const recipeDeeplinkFromConfig = window.appConfig?.get('recipeDeeplink') as string | undefined;
  const initialMessage = routeState.initialMessage;

  useEffect(() => {
    // Create a new session if we have an initialMessage, recipeId, or recipeDeeplink from config but no sessionId
    if (
      (initialMessage || recipeId || recipeDeeplinkFromConfig) &&
      !resumeSessionId &&
      !isCreatingSession
    ) {
      console.log(
        '[PairRouteWrapper] Creating new session for initialMessage, recipeId, or recipeDeeplink from config',
        { initialMessage, recipeId, recipeDeeplinkFromConfig }
      );
      setIsCreatingSession(true);

      (async () => {
        try {
          const newSession = await createSession({
            recipeId,
            recipeDeeplink: recipeDeeplinkFromConfig,
          });

          // Add to active sessions
          // Only new sessions should have initialMessage
          setActiveSessions([
            ...activeSessions,
            {
              sessionId: newSession.id,
              initialMessage, // Only for new sessions
            },
          ]);

          setSearchParams((prev) => {
            prev.set('resumeSessionId', newSession.id);
            prev.delete('recipeId');
            return prev;
          });
        } catch (error) {
          console.error('[PairRouteWrapper] Failed to create session:', error);
        } finally {
          setIsCreatingSession(false);
        }
      })();
    }
  }, [
    initialMessage,
    recipeId,
    recipeDeeplinkFromConfig,
    resumeSessionId,
    isCreatingSession,
    setSearchParams,
    activeSessions,
    setActiveSessions,
  ]);

  // Add resumed session to active sessions if not already there
  useEffect(() => {
    if (resumeSessionId && !activeSessions.some((s) => s.sessionId === resumeSessionId)) {
      const resumedSession = {
        sessionId: resumeSessionId,
        // Pass initialMessage from route state if it exists (e.g., for forked sessions)
        // BaseChat will determine whether to submit it based on session state
        initialMessage: initialMessage,
      };

      setActiveSessions([...activeSessions, resumedSession]);
    }
  }, [resumeSessionId, activeSessions, setActiveSessions, initialMessage]);

  return null;
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
  const onClose = useCallback(() => {
    onSelectProvider();
    navigate('/');
  }, [navigate, onSelectProvider]);

  return (
    <div className="w-screen h-screen bg-background-default">
      <ProviderSettings onClose={onClose} isOnboarding={true} />
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
  const [isExtensionsLoading, setIsExtensionsLoading] = useState(false);
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

  const [activeSessions, setActiveSessions] = useState<
    Array<{ sessionId: string; initialMessage?: string }>
  >([]);

  useEffect(() => {
    const handleAddActiveSession = (event: CustomEvent) => {
      const { sessionId, initialMessage } = event.detail;

      setActiveSessions((prev) => {
        const existingSession = prev.find((s) => s.sessionId === sessionId);
        if (existingSession) {
          return prev;
        }

        const newSession = { sessionId, initialMessage };

        return [...prev, newSession];
      });
    };

    const handleClearInitialMessage = (event: CustomEvent) => {
      const { sessionId } = event.detail;

      setActiveSessions((prev) => {
        return prev.map((session) => {
          if (session.sessionId === sessionId) {
            // Clear the initial message after it's been submitted
            return { ...session, initialMessage: undefined };
          }
          return session;
        });
      });
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    window.addEventListener('add-active-session', handleAddActiveSession as any);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    window.addEventListener('clear-initial-message', handleClearInitialMessage as any);
    return () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      window.removeEventListener('add-active-session', handleAddActiveSession as any);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      window.removeEventListener('clear-initial-message', handleClearInitialMessage as any);
    };
  }, []);

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
            setIsExtensionsLoading,
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
          const workingDir = window.appConfig?.get('GOOSE_WORKING_DIR');
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
    const handleSetInitialMessage = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const initialMessage = args[0] as string;
      if (initialMessage) {
        console.log('Received initial message from launcher:', initialMessage);
        navigate('/pair', { state: { initialMessage } });
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
    <SessionStatusProvider>
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
        <div style={{ position: 'relative', width: '100%', height: '100%' }}>
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
                    <AppLayout activeSessions={activeSessions} />
                  </ChatProvider>
                </ProviderGuard>
              }
            >
              <Route
                index
                element={<HubRouteWrapper isExtensionsLoading={isExtensionsLoading} />}
              />
              <Route
                path="pair"
                element={
                  <PairRouteWrapper
                    activeSessions={activeSessions}
                    setActiveSessions={setActiveSessions}
                  />
                }
              />
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
      </div>
    </SessionStatusProvider>
  );
}

export default function App() {
  return (
    <ModelAndProviderProvider>
      <HashRouter>
        <AppInner />
      </HashRouter>
      <AnnouncementModal />
    </ModelAndProviderProvider>
  );
}
