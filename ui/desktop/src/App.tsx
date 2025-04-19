import React, { useEffect, useRef, useState, useCallback } from 'react';
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
import { SessionDetails } from './sessions';

import WelcomeView from './components/WelcomeView';
import ChatView from './components/ChatView';
import SuspenseLoader from './suspense-loader';
import SettingsView, { type SettingsViewOptions } from './components/settings/SettingsView';
import SettingsViewV2 from './components/settings_v2/SettingsView';
import MoreModelsView from './components/settings/models/MoreModelsView';
import ConfigureProvidersView from './components/settings/providers/ConfigureProvidersView';
import SessionsView from './components/sessions/SessionsView';
import SharedSessionView from './components/sessions/SharedSessionView';
import ProviderSettings from './components/settings_v2/providers/ProviderSettingsPage';
import RecipeEditor from './components/RecipeEditor';
import { useChat } from './hooks/useChat';
import { addExtension as addExtensionDirect, FullExtensionConfig } from './extensions';

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
  const { getExtensions, addExtension, disableAllExtensions, read } = useConfig();
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

  const disableAllStoredExtensions = () => {
    const userSettingsStr = localStorage.getItem('user_settings');
    if (!userSettingsStr) return;

    try {
      const userSettings = JSON.parse(userSettingsStr);
      // Store original state before modifying
      localStorage.setItem('user_settings_backup', userSettingsStr);
      console.log('Backing up user_settings');

      // Disable all extensions
      userSettings.extensions = userSettings.extensions.map((ext) => ({
        ...ext,
        enabled: false,
      }));

      localStorage.setItem('user_settings', JSON.stringify(userSettings));
      console.log('Disabled all stored extensions');
      window.electron.emit('settings-updated');
    } catch (error) {
      console.error('Error disabling stored extensions:', error);
    }
  };

  // Function to restore original extension states for new non-recipe windows
  const restoreOriginalExtensionStates = () => {
    const backupStr = localStorage.getItem('user_settings_backup');
    if (backupStr) {
      localStorage.setItem('user_settings', backupStr);
      console.log('Restored original extension states');
    }
  };

  const updateUserSettingsWithConfig = (extensions: FullExtensionConfig[]) => {
    try {
      const userSettingsStr = localStorage.getItem('user_settings');
      const userSettings = userSettingsStr ? JSON.parse(userSettingsStr) : { extensions: [] };

      // For each extension in the passed in config
      extensions.forEach((newExtension) => {
        // Find if this extension already exists
        const existingIndex = userSettings.extensions.findIndex(
          (ext) => ext.id === newExtension.id
        );

        if (existingIndex !== -1) {
          // Extension exists - just set its enabled to true
          userSettings.extensions[existingIndex].enabled = true;
        } else {
          // Extension is new - add it to the array
          userSettings.extensions.push({
            ...newExtension,
            enabled: true,
          });
        }
      });

      localStorage.setItem('user_settings', JSON.stringify(userSettings));
      console.log('Updated user settings with new/enabled extensions:', userSettings.extensions);

      // Notify any listeners (like the settings page) that settings have changed
      window.electron.emit('settings-updated');
    } catch (error) {
      console.error('Error updating user settings:', error);
    }
  };

  const enableRecipeConfigExtensions = async (extensions: FullExtensionConfig[]) => {
    if (!extensions?.length) {
      console.log('No extensions to enable from bot config');
      return;
    }

    console.log(`Enabling ${extensions.length} extensions from bot config:`, extensions);

    disableAllStoredExtensions();

    // Wait for initial server readiness
    await new Promise((resolve) => setTimeout(resolve, 2000));

    for (const extension of extensions) {
      try {
        console.log(`Enabling extension: ${extension.name}`);
        const extensionConfig = {
          ...extension,
          enabled: true,
        };

        // Try to add the extension
        const response = await addExtensionDirect(extensionConfig, false);

        if (!response.ok) {
          console.error(
            `Failed to enable extension ${extension.name}: Server returned ${response.status}`
          );
          // If it's a 428, retry once
          if (response.status === 428) {
            console.log('Server not ready, waiting and will retry...');
            await new Promise((resolve) => setTimeout(resolve, 2000));
            try {
              await addExtensionDirect(extensionConfig, true);
              console.log(`Successfully enabled extension ${extension.name} on retry`);
            } catch (retryError) {
              console.error(`Failed to enable extension ${extension.name} on retry:`, retryError);
            }
          }
          continue;
        }
        updateUserSettingsWithConfig(extensions);

        console.log(`Successfully enabled extension: ${extension.name}`);
      } catch (error) {
        console.error(`Failed to enable extension ${extension.name}:`, error);
      }
    }

    console.log('Finished enabling bot config extensions');
  };

  const _enableRecipeConfigExtensionsV2 = useCallback(
    async (extensions: FullExtensionConfig[]) => {
      if (!extensions?.length) {
        console.log('No extensions to enable from bot config');
        return;
      }

      try {
        await disableAllExtensions();
        console.log('Disabled all existing extensions');

        for (const extension of extensions) {
          try {
            console.log(`Enabling extension: ${extension.name}`);
            await addExtension(extension.name, extension, true);
          } catch (error) {
            console.error(`Failed to enable extension ${extension.name}:`, error);
          }
        }
      } catch (error) {
        console.error('Failed to enable bot extensions');
      }
      console.log('Finished enabling bot config extensions');
    },
    [disableAllExtensions, addExtension]
  );

  // settings v2 initialization
  useEffect(() => {
    if (!settingsV2Enabled) {
      return;
    }

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

        // Handle bot config extensions first
        // if (recipeConfig?.extensions?.length > 0 && viewType != 'recipeEditor') {
        //   console.log('Found extensions in bot config:', recipeConfig.extensions);
        //   await enableRecipeConfigExtensionsV2(recipeConfig.extensions);
        // }

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

  // TODO: modify
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

        // Fetch the allowlist and check if the command is allowed
        let warningMessage = '';
        let label = 'OK';
        let title = 'Confirm Extension Installation';
        try {
          const allowedCommands = await window.electron.getAllowedExtensions();

          // Only check and show warning if we have a non-empty allowlist
          if (allowedCommands && allowedCommands.length > 0) {
            const isCommandAllowed = allowedCommands.some((allowedCmd) =>
              command.startsWith(allowedCmd)
            );

            if (!isCommandAllowed) {
              title = '⛔️ Untrusted Extension ⛔️';
              label = 'Override and install';
              warningMessage =
                '\n\n⚠️ WARNING: This extension command is not in the allowed list. Installing extensions from untrusted sources may pose security risks. Please contact and admin if you are unsusure or want to allow this extension.';
            }
          }
        } catch (error) {
          console.error('Error checking allowlist:', error);
        }

        const messageDetails = remoteUrl ? `Remote URL: ${remoteUrl}` : `Command: ${command}`;
        setModalMessage(
          `Are you sure you want to install the ${extName} extension?\n\n${messageDetails}${warningMessage}`
        );
        setExtensionConfirmLabel(label);
        setExtensionConfirmTitle(title);
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

  // TODO: modify
  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null);
  };

  // TODO: remove -- careful removal of these and the useEffect below breaks
  //  reloading to chat view using stored provider
  const { switchModel } = useModel(); // TODO: remove
  const { addRecentModel } = useRecentModels(); // TODO: remove

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    const viewType = urlParams.get('view');
    const recipeConfig = window.appConfig.get('recipeConfig');

    if (settingsV2Enabled) {
      return;
    }

    console.log(`Initializing app with settings v1`);

    // Handle bot config extensions first
    if (recipeConfig?.extensions?.length > 0 && viewType != 'recipeEditor') {
      console.log('Found extensions in bot config:', recipeConfig.extensions);
      enableRecipeConfigExtensions(recipeConfig.extensions);
    }

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

    // if not in any of the states above (in a regular chat)
    if (!recipeConfig) {
      restoreOriginalExtensionStates();
    }

    console.log(`Initializing app with settings v1`);

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
      } catch (error) {
        console.error('DETECTION ERROR:', error);
        setFatalError(`Config detection error: ${error.message || 'Unknown error'}`);
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
      } catch (error) {
        console.error('SETUP ERROR:', error);
        setFatalError(`Setup error: ${error.message || 'Unknown error'}`);
      }
    };

    // Execute the functions with better error handling
    detectStoredProvider();
    setupStoredProvider().catch((error) => {
      console.error('ASYNC SETUP ERROR:', error);
      setFatalError(`Async setup error: ${error.message || 'Unknown error'}`);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
