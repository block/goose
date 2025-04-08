import React, { useEffect, useRef, useState } from 'react';
import { IpcRendererEvent } from 'electron';
import { addExtensionFromDeepLink } from './extensions';
import { openSharedSessionFromDeepLink } from './sessionLinks';
import { getStoredModel } from './utils/providerUtils';
import { getStoredProvider, initializeSystem } from './utils/providerUtils';
import { useModel } from './components/settings/models/ModelContext';
import { useRecentModels } from './components/settings/models/RecentModels';
import { createSelectedModel } from './components/settings/models/utils';
import { getDefaultModel } from './components/settings/models/hardcoded_stuff';
import { ErrorUI } from './components/ErrorBoundary';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { toastService } from './toasts';
import { settingsV2Enabled } from './flags';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';

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
import { useConfig, MalformedConfigError } from './components/ConfigContext';
import { addExtensionFromDeepLink as addExtensionFromDeepLinkV2 } from './components/settings_v2/extensions';

import { View, ViewConfig, ViewOptionsBase } from './types/views';

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
  const { switchModel } = useModel();
  const { addRecentModel } = useRecentModels();
  const initAttemptedRef = useRef(false);

  // Utility function to extract the command from the link
  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim();
  }

  const setView = (view: View, viewOptions: ViewOptionsBase = {}) => {
    console.log(`Setting view to: ${view}`, viewOptions);
    setInternalView({ view, viewOptions });
  };

  // Single initialization effect that handles both v1 and v2 settings
  useEffect(() => {
    // Guard against multiple initialization attempts
    if (initAttemptedRef.current) {
      return;
    }
    initAttemptedRef.current = true;

    const initializeApp = async () => {
      try {
        const config = window.electron.getConfig();
        console.log('Loaded config:', JSON.stringify(config));

        if (settingsV2Enabled) {
          console.log('Initializing app with settings v2');
          const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
          const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

          if (provider && model) {
            try {
              await initializeSystem(provider, model, {
                getExtensions,
                addExtension,
              });
              setView('chat');
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
        } else {
          console.log('Initializing app with settings v1');
          const storedProvider = getStoredProvider(config);
          console.log('Stored provider:', storedProvider);

          if (config.GOOSE_PROVIDER && config.GOOSE_MODEL) {
            console.log('using GOOSE_PROVIDER and GOOSE_MODEL from config');
            await initializeSystem(config.GOOSE_PROVIDER, config.GOOSE_MODEL);
            setView('chat');
            return;
          }

          if (storedProvider) {
            const storedModel = getStoredModel();
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
              setView('chat');
            } catch (error) {
              console.error('Failed to initialize with stored provider:', error);
              setFatalError(
                `Initialization failed: ${error instanceof Error ? error.message : 'Unknown error'}`
              );
              setView('welcome');
            }
          } else {
            setView('welcome');
          }
        }

        // Reset toast service after initialization
        toastService.configure({ silent: false });
      } catch (error) {
        console.error('Initialization error:', error);
        setFatalError(`${error instanceof Error ? error.message : 'Unknown error'}`);
        setView('welcome');
      }
    };

    initializeApp().catch((error) => {
      console.error('Unhandled error in initialization:', error);
      setFatalError(`${error instanceof Error ? error.message : 'Unknown error'}`);
    });
  }, [addRecentModel, switchModel, read, getExtensions, addExtension]);

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

  // Handle shared session deep links
  useEffect(() => {
    const handleOpenSharedSession = async (_event: IpcRendererEvent, link: string) => {
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
    const handleFatalError = (_event: IpcRendererEvent, errorMessage: string) => {
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
    const handleSetView = (_event: IpcRendererEvent, newView: View) => {
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

  useEffect(() => {
    console.log('Setting up extension handler');
    const handleAddExtension = (_event: IpcRendererEvent, link: string) => {
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

  const handleConfirm = async () => {
    if (pendingLink) {
      console.log(`Confirming installation of extension from: ${pendingLink}`);
      setModalVisible(false); // Dismiss modal immediately
      try {
        if (settingsV2Enabled) {
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
            (settingsV2Enabled ? (
              <ProviderSettings onClose={() => setView('chat')} isOnboarding={true} />
            ) : (
              <WelcomeView
                onSubmit={() => {
                  setView('chat');
                }}
              />
            ))}
          {view === 'settings' &&
            (settingsV2Enabled ? (
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
              session={viewOptions?.sessionDetails}
              isLoading={isLoadingSharedSession}
              error={viewOptions?.error || sharedSessionError}
              onBack={() => setView('sessions')}
              onRetry={async () => {
                if (viewOptions?.shareToken && viewOptions?.baseUrl) {
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
          directory={window.appConfig.get('GOOSE_WORKING_DIR') as string}
          setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        />
      )}
    </>
  );
}
