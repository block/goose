import { useEffect, useRef, useState } from 'react';
import { IpcRendererEvent } from 'electron';
import { openSharedSessionFromDeepLink, type SessionLinksViewOptions } from './sessionLinks';
import { type SharedSessionDetails } from './sharedSessions';
import { initializeSystem } from './utils/providerUtils';
import { initializeCostDatabase } from './utils/costDatabase';
import { ErrorUI } from './components/ErrorBoundary';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { toastService } from './toasts';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';
import { type ExtensionConfig } from './extensions';
import { type Recipe } from './recipe';
import AnnouncementModal from './components/AnnouncementModal';

import ChatView from './components/ChatView';
import SuspenseLoader from './suspense-loader';
import SettingsView, { SettingsViewOptions } from './components/settings/SettingsView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import SchedulesView from './components/schedule/SchedulesView';
import ProviderSettings from './components/settings/providers/ProviderSettingsPage';
import RecipeEditor from './components/RecipeEditor';
import RecipesView from './components/RecipesView';
import { useChat } from './hooks/useChat';

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

import { type SessionDetails } from './sessions';

export type View =
  | 'welcome'
  | 'chat'
  | 'settings'
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

const getInitialView = (): ViewConfig => {
  const urlParams = new URLSearchParams(window.location.search);
  const viewFromUrl = urlParams.get('view');
  const windowConfig = window.electron.getConfig();

  if (viewFromUrl === 'recipeEditor' && windowConfig?.recipeConfig) {
    return {
      view: 'recipeEditor',
      viewOptions: {
        config: windowConfig.recipeConfig,
      },
    };
  }

  if (viewFromUrl) {
    return {
      view: viewFromUrl as View,
      viewOptions: {},
    };
  }

  return {
    view: 'loading',
    viewOptions: {},
  };
};

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [modalVisible, setModalVisible] = useState(false);
  const [pendingLink, setPendingLink] = useState<string | null>(null);
  const [modalMessage, setModalMessage] = useState<string>('');
  const [extensionConfirmLabel, setExtensionConfirmLabel] = useState<string>('');
  const [extensionConfirmTitle, setExtensionConfirmTitle] = useState<string>('');
  const [{ view, viewOptions }, setInternalView] = useState<ViewConfig>(getInitialView());

  const { getExtensions, addExtension, read } = useConfig();
  const initAttemptedRef = useRef(false);

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

  const setView = (view: View, viewOptions: ViewOptions = {}) => {
    console.log(`Setting view to: ${view}`, viewOptions);
    setInternalView({ view, viewOptions });
  };

  useEffect(() => {
    if (initAttemptedRef.current) {
      console.log('Initialization already attempted, skipping...');
      return;
    }
    initAttemptedRef.current = true;

    console.log(`Initializing app with settings v2`);

    const urlParams = new URLSearchParams(window.location.search);
    const viewType = urlParams.get('view');
    const recipeConfig = window.appConfig.get('recipeConfig');

    if (viewType) {
      if (viewType === 'recipeEditor' && recipeConfig) {
        console.log('Setting view to recipeEditor with config:', recipeConfig);
        setView('recipeEditor', { config: recipeConfig });
      } else {
        setView(viewType as View);
      }
      return;
    }

    const initializeApp = async () => {
      try {
        // Initialize cost database early to pre-load pricing data
        initializeCostDatabase().catch((error) => {
          console.error('Failed to initialize cost database:', error);
        });

        await initConfig();
        try {
          await readAllConfig({ throwOnError: true });
        } catch (error) {
          const configVersion = localStorage.getItem('configVersion');
          const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;
          if (shouldMigrateExtensions) {
            await backupConfig({ throwOnError: true });
            await initConfig();
          } else {
            // Config appears corrupted, try recovery
            console.warn('Config file appears corrupted, attempting recovery...');
            try {
              // First try to validate the config
              try {
                await validateConfig({ throwOnError: true });
                // Config is valid but readAllConfig failed for another reason
                throw new Error('Unable to read config file, it may be malformed');
              } catch (validateError) {
                console.log('Config validation failed, attempting recovery...');

                // Try to recover the config
                try {
                  const recoveryResult = await recoverConfig({ throwOnError: true });
                  console.log('Config recovery result:', recoveryResult);

                  // Try to read config again after recovery
                  try {
                    await readAllConfig({ throwOnError: true });
                    console.log('Config successfully recovered and loaded');
                  } catch (retryError) {
                    console.warn('Config still corrupted after recovery, reinitializing...');
                    await initConfig();
                  }
                } catch (recoverError) {
                  console.warn('Config recovery failed, reinitializing...');
                  await initConfig();
                }
              }
            } catch (recoveryError) {
              console.error('Config recovery process failed:', recoveryError);
              throw new Error('Unable to read config file, it may be malformed');
            }
          }
        }

        if (recipeConfig === null) {
          setFatalError('Cannot read recipe config. Please check the deeplink and try again.');
          return;
        }

        const config = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          setView('chat');
          try {
            await initializeSystem(provider as string, model as string, {
              getExtensions,
              addExtension,
            });
          } catch (error) {
            console.error('Error in initialization:', error);
            if (error instanceof MalformedConfigError) {
              throw error;
            }
            setView('welcome');
          }
        } else {
          console.log('Missing required configuration, showing onboarding');
          setView('welcome');
        }
      } catch (error) {
        setFatalError(
          `Initialization failed: ${error instanceof Error ? error.message : 'Unknown error'}`
        );
        setView('welcome');
      }
      toastService.configure({ silent: false });
    };

    initializeApp().catch((error) => {
      console.error('Unhandled error in initialization:', error);
      setFatalError(`${error instanceof Error ? error.message : 'Unknown error'}`);
    });
  }, [read, getExtensions, addExtension]);

  const [isGoosehintsModalOpen, setIsGoosehintsModalOpen] = useState(false);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [sharedSessionError, setSharedSessionError] = useState<string | null>(null);
  const [isLoadingSharedSession, setIsLoadingSharedSession] = useState(false);
  const { chat, setChat } = useChat({ setView, setIsLoadingSession });

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

  useEffect(() => {
    const handleOpenSharedSession = async (_event: IpcRendererEvent, ...args: unknown[]) => {
      const link = args[0] as string;
      window.electron.logInfo(`Opening shared session from deep link ${link}`);
      setIsLoadingSharedSession(true);
      setSharedSessionError(null);
      try {
        await openSharedSessionFromDeepLink(
          link,
          (view: View, options?: SessionLinksViewOptions) => {
            setView(view, options as ViewOptions);
          }
        );
      } catch (error) {
        console.error('Unexpected error opening shared session:', error);
        setView('sessions');
      } finally {
        setIsLoadingSharedSession(false);
      }
    };
    window.electron.on('open-shared-session', handleOpenSharedSession);
    return () => {
      window.electron.off('open-shared-session', handleOpenSharedSession);
    };
  }, []);

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

  useEffect(() => {
    console.log('Setting up fatal error handler');
    const handleFatalError = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const errorMessage = args[0] as string;
      console.error('Encountered a fatal error: ', errorMessage);
      console.error('Current view:', view);
      console.error('Is loading session:', isLoadingSession);
      setFatalError(errorMessage);
    };
    window.electron.on('fatal-error', handleFatalError);
    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, [view, isLoadingSession]);

  useEffect(() => {
    console.log('Setting up view change handler');
    const handleSetView = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const newView = args[0] as View;
      const section = args[1] as string | undefined;
      console.log(
        `Received view change request to: ${newView}${section ? `, section: ${section}` : ''}`
      );

      if (section && newView === 'settings') {
        setView(newView, { section });
      } else {
        setView(newView);
      }
    };
    const urlParams = new URLSearchParams(window.location.search);
    const viewFromUrl = urlParams.get('view');
    if (viewFromUrl) {
      const windowConfig = window.electron.getConfig();
      if (viewFromUrl === 'recipeEditor') {
        const initialViewOptions = {
          recipeConfig: windowConfig?.recipeConfig,
          view: viewFromUrl,
        };
        setView(viewFromUrl, initialViewOptions);
      } else {
        setView(viewFromUrl as View);
      }
    }
    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, []);

  useEffect(() => {
    console.log(`View changed to: ${view}`);
    if (view !== 'chat' && view !== 'recipeEditor') {
      console.log('Not in chat view, clearing loading session state');
      setIsLoadingSession(false);
    }
  }, [view]);

  const config = window.electron.getConfig();
  const STRICT_ALLOWLIST = config.GOOSE_ALLOWLIST_WARNING === true ? false : true;

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
          setView(view as View, options as ViewOptions);
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
    <ModelAndProviderProvider>
      <ToastContainer
        aria-label="Toast notifications"
        toastClassName={() =>
          `relative min-h-16 mb-4 p-2 rounded-lg
           flex justify-between overflow-hidden cursor-pointer
           text-textProminentInverse bg-bgStandardInverse dark:bg-bgAppInverse
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
      <div className="relative w-screen h-screen overflow-hidden bg-bgApp flex flex-col">
        <div className="titlebar-drag-region" />
        <div>
          {view === 'loading' && <SuspenseLoader />}
          {view === 'welcome' && (
            <ProviderSettings onClose={() => setView('chat')} isOnboarding={true} />
          )}
          {view === 'settings' && (
            <SettingsView
              onClose={() => {
                setView('chat');
              }}
              setView={setView}
              viewOptions={viewOptions as SettingsViewOptions}
            />
          )}
          {view === 'ConfigureProviders' && (
            <ProviderSettings onClose={() => setView('chat')} isOnboarding={false} />
          )}
          {view === 'chat' && !isLoadingSession && (
            <ChatView
              chat={chat}
              setChat={setChat}
              setView={setView}
              setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            />
          )}
          {view === 'sessions' && <SessionsView setView={setView} />}
          {view === 'schedules' && <SchedulesView onClose={() => setView('chat')} />}
          {view === 'sharedSession' && (
            <SharedSessionView
              session={
                (viewOptions?.sessionDetails as unknown as SharedSessionDetails | null) || null
              }
              isLoading={isLoadingSharedSession}
              error={viewOptions?.error || sharedSessionError}
              onBack={() => setView('sessions')}
              onRetry={async () => {
                if (viewOptions?.shareToken && viewOptions?.baseUrl) {
                  setIsLoadingSharedSession(true);
                  try {
                    await openSharedSessionFromDeepLink(
                      `goose://sessions/${viewOptions.shareToken}`,
                      (view: View, options?: SessionLinksViewOptions) => {
                        setView(view, options as ViewOptions);
                      },
                      viewOptions.baseUrl
                    );
                  } catch (error) {
                    console.error('Failed to retry loading shared session:', error);
                  } finally {
                    setIsLoadingSharedSession(false);
                  }
                }
              }}
            />
          )}
          {view === 'recipeEditor' && (
            <RecipeEditor
              config={(viewOptions?.config as Recipe) || window.electron.getConfig().recipeConfig}
            />
          )}
          {view === 'recipes' && <RecipesView onBack={() => setView('chat')} />}
          {view === 'permission' && (
            <PermissionSettingsView
              onClose={() => setView((viewOptions as { parentView: View }).parentView)}
            />
          )}
        </div>
      </div>
      {isGoosehintsModalOpen && (
        <GoosehintsModal
          directory={window.appConfig.get('GOOSE_WORKING_DIR') as string}
          setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        />
      )}
      <AnnouncementModal />
    </ModelAndProviderProvider>
  );
}
