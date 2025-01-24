import React, { useEffect, useState } from 'react';
import { addExtensionFromDeepLink } from './extensions';
import { useNavigate } from 'react-router-dom';
import LauncherWindow from './LauncherWindow';
import ChatWindow from './ChatWindow';
import ErrorScreen from './components/ErrorScreen';
import 'react-toastify/dist/ReactToastify.css';
import { ToastContainer } from 'react-toastify';
import { ModelProvider } from './components/settings/models/ModelContext';
import { ActiveKeysProvider } from './components/settings/api_keys/ActiveKeysContext';

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const searchParams = new URLSearchParams(window.location.search);
  const isLauncher = searchParams.get('window') === 'launcher';
  const navigate = useNavigate();

  useEffect(() => {
    window.electron.on('add-extension', (_, link) => {
      window.electron.logInfo(`Adding extension from deep link ${link}`);
      addExtensionFromDeepLink(link, navigate);
    });
  }, [navigate]);

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
  );
}
