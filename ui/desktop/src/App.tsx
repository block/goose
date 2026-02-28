import type { IpcRendererEvent } from 'electron';
import { useEffect, useRef, useState } from 'react';
import {
  HashRouter,
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
  useParams,
} from 'react-router-dom';
import { ToastContainer } from 'react-toastify';
import { AuthGuard } from '@/components/organisms/guards/AuthGuard';
import ProviderGuard from '@/components/organisms/guards/ProviderGuard';
import AnnouncementModal from '@/components/organisms/modals/AnnouncementModal';
import { ExtensionInstallModal } from '@/components/organisms/modals/ExtensionInstallModal';
import TelemetryOptOutModal from '@/components/organisms/modals/TelemetryOptOutModal';
import LoginView from '@/components/pages/LoginView';
import AgentsPage from '@/components/pages/AgentsPage';
import AppsPage from '@/components/pages/AppsPage';
import CatalogsPage from '@/components/pages/CatalogsPage';
import EvaluatePage from '@/components/pages/EvaluatePage';
import MonitoringPage from '@/components/pages/MonitoringPage';
import PipelinesPage from '@/components/pages/PipelinesPage';
import RecipesPage from '@/components/pages/RecipesPage';
import SchedulesPage from '@/components/pages/SchedulesPage';
import SessionsPage from '@/components/pages/SessionsPage';
import ToolsPage from '@/components/pages/ToolsPage';
import WorkflowsPage from '@/components/pages/WorkflowsPage';
import WelcomePage from '@/components/pages/WelcomePage';
import { ErrorUI } from '@/components/organisms/shared/ErrorBoundary';
import { TooltipProvider } from '@/components/atoms/tooltip';
import { AuthProvider } from '@/hooks/useAuth';
import { setupAuthInterceptor } from '@/lib/authInterceptor';
import { openSharedSessionFromDeepLink } from './sessionLinks';
import { createSession } from '@/sessions';
import type { SharedSessionDetails } from './sharedSessions';

// Initialize auth interceptor before any API calls — attaches
// the Bearer token from localStorage to every outgoing request
setupAuthInterceptor();

import type { ChatType } from '@/types/chat';
import type { UserInput } from '@/types/message';

interface SessionRouteState {
  initialMessage?: UserInput;
  shouldStartAgent?: boolean;
}

import { AppLayout } from '@/components/templates/layout/AppLayout';
import LauncherView from '@/components/pages/LauncherView';
import SharedSessionView from '@/components/organisms/sessions/SharedSessionView';
import ProviderSettings from '@/components/organisms/settings/providers/ProviderSettingsPage';
import type { SettingsViewOptions } from '@/components/organisms/settings/SettingsView';
import SettingsView from '@/components/organisms/settings/SettingsView';
import { ChatProvider, DEFAULT_CHAT_TITLE } from '@/contexts/ChatContext';

import 'react-toastify/dist/ReactToastify.css';
import StandaloneAppView from '@/components/organisms/apps/StandaloneAppView';
import type { ExtensionsViewOptions } from '@/components/organisms/extensions/ExtensionsView';
import ExtensionsView from '@/components/organisms/extensions/ExtensionsView';
import PermissionSettingsView from '@/components/organisms/settings/permission/PermissionSetting';
import { AppEvents } from '@/constants/events';
import { useConfig } from '@/contexts/ConfigContext';
import { ModelAndProviderProvider } from '@/contexts/ModelAndProviderContext';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { usePageViewTracking } from '@/hooks/useAnalytics';
import { useNavigation } from '@/hooks/useNavigation';
import { trackErrorWithContext } from '@/utils/analytics';
import { errorMessage } from '@/utils/conversionUtils';
import type { View, ViewOptions } from '@/utils/navigationUtils';
import { registerPlatformEventHandlers } from '@/utils/platform_events';
import { getInitialWorkingDir } from '@/utils/workingDir';

function PageViewTracker() {
  usePageViewTracking();
  return null;
}

// Route Components
// "/" redirects to "/sessions".
// With no sessions, ChatSessionsContainer shows WelcomeState.
const HomeRedirectWrapper = () => {
  return <Navigate to="/sessions" replace />;
};

const SessionRouteWrapper = ({
  activeSessions,
}: {
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
  setActiveSessions: (sessions: Array<{ sessionId: string; initialMessage?: UserInput }>) => void;
}) => {
  const { extensionsList } = useConfig();
  const location = useLocation();
  const { sessionId: sessionIdParam } = useParams();

  const routeState =
    (location.state as SessionRouteState) || (window.history.state as SessionRouteState) || {};

  const [isCreatingSession, setIsCreatingSession] = useState(false);

  const resumeSessionId = sessionIdParam ? decodeURIComponent(sessionIdParam) : undefined;
  const recipeDeeplinkFromConfig = window.appConfig?.get('recipeDeeplink') as string | undefined;
  const initialMessage = routeState.initialMessage;

  // Only create a session if we have an initial message (launcher) or a recipe deeplink,
  // and we are not already on a specific session route.
  useEffect(() => {
    if ((initialMessage || recipeDeeplinkFromConfig) && !resumeSessionId && !isCreatingSession) {
      setIsCreatingSession(true);

      (async () => {
        try {
          const newSession = await createSession(getInitialWorkingDir(), {
            recipeDeeplink: recipeDeeplinkFromConfig,
            allExtensions: extensionsList,
          });

          window.dispatchEvent(
            new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
              detail: {
                sessionId: newSession.id,
                initialMessage,
              },
            })
          );

          // Navigate to the new session URL.
          // IMPORTANT: use the router's navigate() rather than window.history.replaceState.
          // replaceState does not notify React Router, which can leave the UI in a blank state.
          navigate(`/sessions/${encodeURIComponent(newSession.id)}`, { replace: true });
        } catch (error) {
          console.error('Failed to create session:', error);
          trackErrorWithContext(error, {
            component: 'SessionRouteWrapper',
            action: 'create_session',
            recoverable: true,
          });
        } finally {
          setIsCreatingSession(false);
        }
      })();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialMessage, recipeDeeplinkFromConfig, resumeSessionId, extensionsList, isCreatingSession]);

  // Add resumed session to active sessions if not already there
  useEffect(() => {
    const sessions = activeSessions ?? [];

    if (resumeSessionId && !sessions.some((s) => s.sessionId === resumeSessionId)) {
      window.dispatchEvent(
        new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
          detail: {
            sessionId: resumeSessionId,
            initialMessage,
          },
        })
      );
    }
  }, [resumeSessionId, activeSessions, initialMessage]);

  return null;
};

const SettingsRoute = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const setView = useNavigation();

  // Get viewOptions from location.state or history.state
  const viewOptions =
    (location.state as SettingsViewOptions) || (window.history.state as SettingsViewOptions) || {};

  return <SettingsView onClose={() => navigate('/')} setView={setView} viewOptions={viewOptions} />;
};

const SessionsRoute = () => {
  return <SessionsPage />;
};

const SchedulesRoute = () => {
  return <SchedulesPage />;
};

const RecipesRoute = () => {
  return <RecipesPage />;
};

const AgentsRoute = () => {
  return <AgentsPage />;
};

const AnalyticsRoute = () => {
  return <MonitoringPage />;
};

const MonitoringRoute = () => {
  return <MonitoringPage />;
};

const EvaluateRoute = () => {
  return <EvaluatePage />;
};

const ToolsRoute = () => {
  return <ToolsPage />;
};

const CatalogsRoute = () => {
  return <CatalogsPage />;
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
          case 'session':
            if (parentViewOptions?.resumeSessionId) {
              navigate(`/sessions/${encodeURIComponent(parentViewOptions.resumeSessionId)}`, {
                state: parentViewOptions,
              });
            } else {
              navigate('/sessions', { state: parentViewOptions });
            }
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
    <WelcomePage
      onComplete={() => {
        onSelectProvider();
        navigate('/', { replace: true });
      }}
    />
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
      setView={setView}
      viewOptions={viewOptions}
    />
  );
};

export function AppInner() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [isLoadingSharedSession, setIsLoadingSharedSession] = useState(false);
  const [sharedSessionError, setSharedSessionError] = useState<string | null>(null);
  const [didSelectProvider, setDidSelectProvider] = useState<boolean>(false);

  const navigate = useNavigate();
  const setView = useNavigation();
  const location = useLocation();

  const [chat, setChat] = useState<ChatType>({
    sessionId: '',
    name: DEFAULT_CHAT_TITLE,
    messages: [],
    recipe: null,
  });

  const MAX_ACTIVE_SESSIONS = 10;

  const [activeSessions, setActiveSessions] = useState<
    Array<{ sessionId: string; initialMessage?: UserInput }>
  >([]);

  useEffect(() => {
    const handleAddActiveSession = (event: Event) => {
      const { sessionId, initialMessage } = (
        event as CustomEvent<{
          sessionId: string;
          initialMessage?: UserInput;
        }>
      ).detail;

      setActiveSessions((prev) => {
        const existingIndex = prev.findIndex((s) => s.sessionId === sessionId);

        if (existingIndex !== -1) {
          // Session exists - move to end of LRU list (most recently used)
          const existing = prev[existingIndex];
          return [...prev.slice(0, existingIndex), ...prev.slice(existingIndex + 1), existing];
        }

        // New session - add to end with LRU eviction if needed
        const newSession = { sessionId, initialMessage };
        const updated = [...prev, newSession];
        if (updated.length > MAX_ACTIVE_SESSIONS) {
          return updated.slice(updated.length - MAX_ACTIVE_SESSIONS);
        }
        return updated;
      });
    };

    const handleClearInitialMessage = (event: Event) => {
      const { sessionId } = (event as CustomEvent<{ sessionId: string }>).detail;

      setActiveSessions((prev) => {
        return prev.map((session) => {
          if (session.sessionId === sessionId) {
            return { ...session, initialMessage: undefined };
          }
          return session;
        });
      });
    };

    const handleSessionDeleted = (event: Event) => {
      const { sessionId } = (event as CustomEvent<{ sessionId: string }>).detail;
      setActiveSessions((prev) => prev.filter((s) => s.sessionId !== sessionId));
      setChat((prev) => {
        if (prev.sessionId === sessionId) {
          return { sessionId: '', name: DEFAULT_CHAT_TITLE, messages: [], recipe: null };
        }
        return prev;
      });
    };

    window.addEventListener(AppEvents.ADD_ACTIVE_SESSION, handleAddActiveSession);
    window.addEventListener(AppEvents.CLEAR_INITIAL_MESSAGE, handleClearInitialMessage);
    window.addEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
    return () => {
      window.removeEventListener(AppEvents.ADD_ACTIVE_SESSION, handleAddActiveSession);
      window.removeEventListener(AppEvents.CLEAR_INITIAL_MESSAGE, handleClearInitialMessage);
      window.removeEventListener(AppEvents.SESSION_DELETED, handleSessionDeleted);
    };
  }, []);

  const { addExtension } = useConfig();

  useEffect(() => {
    try {
      window.electron.reactReady();
    } catch (error) {
      console.error('Error sending reactReady:', error);
      setFatalError(`React ready notification failed: ${errorMessage(error, 'Unknown error')}`);
    }
  }, []);

  // If the user is currently viewing a deleted session route, navigate back to /sessions
  // to avoid follow-up 404s and a blank state.
  useEffect(() => {
    if (!location.pathname.startsWith('/sessions/')) return;
    const maybeId = location.pathname.slice('/sessions/'.length);
    if (!maybeId) return;

    const stillActive = activeSessions.some((s) => s.sessionId === maybeId);
    if (!stillActive) {
      navigate('/sessions', { replace: true });
    }
  }, [activeSessions, location.pathname, navigate]);

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
        trackErrorWithContext(error, {
          component: 'AppInner',
          action: 'open_shared_session',
          recoverable: true,
        });
        // Navigate to shared session view with error
        const shareToken = link.replace('goose://sessions/', '');
        const options = {
          sessionDetails: null,
          error: errorMessage(error, 'Unknown error'),
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
    const handleNewChat = (_event: IpcRendererEvent, ..._args: unknown[]) => {
      window.dispatchEvent(new CustomEvent(AppEvents.TRIGGER_NEW_CHAT));
    };

    window.electron.on('new-chat', handleNewChat);
    return () => window.electron.off('new-chat', handleNewChat);
  }, []);

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

  // Handle initial message from launcher
  const isProcessingRef = useRef(false);

  useEffect(() => {
    const handleSetInitialMessage = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const initialMessage = args[0] as string;

      if (initialMessage && !isProcessingRef.current) {
        isProcessingRef.current = true;
        navigate('/sessions', {
          state: {
            initialMessage: { msg: initialMessage, images: [] },
          },
        });
        setTimeout(() => {
          isProcessingRef.current = false;
        }, 1000);
      } else if (initialMessage) {
        console.debug('[App] Ignoring duplicate initial message (already processing)');
      }
    };
    window.electron.on('set-initial-message', handleSetInitialMessage);
    return () => {
      window.electron.off('set-initial-message', handleSetInitialMessage);
    };
  }, [navigate]);

  // Register platform event handlers for app lifecycle management
  useEffect(() => {
    return registerPlatformEventHandlers();
  }, []);

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
        <div style={{ position: 'relative', width: '100%', height: '100%' }}>
          <Routes>
            <Route path="launcher" element={<LauncherView />} />
            <Route path="login" element={<LoginView />} />
            <Route
              path="welcome"
              element={<WelcomeRoute onSelectProvider={() => setDidSelectProvider(true)} />}
            />
            <Route path="configure-providers" element={<ConfigureProvidersRoute />} />
            <Route path="standalone-app" element={<StandaloneAppView />} />
            <Route
              path="/"
              element={
                <AuthGuard>
                  <ProviderGuard didSelectProvider={didSelectProvider}>
                    <ChatProvider chat={chat} setChat={setChat} contextKey="hub">
                      <AppLayout activeSessions={activeSessions} />
                    </ChatProvider>
                  </ProviderGuard>
                </AuthGuard>
              }
            >
              <Route index element={<HomeRedirectWrapper />} />
              <Route path="settings" element={<SettingsRoute />} />
              <Route
                path="extensions"
                element={
                  <ChatProvider chat={chat} setChat={setChat} contextKey="extensions">
                    <ExtensionsRoute />
                  </ChatProvider>
                }
              />
              <Route path="apps" element={<AppsPage />} />
              <Route path="sessions" element={<SessionsRoute />} />
              <Route path="sessions/:sessionId" element={<SessionRouteWrapper />} />
              <Route path="schedules" element={<SchedulesRoute />} />
              <Route path="workflows" element={<WorkflowsPage />} />
              <Route path="recipes" element={<RecipesRoute />} />
              <Route path="agents" element={<AgentsRoute />} />
              <Route path="pipelines" element={<PipelinesPage />} />
              <Route path="analytics" element={<AnalyticsRoute />} />
              <Route path="monitoring" element={<MonitoringRoute />} />
              <Route path="evaluate" element={<EvaluateRoute />} />
              <Route path="tools" element={<ToolsRoute />} />
              <Route path="catalogs" element={<CatalogsRoute />} />
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
    </>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <ModelAndProviderProvider>
        <TooltipProvider>
          <HashRouter>
            <AuthProvider>
              <AppInner />
            </AuthProvider>
          </HashRouter>
          <AnnouncementModal />
          <TelemetryOptOutModal controlled={false} />
        </TooltipProvider>
      </ModelAndProviderProvider>
    </ThemeProvider>
  );
}
