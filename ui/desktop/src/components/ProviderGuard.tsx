import { useEffect, useState, useCallback } from 'react';
import { useConfig } from './ConfigContext';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { toastService } from '../toasts';
import { OllamaSetup } from './OllamaSetup';
import WelcomePage from './WelcomePage';

interface ProviderGuardProps {
  didSelectProvider: boolean;
  children: React.ReactNode;
}

export default function ProviderGuard({ didSelectProvider, children }: ProviderGuardProps) {
  const { read } = useConfig();
  const [hasProvider, setHasProvider] = useState(false);
  const [showFirstTimeSetup, setShowFirstTimeSetup] = useState(false);
  const [isChecking, setIsChecking] = useState(true);
  const [showOllamaSetup, setShowOllamaSetup] = useState(false);

  const handleOllamaComplete = useCallback(() => {
    setShowOllamaSetup(false);
    setHasProvider(true);
    setShowFirstTimeSetup(false);
  }, []);

  const handleOllamaCancel = useCallback(() => {
    setShowOllamaSetup(false);
  }, []);

  useEffect(() => {
    const checkProvider = async () => {
      try {
        const provider = ((await read('GOOSE_PROVIDER', false)) as string) || '';
        const hasConfiguredProvider = provider.trim() !== '';

        if (hasConfiguredProvider || didSelectProvider) {
          setHasProvider(true);
          setShowFirstTimeSetup(false);
        } else {
          setHasProvider(false);
          setShowFirstTimeSetup(true);
        }
      } catch (error) {
        console.error('Error checking provider:', error);
        toastService.error({
          title: 'Configuration Error',
          msg: 'Failed to check provider configuration.',
          traceback: error instanceof Error ? error.stack || '' : '',
        });
        setHasProvider(false);
        setShowFirstTimeSetup(true);
      } finally {
        setIsChecking(false);
      }
    };

    checkProvider();
  }, [read, didSelectProvider]);

  if (isChecking) {
    return (
      <div className="h-screen w-full bg-background-default flex items-center justify-center">
        <WelcomeGooseLogo />
      </div>
    );
  }

  if (showOllamaSetup) {
    return <OllamaSetup onSuccess={handleOllamaComplete} onCancel={handleOllamaCancel} />;
  }

  if (!hasProvider && showFirstTimeSetup) {
    return (
      <WelcomePage
        onComplete={() => {
          setHasProvider(true);
          setShowFirstTimeSetup(false);
        }}
      />
    );
  }

  return <>{children}</>;
}
