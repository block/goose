import React, { useEffect, useState } from 'react';
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

import WelcomeView from './components/WelcomeView';
import ChatView from './components/ChatView';
import SettingsView from './components/settings/SettingsView';
import MoreModelsView from './components/settings/models/MoreModelsView';
import ConfigureProvidersView from './components/settings/providers/ConfigureProvidersView';

import 'react-toastify/dist/ReactToastify.css';

export type View =
  | 'welcome'
  | 'chat'
  | 'settings'
  | 'moreModels'
  | 'configureProviders'
  | 'configPage';

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [modalVisible, setModalVisible] = useState(false);
  const [pendingLink, setPendingLink] = useState<string | null>(null);
  const [modalMessage, setModalMessage] = useState<string>('');
  const [isInstalling, setIsInstalling] = useState(false);
  const [view, setView] = useState<View>('welcome');
  const { switchModel } = useModel();
  const { addRecentModel } = useRecentModels();

  // Utility function to extract the command from the link
  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim();
  }

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

  // Keyboard shortcut handler
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        window.electron.createChatWindow();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Attempt to detect config for a stored provider
  useEffect(() => {
    const config = window.electron.getConfig();
    const storedProvider = getStoredProvider(config);
    if (storedProvider) {
      setView('chat');
    } else {
      setView('welcome');
    }
  }, []);

  // Initialize system if we have a stored provider
  useEffect(() => {
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
          console.error('Failed to initialize with stored provider:', error);
        }
      }
    };

    setupStoredProvider();
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

  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null);
  };

  if (fatalError) {
    return <ErrorScreen error={fatalError} onReload={() => window.electron.reloadApp()} />;
  }

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
          {view === 'welcome' && (
            <WelcomeView
              onSubmit={() => {
                setView('chat');
              }}
            />
          )}
          {view === 'settings' && (
            <SettingsView
              onClose={() => {
                setView('chat');
              }}
              setView={setView}
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
          {view === 'chat' && <ChatView setView={setView} />}
        </div>
      </div>
    </>
  );
}
