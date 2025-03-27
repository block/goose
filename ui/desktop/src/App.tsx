import React, { useEffect, useRef, useState } from 'react';
import { addExtensionFromDeepLink } from './extensions';
import { openSharedSessionFromDeepLink } from './sessionLinks';
import { getStoredModel } from './utils/providerUtils';
import { getStoredProvider, initializeSystem } from './utils/providerUtils';
import { useModel } from './components/settings/models/ModelContext';
import { useRecentModels } from './components/settings/models/RecentModels';
import { createSelectedModel } from './components/settings/models/utils';
import { getDefaultModel } from './components/settings/models/hardcoded_stuff';
import ErrorScreen from './components/ErrorScreen';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { toastService } from './toasts';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';
import { SessionDetails } from './sessions';
import { SharedSessionDetails } from './sharedSessions';

import WelcomeView from './components/WelcomeView';
import ChatView from './components/ChatView';
import SettingsView, { type SettingsViewOptions } from './components/settings/SettingsView';
import SettingsViewV2 from './components/settings_v2/SettingsView';
import MoreModelsView from './components/settings/models/MoreModelsView';
import ConfigureProvidersView from './components/settings/providers/ConfigureProvidersView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import ProviderSettings from './components/settings_v2/providers/ProviderSettingsPage';
import { useChat } from './hooks/useChat';

import 'react-toastify/dist/ReactToastify.css';
import { FixedExtensionEntry, useConfig } from './components/ConfigContext';
import {
  initializeBuiltInExtensions,
  syncBuiltInExtensions,
  addExtensionFromDeepLink as addExtensionFromDeepLinkV2,
  addToAgentOnStartup,
} from './components/settings_v2/extensions';
import { extractExtensionConfig } from './components/settings_v2/extensions/utils';

// Views and their options
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
  | 'sharedSession';

export type ViewConfig = {
  view: View;
  viewOptions?:
    | SettingsViewOptions
    | {
        resumedSession?: SessionDetails;
      }
    | Record<string, any>;
};

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [modalVisible, setModalVisible] = useState(false);
  const [pendingLink, setPendingLink] = useState<string | null>(null);
  const [modalMessage, setModalMessage] = useState<string>('');
  const [{ view, viewOptions }, setInternalView] = useState<ViewConfig>({
    view: 'welcome',
    viewOptions: {},
  });
  const { getExtensions, addExtension, read } = useConfig();
  const initAttemptedRef = useRef(false);

  // Utility function to extract the command from the link
  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim();
  }

  useEffect(() => {
    if (!process.env.ALPHA) {
      return;
    }

    // Guard against multiple initialization attempts
    if (initAttemptedRef.current) {
      console.log('Initialization already attempted, skipping...');
      return;
    }
    initAttemptedRef.current = true;

    console.log(`Initializing app in alpha mode...`);

    const initializeApp = async () => {
      try {
        const config = window.electron.getConfig();
        const provider = config.GOOSE_PROVIDER ?? (await read('GOOSE_PROVIDER', false));
        const model = config.GOOSE_MODEL ?? (await read('GOOSE_MODEL', false));

        if (provider && model) {
          console.log(`Using provider: ${provider}, model: ${model}`);
          setView('chat');

          try {
            await initializeSystem(provider, model);

            // Initialize or sync built-in extensions into config.yaml
            let refreshedExtensions = await getExtensions(true);

            if (refreshedExtensions.length === 0) {
              await initializeBuiltInExtensions(addExtension);
              refreshedExtensions = await getExtensions(true);
            } else {
              await syncBuiltInExtensions(refreshedExtensions, addExtension);
            }

            // Add enabled extensions to agent
            for (const extensionEntry of refreshedExtensions) {
              if (extensionEntry.enabled) {
                const extensionConfig = extractExtensionConfig(extensionEntry);
                await addToAgentOnStartup({ addToConfig: addExtension, extensionConfig });
              }
            }
          } catch (error) {
            console.error('Error in alpha initialization:', error);
            setFatalError(`System initialization error: ${error.message || 'Unknown error'}`);
            setView('welcome');
          }
        } else {
          console.log('Missing required configuration, showing onboarding');
          setView('welcome');
        }
      } catch (error) {
        console.error('Error in alpha config check:', error);
        setFatalError(`Configuration error: ${error.message || 'Unknown error'}`);
        setView('welcome');
      }

      // Reset toast service after initialization
      toastService.configure({ silent: false });
    };

    initializeApp().catch((error) => {
      console.error('Unhandled error in initialization:', error);
      setFatalError(`Initialization error: ${error.message || 'Unknown error'}`);
    });
  }, []);

  const setView = (view: View, viewOptions: Record<any, any> = {}) => {
    console.log(`Setting view to: ${view}`, viewOptions);
    setInternalView({ view, viewOptions });
  };

  const [isGoosehintsModalOpen, setIsGoosehintsModalOpen] = useState(false);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [sharedSession, setSharedSession] = useState<SharedSessionDetails | null>(null);
  const [sharedSessionError, setSharedSessionError] = useState<string | null>(null);
  const [isLoadingSharedSession, setIsLoadingSharedSession] = useState(false);
  const { chat, setChat } = useChat({ setView, setIsLoadingSession });

  useEffect(() => {
    console.log('Sending reactReady signal to Electron');
    try {
      window.electron.reactReady();
    } catch (error) {
      console.error('Error sending reactReady:', error);
      setFatalError(`React ready notification failed: ${error.message}`);
    }
  }, []);

  // Handle shared session deep links
  useEffect(() => {
    const handleOpenSharedSession = async (_: any, link: string) => {
      window.electron.logInfo(`Opening shared session from deep link ${link}`);
      setIsLoadingSharedSession(true);
      setSharedSessionError(null);

      try {
        await openSharedSessionFromDeepLink(link, setView);
        // No need to handle errors here as openSharedSessionFromDeepLink now handles them internally
      } catch (error) {
        // This should not happen, but just in case
        console.error('Unexpected error opening shared session:', error);
        setView('sessions'); // Fallback to sessions view
      } finally {
        setIsLoadingSharedSession(false);
      }
    };

    window.electron.on('open-shared-session', handleOpenSharedSession);
    return () => {
      window.electron.off('open-shared-session', handleOpenSharedSession);
    };
  }, []);

  // Keyboard shortcut handler
  useEffect(() => {
    console.log('Setting up keyboard shortcuts');
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        try {
          const workingDir = window.appConfig.get('GOOSE_WORKING_DIR');
          console.log(`Creating new chat window with working dir: ${workingDir}`);
          window.electron.createChatWindow(undefined, workingDir);
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
    const handleFatalError = (_: any, errorMessage: string) => {
      console.error('Encountered a fatal error: ', errorMessage);
      // Log additional context that might help diagnose the issue
      console.error('Current view:', view);
      console.error('Is loading session:', isLoadingSession);
      setFatalError(errorMessage);
    };

    window.electron.on('fatal-error', handleFatalError);
    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, [view, isLoadingSession]); // Add dependencies to provide context in error logs

  useEffect(() => {
    console.log('Setting up view change handler');
    const handleSetView = (_, newView) => {
      console.log(`Received view change request to: ${newView}`);
      setView(newView);
    };

    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, []);

  // Add cleanup for session states when view changes
  useEffect(() => {
    console.log(`View changed to: ${view}`);
    if (view !== 'chat') {
      console.log('Not in chat view, clearing loading session state');
      setIsLoadingSession(false);
    }
  }, [view]);

  // TODO: modify
  useEffect(() => {
    console.log('Setting up extension handler');
    const handleAddExtension = (_: any, link: string) => {
      try {
        console.log(`Received add-extension event with link: ${link}`);
        const command = extractCommand(link);
        const extName = extractExtensionName(link);
        window.electron.logInfo(`Adding extension from deep link ${link}`);
        setPendingLink(link);
        setModalMessage(
          `Are you sure you want to install the ${extName} extension?\n\nCommand: ${command}`
        );
        setModalVisible(true);
      } catch (error) {
        console.error('Error handling add-extension event:', error);
      }
    };

    window.electron.on('add-extension', handleAddExtension);
    return () => {
      window.electron.off('add-extension', handleAddExtension);
    };
  }, []);

  // TODO: modify
  const handleConfirm = async () => {
    if (pendingLink) {
      console.log(`Confirming installation of extension from: ${pendingLink}`);
      setModalVisible(false); // Dismiss modal immediately
      try {
        if (process.env.ALPHA) {
          await addExtensionFromDeepLinkV2(pendingLink, addExtension, setView);
        } else {
          await addExtensionFromDeepLink(pendingLink, setView);
        }
        console.log('Extension installation successful');
      } catch (error) {
        console.error('Failed to add extension:', error);
        // Consider showing a user-visible error notification here
      } finally {
        setPendingLink(null);
      }
    }
  };

  // TODO: modify
  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null);
  };

  // TODO: remove
  const { switchModel } = useModel(); // TODO: remove
  const { addRecentModel } = useRecentModels(); // TODO: remove

  useEffect(() => {
    if (process.env.ALPHA) {
      return;
    }

    console.log(`Initializing app in non-alpha mode...`);

    // Attempt to detect config for a stored provider
    const detectStoredProvider = () => {
      try {
        const config = window.electron.getConfig();
        console.log('Loaded config:', JSON.stringify(config));

        const storedProvider = getStoredProvider(config);
        console.log('Stored provider:', storedProvider);

        if (storedProvider) {
          setView('chat');
        } else {
          setView('welcome');
        }
      } catch (err) {
        console.error('DETECTION ERROR:', err);
        setFatalError(`Config detection error: ${err.message || 'Unknown error'}`);
      }
    };

    // Initialize system if we have a stored provider
    const setupStoredProvider = async () => {
      try {
        const config = window.electron.getConfig();

        if (config.GOOSE_PROVIDER && config.GOOSE_MODEL) {
          console.log('using GOOSE_PROVIDER and GOOSE_MODEL from config');
          await initializeSystem(config.GOOSE_PROVIDER, config.GOOSE_MODEL);
          return;
        }

        const storedProvider = getStoredProvider(config);
        const storedModel = getStoredModel();

        if (storedProvider) {
          try {
            await initializeSystem(storedProvider, storedModel);
            console.log('Setup using locally stored provider:', storedProvider);
            console.log('Setup using locally stored model:', storedModel);

            if (!storedModel) {
              const modelName = getDefaultModel(storedProvider.toLowerCase());
              const model = createSelectedModel(storedProvider.toLowerCase(), modelName);
              switchModel(model);
              addRecentModel(model);
            }
          } catch (error) {
            console.error('Failed to initialize with stored provider:', error);
            setFatalError(`Initialization failed: ${error.message || 'Unknown error'}`);
          }
        }
      } catch (err) {
        console.error('SETUP ERROR:', err);
        setFatalError(`Setup error: ${err.message || 'Unknown error'}`);
      }
    };

    // Execute the functions with better error handling
    detectStoredProvider();
    setupStoredProvider().catch((err) => {
      console.error('ASYNC SETUP ERROR:', err);
      setFatalError(`Async setup error: ${err.message || 'Unknown error'}`);
    });
  }, []);

  // keep
  if (fatalError) {
    return <ErrorScreen error={fatalError} onReload={() => window.electron.reloadApp()} />;
  }

  if (isLoadingSession)
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
      </div>
    );

  return (
    <>
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
          title="Confirm Extension Installation"
          message={modalMessage}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
      <div className="relative w-screen h-screen overflow-hidden bg-bgApp flex flex-col">
        <div className="titlebar-drag-region" />
        <div>
          {view === 'welcome' &&
            (process.env.ALPHA ? (
              <ProviderSettings onClose={() => setView('chat')} isOnboarding={true} />
            ) : (
              <WelcomeView
                onSubmit={() => {
                  setView('chat');
                }}
              />
            ))}
          {view === 'settings' &&
            (process.env.ALPHA ? (
              <SettingsViewV2
                onClose={() => {
                  setView('chat');
                }}
                setView={setView}
                viewOptions={viewOptions as SettingsViewOptions}
              />
            ) : (
              <SettingsView
                onClose={() => {
                  setView('chat');
                }}
                setView={setView}
                viewOptions={viewOptions as SettingsViewOptions}
              />
            ))}
          {view === 'moreModels' && (
            <MoreModelsView
              onClose={() => {
                setView('settings');
              }}
              setView={setView}
            />
          )}
          {view === 'configureProviders' && (
            <ConfigureProvidersView
              onClose={() => {
                setView('settings');
              }}
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
          {view === 'sharedSession' && (
            <SharedSessionView
              session={viewOptions.sessionDetails}
              isLoading={isLoadingSharedSession}
              error={viewOptions.error || sharedSessionError}
              onBack={() => setView('sessions')}
              onRetry={async () => {
                if (viewOptions.shareToken && viewOptions.baseUrl) {
                  setIsLoadingSharedSession(true);
                  try {
                    await openSharedSessionFromDeepLink(
                      `goose://sessions/${viewOptions.shareToken}`,
                      setView,
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
        </div>
      </div>
      {isGoosehintsModalOpen && (
        <GoosehintsModal
          directory={window.appConfig.get('GOOSE_WORKING_DIR')}
          setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        />
      )}
    </>
  );
}
