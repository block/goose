import React, { useEffect, useRef, useState } from 'react';
import { IpcRendererEvent } from 'electron';
import { openSharedSessionFromDeepLink } from './sessionLinks';
import { initializeSystem } from './utils/providerUtils';
import { ErrorUI } from './components/ErrorBoundary';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { toastService } from './toasts';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';
import { SessionDetails } from './sessions';

import ChatView from './components/ChatView';
import SuspenseLoader from './suspense-loader';
import { type SettingsViewOptions } from './components/settings/SettingsView';
import SettingsViewV2 from './components/settings_v2/SettingsView';
import MoreModelsView from './components/settings/models/MoreModelsView';
import ConfigureProvidersView from './components/settings/providers/ConfigureProvidersView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import ProviderSettings from './components/settings_v2/providers/ProviderSettingsPage';
import RecipeEditor from './components/RecipeEditor';
import { useChat } from './hooks/useChat';

import 'react-toastify/dist/ReactToastify.css';
import { useConfig, MalformedConfigError } from './components/ConfigContext';
import { addExtensionFromDeepLink as addExtensionFromDeepLinkV2 } from './components/settings_v2/extensions';
import { backupConfig, initConfig, readAllConfig } from './api/sdk.gen';
import PermissionSettingsView from './components/settings_v2/permission/PermissionSetting';

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
  | 'sharedSession'
  | 'loading'
  | 'recipeEditor'
  | 'permission';

export type ViewOptions =
  | SettingsViewOptions
  | { resumedSession?: SessionDetails }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  | Record<string, any>;

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

  // Any other URL-specified view
  if (viewFromUrl) {
    return {
      view: viewFromUrl as View,
      viewOptions: {},
    };
  }

  // Default case
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

  // Utility function to extract the command from the link
  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim();
  }

  // Utility function to extract the remote url from the link
  function extractRemoteUrl(link: string): string {
    const url = new URL(link);
    return url.searchParams.get('url');
  }

  const setView = (view: View, viewOptions: ViewOptions = {}) => {
    console.log(`Setting view to: ${view}`, viewOptions);
    setInternalView({ view, viewOptions });
  };

  useEffect(() => {
    // Guard against multiple initialization attempts
    if (initAttemptedRef.current) {
      console.log('Initialization already attempted, skipping...');
      return;
    }
    initAttemptedRef.current = true;

    console.log(`Initializing app with settings v2`);

    const urlParams = new URLSearchParams(window.location.search);
    const viewType = urlParams.get('view');
    const recipeConfig = window.appConfig.get('recipeConfig');

    // If we have a specific view type in the URL, use that and skip provider detection
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
        // checks if there is a config, and if not creates it
        await initConfig();

        // now try to read config, if we fail and are migrating backup, then re-init config
        try {
          await readAllConfig({ throwOnError: true });
        } catch (error) {
          // NOTE: we do this check here and in providerUtils.ts, be sure to clean up both in the future
          const configVersion = localStorage.getItem('configVersion');
          const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;
          if (shouldMigrateExtensions) {
            await backupConfig({ throwOnError: true });
            await initConfig();
          } else {
            // if we've migrated throw this back up
            throw new Error('Unable to read config file, it may be malformed');
          }
        }

        // note: if in a non recipe session, recipeConfig is undefined, otherwise null if error
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
            await initializeSystem(provider, model, {
              getExtensions,
              addExtension,
            });
          } catch (error) {
            console.error('Error in initialization:', error);

            // propagate the error upward so the global ErrorUI shows in cases
            // where going through welcome/onboarding wouldn't address the issue
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

      // Reset toast service after initialization
      toastService.configure({ silent: false });
    };

    initializeApp().catch((error) => {
      console.error('Unhandled error in initialization:', error);
      setFatalError(`${error instanceof Error ? error.message : 'Unknown error'}`);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty dependency array since we only want this to run once

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

    // Get initial view and config
    const urlParams = new URLSearchParams(window.location.search);
    const viewFromUrl = urlParams.get('view');
    if (viewFromUrl) {
      // Get the config from the electron window config
      const windowConfig = window.electron.getConfig();

      if (viewFromUrl === 'recipeEditor') {
        const initialViewOptions = {
          recipeConfig: windowConfig?.recipeConfig,
          view: viewFromUrl,
        };
        setView(viewFromUrl, initialViewOptions);
      } else {
        setView(viewFromUrl);
      }
    }

    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, []);

  // Add cleanup for session states when view changes
  useEffect(() => {
    console.log(`View changed to: ${view}`);
    if (view !== 'chat' && view !== 'recipeEditor') {
      console.log('Not in chat view, clearing loading session state');
      setIsLoadingSession(false);
    }
  }, [view]);

  // Configuration for extension security
  const config = window.electron.getConfig();
  // If GOOSE_ALLOWLIST_WARNING is true, use warning-only mode (STRICT_ALLOWLIST=false)
  // If GOOSE_ALLOWLIST_WARNING is not set or false, use strict blocking mode (STRICT_ALLOWLIST=true)
  const STRICT_ALLOWLIST = config.GOOSE_ALLOWLIST_WARNING === true ? false : true;

  useEffect(() => {
    console.log('Setting up extension handler');
    const handleAddExtension = async (_event: IpcRendererEvent, link: string) => {
      try {
        console.log(`Received add-extension event with link: ${link}`);
        const command = extractCommand(link);
        const remoteUrl = extractRemoteUrl(link);
        const extName = extractExtensionName(link);
        window.electron.logInfo(`Adding extension from deep link ${link}`);
        setPendingLink(link);

        // Default values for confirmation dialog
        let warningMessage = '';
        let label = 'OK';
        let title = 'Confirm Extension Installation';
        let isBlocked = false;
        let useDetailedMessage = false;

        // For SSE extensions (with remoteUrl), always use detailed message
        if (remoteUrl) {
          useDetailedMessage = true;
        } else {
          // For command-based extensions, check against allowlist
          try {
            const allowedCommands = await window.electron.getAllowedExtensions();

            // Only check and show warning if we have a non-empty allowlist
            if (allowedCommands && allowedCommands.length > 0) {
              const isCommandAllowed = allowedCommands.some((allowedCmd) =>
                command.startsWith(allowedCmd)
              );

              if (!isCommandAllowed) {
                // Not in allowlist - use detailed message and show warning/block
                useDetailedMessage = true;
                title = '⛔️ Untrusted Extension ⛔️';

                if (STRICT_ALLOWLIST) {
                  // Block installation completely unless override is active
                  isBlocked = true;
                  label = 'Extension Blocked';
                  warningMessage =
                    '\n\n⛔️ BLOCKED: This extension command is not in the allowed list. ' +
                    'Installation is blocked by your administrator. ' +
                    'Please contact your administrator if you need this extension.';
                } else {
                  // Allow override (either because STRICT_ALLOWLIST is false or secret key combo was used)
                  label = 'Override and install';
                  warningMessage =
                    '\n\n⚠️ WARNING: This extension command is not in the allowed list. ' +
                    'Installing extensions from untrusted sources may pose security risks. ' +
                    'Please contact an admin if you are unsure or want to allow this extension.';
                }
              }
              // If in allowlist, use simple message (useDetailedMessage remains false)
            }
            // If no allowlist, use simple message (useDetailedMessage remains false)
          } catch (error) {
            console.error('Error checking allowlist:', error);
          }
        }

        // Set the appropriate message based on the extension type and allowlist status
        if (useDetailedMessage) {
          // Detailed message for SSE extensions or non-allowlisted command extensions
          const detailedMessage = remoteUrl
            ? `You are about to install the ${extName} extension which connects to:\n\n${remoteUrl}\n\nThis extension will be able to access your conversations and provide additional functionality.`
            : `You are about to install the ${extName} extension which runs the command:\n\n${command}\n\nThis extension will be able to access your conversations and provide additional functionality.`;

          setModalMessage(`${detailedMessage}${warningMessage}`);
        } else {
          // Simple message for allowlisted command extensions or when no allowlist exists
          const messageDetails = `Command: ${command}`;
          setModalMessage(
            `Are you sure you want to install the ${extName} extension?\n\n${messageDetails}`
          );
        }

        setExtensionConfirmLabel(label);
        setExtensionConfirmTitle(title);

        // If blocked, disable the confirmation button functionality by setting a special flag
        if (isBlocked) {
          setPendingLink(null); // Clear the pending link so confirmation does nothing
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

  // Focus the first found input field
  useEffect(() => {
    const handleFocusInput = (_event: IpcRendererEvent) => {
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

  // TODO: modify
  const handleConfirm = async () => {
    if (pendingLink) {
      console.log(`Confirming installation of extension from: ${pendingLink}`);
      setModalVisible(false); // Dismiss modal immediately
      try {
        await addExtensionFromDeepLinkV2(pendingLink, addExtension, setView);
        console.log('Extension installation successful');
      } catch (error) {
        console.error('Failed to add extension:', error);
        // Consider showing a user-visible error notification here
      } finally {
        setPendingLink(null);
      }
    } else {
      // This case happens when pendingLink was cleared due to blocking
      console.log('Extension installation blocked by allowlist restrictions');
      setModalVisible(false);
    }
  };

  // TODO: modify
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
            <SettingsViewV2
              onClose={() => {
                setView('chat');
              }}
              setView={setView}
              viewOptions={viewOptions as SettingsViewOptions}
            />
          )}
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
          {view === 'recipeEditor' && (
            <RecipeEditor
              key={viewOptions?.config ? 'with-config' : 'no-config'}
              config={viewOptions?.config || window.electron.getConfig().recipeConfig}
              onClose={() => setView('chat')}
              setView={setView}
              onSave={(config) => {
                console.log('Saving recipe config:', config);
                window.electron.createChatWindow(
                  undefined,
                  undefined,
                  undefined,
                  undefined,
                  config,
                  'recipeEditor',
                  { config }
                );
                setView('chat');
              }}
            />
          )}
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
    </>
  );
}
