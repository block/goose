import React, { useEffect, useState } from 'react';
import LauncherWindow from './LauncherWindow';
import ChatWindow from './ChatWindow';
import ErrorScreen from './components/ErrorScreen';
import FeatureFlagsWindow from './components/FeatureFlags';

export default function App() {
  const [fatalError, setFatalError] = useState<string | null>(null);
  const searchParams = new URLSearchParams(window.location.search);
  const targetWindow = searchParams.get('window');

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
  
  if (targetWindow === 'launcher') {
    return <LauncherWindow />;
  } else if (targetWindow === 'featureFlags') {
    return <FeatureFlagsWindow />;
  } else {
    return <ChatWindow />;
  }
}