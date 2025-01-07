import React, { useEffect, useState } from 'react';
import LauncherWindow from './LauncherWindow';
import ChatWindow from './ChatWindow';
import ErrorScreen from './components/ErrorScreen';
import { ApiKeyWarning } from './components/ApiKeyWarning';

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const searchParams = new URLSearchParams(window.location.search);
  const isLauncher = searchParams.get('window') === 'launcher';

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

  // Check API credentials
  const apiCredsMissing = window.electron.getConfig().apiCredsMissing;

  if (fatalError) {
    return <ErrorScreen error={fatalError} onReload={() => window.electron.reloadApp()} />;
  }

  if (apiCredsMissing && !isLauncher) {
    return <div className="relative w-screen h-screen overflow-hidden dark:bg-dark-window-gradient bg-window-gradient flex flex-col">
      <ApiKeyWarning className="w-full h-full" />
    </div>;
  }
  
  return isLauncher ? <LauncherWindow /> : <ChatWindow />;
}