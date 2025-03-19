import React, { useEffect, useRef, useState } from 'react';
import { addExtensionFromDeepLink } from './extensions';
import { getStoredModel } from './utils/providerUtils';
import { getStoredProvider, initializeSystem } from './utils/providerUtils';
import { useModel } from './components/settings/models/ModelContext';
import { useRecentModels } from './components/settings/models/RecentModels';
import { createSelectedModel } from './components/settings/models/utils';
import { getDefaultModel } from './components/settings/models/hardcoded_stuff';
import ErrorScreen from './components/ErrorScreen';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import { ToastContainer } from 'react-toastify';
import { extractExtensionName } from './components/settings/extensions/utils';
import { GoosehintsModal } from './components/GoosehintsModal';
import { SessionDetails, fetchSessionDetails } from './sessions';

import WelcomeView from './components/WelcomeView';
import ChatView from './components/ChatView';
import SettingsView, { type SettingsViewOptions } from './components/settings/SettingsView';
import SettingsViewV2 from './components/settings_v2/SettingsView';
import MoreModelsView from './components/settings/models/MoreModelsView';
import ConfigureProvidersView from './components/settings/providers/ConfigureProvidersView';
import SessionsView from './components/sessions/SessionsView';
import ProviderSettings from './components/settings_v2/providers/ProviderSettingsPage';
import { useChat } from './hooks/useChat';

import 'react-toastify/dist/ReactToastify.css';
import { useConfig } from './components/ConfigContext';
import {
  initializeBuiltInExtensions,
  syncBuiltInExtensions,
} from './components/settings_v2/extensions/LoadBuiltins';

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
  | 'sessions';

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
  const [isInstalling, setIsInstalling] = useState(false);
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
    // Skip if feature flag is not enabled
    if (!process.env.ALPHA) {
      return;
    }

    const setupExtensions = async () => {
      try {
        // Set the ref immediately to prevent duplicate runs
        initAttemptedRef.current = true;

        // Force refresh extensions from the backend to ensure we have the latest
        const refreshedExtensions = await getExtensions(true);

        if (refreshedExtensions.length === 0) {
          // If we still have no extensions, this is truly a first-time setup
          console.log('First-time setup: Adding all built-in extensions...');
          await initializeBuiltInExtensions(addExtension);
        } else {
          // Extensions exist, check for any missing built-ins
          console.log('Checking for missing built-in extensions...');
          console.log(refreshedExtensions);
          await syncBuiltInExtensions(refreshedExtensions, addExtension);
        }
      } catch (error) {
        console.error('Error setting up extensions:', error);
      }
    };

    const initializeApp = async () => {
      try {
        // Check if we have the required configuration
        const provider = (await read('GOOSE_PROVIDER', false)) as string;
        const model = (await read('GOOSE_MODEL', false)) as string;

        if (provider && model) {
          // We have all needed configuration, initialize the system
          console.log('Initializing system with stored GOOSE_MODEL and GOOSE_PROVIDER');
          await initializeSystem(provider, model);
          setView('chat');
        } else {
          // Missing configuration, show onboarding
          console.log('Missing configuration, showing onboarding');
          setView('welcome');
        }
      } catch (error) {
        console.error('Error initializing app:', error);
        setView('welcome');
      }
    };

    initializeApp().then();
    setupExtensions().then();
  }, []); // Empty dependency array since we're using initAttemptedRef

  const setView = (view: View, viewOptions: Record<any, any> = {}) => {
    setInternalView({ view, viewOptions });
  };

  const [isGoosehintsModalOpen, setIsGoosehintsModalOpen] = useState(false);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const { chat, setChat } = useChat({ setView, setIsLoadingSession });

  useEffect(() => window.electron.reactReady(), []);

  // Keyboard shortcut handler
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        window.electron.createChatWindow(undefined, window.appConfig.get('GOOSE_WORKING_DIR'));
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  useEffect(() => {
    const handleFatalError = (_: any, errorMessage: string) => {
      setFatalError(errorMessage);
    };

    window.electron.on('fatal-error', handleFatalError);
    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, []);

  useEffect(() => {
    const handleSetView = (_, view) => setView(view);
    window.electron.on('set-view', handleSetView);
    return () => window.electron.off('set-view', handleSetView);
  }, []);

  // Add cleanup for session states when view changes
  useEffect(() => {
    if (view !== 'chat') {
      setIsLoadingSession(false);
    }
  }, [view]);

  // TODO: modify
  useEffect(() => {
    const handleAddExtension = (_: any, link: string) => {
      const command = extractCommand(link);
      const extName = extractExtensionName(link);
      window.electron.logInfo(`Adding extension from deep link ${link}`);
      setPendingLink(link);
      setModalMessage(
        `Are you sure you want to install the ${extName} extension?\n\nCommand: ${command}`
      );
      setModalVisible(true);
    };

    window.electron.on('add-extension', handleAddExtension);
    return () => {
      window.electron.off('add-extension', handleAddExtension);
    };
  }, []);

  // TODO: modify
  const handleConfirm = async () => {
    if (pendingLink && !isInstalling) {
      setIsInstalling(true);
      try {
        await addExtensionFromDeepLink(pendingLink, setView);
      } catch (error) {
        console.error('Failed to add extension:', error);
      } finally {
        setModalVisible(false);
        setPendingLink(null);
        setIsInstalling(false);
      }
    }
  };

  // TODO: modify
  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null);
  };

  const { switchModel } = useModel(); // TODO: remove
  const { addRecentModel } = useRecentModels(); // TODO: remove

  useEffect(() => {
    if (process.env.ALPHA) {
      return;
    }

    // TODO: remove
    // Attempt to detect config for a stored provider
    const detectStoredProvider = () => {
      const config = window.electron.getConfig();
      const storedProvider = getStoredProvider(config);
      if (storedProvider) {
        setView('chat');
      } else {
        setView('welcome');
      }
    };

    // TODO: remove
    // Initialize system if we have a stored provider
    const setupStoredProvider = async () => {
      const config = window.electron.getConfig();

      if (config.GOOSE_PROVIDER && config.GOOSE_MODEL) {
        window.electron.logInfo(
          'Initializing system with environment: GOOSE_MODEL and GOOSE_PROVIDER as priority.'
        );
        await initializeSystem(config.GOOSE_PROVIDER, config.GOOSE_MODEL);
        return;
      }
      const storedProvider = getStoredProvider(config);
      const storedModel = getStoredModel();
      if (storedProvider) {
        try {
          await initializeSystem(storedProvider, storedModel);

          if (!storedModel) {
            const modelName = getDefaultModel(storedProvider.toLowerCase());
            const model = createSelectedModel(storedProvider.toLowerCase(), modelName);
            switchModel(model);
            addRecentModel(model);
          }
        } catch (error) {
          // TODO: add sessionError state and show error screen with option to start fresh
          console.error('Failed to initialize with stored provider:', error);
        }
      }
    };
    detectStoredProvider();
    setupStoredProvider();
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
          isSubmitting={isInstalling}
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
