import React, { useEffect, useState } from 'react';
import { addExtensionFromDeepLink } from './extensions';
import { useNavigate } from 'react-router-dom';
import LauncherWindow from './LauncherWindow';
import ChatWindow from './ChatWindow';
import ErrorScreen from './components/ErrorScreen';
import { ConfirmationModal } from './components/ui/ConfirmationModal';
import 'react-toastify/dist/ReactToastify.css';
import { ToastContainer } from 'react-toastify';
import { ModelProvider } from './components/settings/models/ModelContext';
import { ActiveKeysProvider } from './components/settings/api_keys/ActiveKeysContext';
import { extractExtensionName } from './components/settings/extensions/utils';

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [modalVisible, setModalVisible] = useState(false);
  const [pendingLink, setPendingLink] = useState<string | null>(null);
  const [modalMessage, setModalMessage] = useState<string>('');
  const [isInstalling, setIsInstalling] = useState(false); // Track installation progress
  const searchParams = new URLSearchParams(window.location.search);
  const isLauncher = searchParams.get('window') === 'launcher';
  const navigate = useNavigate();

  // Utility function to extract the command from the link
  function extractCommand(link: string): string {
    const url = new URL(link);
    const cmd = url.searchParams.get('cmd') || 'Unknown Command';
    const args = url.searchParams.getAll('arg').map(decodeURIComponent);
    return `${cmd} ${args.join(' ')}`.trim(); // Combine the command and arguments
  }

  useEffect(() => {
    const handleAddExtension = (_, link: string) => {
      const command = extractCommand(link); // Extract and format the command
      const extName = extractExtensionName(link);
      window.electron.logInfo(`Adding extension from deep link ${link}`);
      setPendingLink(link); // Save the link for later use
      setModalMessage(
        `Are you sure you want to install the ${extName} extension?\n\nCommand: ${command}`
      ); // Display command
      setModalVisible(true); // Show confirmation modal
    };

    window.electron.on('add-extension', handleAddExtension);

    return () => {
      // Clean up the event listener when the component unmounts
      window.electron.off('add-extension', handleAddExtension);
    };
  }, []);

  const handleConfirm = async () => {
    if (pendingLink && !isInstalling) {
      setIsInstalling(true); // Disable further attempts
      console.log('Confirming installation for link:', pendingLink);

      try {
        await addExtensionFromDeepLink(pendingLink, navigate); // Proceed with adding the extension
      } catch (error) {
        console.error('Failed to add extension:', error);
      } finally {
        // Always reset states
        setModalVisible(false);
        setPendingLink(null);
        setIsInstalling(false);
      }
    }
  };

  const handleCancel = () => {
    console.log('Cancelled extension installation.');
    setModalVisible(false);
    setPendingLink(null); // Clear the link if the user cancels
  };

  useEffect(() => {
    const handleFatalError = (_: any, errorMessage: string) => {
      setFatalError(errorMessage);
    };

    // Listen for fatal errors from main process
    window.electron.on('fatal-error', handleFatalError);

    return () => {
      window.electron.off('fatal-error', handleFatalError);
    };
  }, []);

  if (fatalError) {
    return <ErrorScreen error={fatalError} onReload={() => window.electron.reloadApp()} />;
  }

  return (
    <>
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
      <ModelProvider>
        <ActiveKeysProvider>
          {isLauncher ? <LauncherWindow /> : <ChatWindow />}
          <ToastContainer
            aria-label="Toast notifications"
            position="top-right"
            autoClose={3000}
            closeOnClick
            pauseOnHover
          />
        </ActiveKeysProvider>
      </ModelProvider>
    </>
  );
}
