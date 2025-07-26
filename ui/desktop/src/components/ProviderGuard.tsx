import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from './ConfigContext';
import { SetupModal } from './SetupModal';
import { startOpenRouterSetup } from '../utils/openRouterSetup';
import WelcomeGooseLogo from './WelcomeGooseLogo';

interface ProviderGuardProps {
  children: React.ReactNode;
}

export default function ProviderGuard({ children }: ProviderGuardProps) {
  const { read } = useConfig();
  const navigate = useNavigate();
  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);
  const [showFirstTimeSetup, setShowFirstTimeSetup] = useState(false);
  const [openRouterSetupState, setOpenRouterSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showProgress: boolean;
    showRetry: boolean;
    autoClose?: number;
  } | null>(null);

  const handleOpenRouterSetup = async () => {
    setOpenRouterSetupState({
      show: true,
      title: 'Setting up OpenRouter',
      message: 'A browser window will open for authentication...',
      showProgress: true,
      showRetry: false,
    });

    try {
      const result = await startOpenRouterSetup();

      if (result.success) {
        setOpenRouterSetupState({
          show: true,
          title: 'Setup Complete!',
          message: 'OpenRouter has been configured successfully.',
          showProgress: false,
          showRetry: false,
          autoClose: 3000,
        });

        // Reload the page after successful setup
        setTimeout(() => {
          window.location.reload();
        }, 3000);
      } else {
        setOpenRouterSetupState({
          show: true,
          title: 'Setup Pending',
          message: result.message,
          showProgress: false,
          showRetry: true,
        });
      }
    } catch (error) {
      setOpenRouterSetupState({
        show: true,
        title: 'Setup Error',
        message: 'Failed to complete OpenRouter setup',
        showProgress: false,
        showRetry: true,
      });
    }
  };

  useEffect(() => {
    const checkProvider = async () => {
      try {
        const config = window.electron.getConfig();
        console.log('ProviderGuard - Full config:', config);

        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        console.log('ProviderGuard - Provider:', provider, 'Model:', model);

        if (provider && model) {
          console.log('ProviderGuard - Provider and model found, continuing normally');
          setHasProvider(true);
        } else {
          console.log('ProviderGuard - No provider/model configured, showing first time setup');
          setShowFirstTimeSetup(true);
        }
      } catch (error) {
        console.error('Error checking provider configuration:', error);
        // On error, show first time setup
        setShowFirstTimeSetup(true);
      } finally {
        setIsChecking(false);
      }
    };

    checkProvider();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [read]);

  if (isChecking && !openRouterSetupState?.show && !showFirstTimeSetup) {
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
      </div>
    );
  }

  if (openRouterSetupState?.show) {
    return (
      <SetupModal
        title={openRouterSetupState.title}
        message={openRouterSetupState.message}
        showProgress={openRouterSetupState.showProgress}
        showRetry={openRouterSetupState.showRetry}
        onRetry={handleOpenRouterSetup}
        autoClose={openRouterSetupState.autoClose}
        onClose={() => setOpenRouterSetupState(null)}
      />
    );
  }

  if (showFirstTimeSetup) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center bg-background-default">
        <div className="max-w-md w-full mx-auto p-8 text-center">
          <WelcomeGooseLogo />
          <h1 className="text-2xl font-bold text-text-standard mt-8 mb-4">Welcome to Goose!</h1>
          <p className="text-text-muted mb-8">
            Let's get you set up with an AI provider to start using Goose.
          </p>

          <div className="space-y-4">
            <button
              onClick={handleOpenRouterSetup}
              className="w-full px-6 py-3 bg-accent text-text-on-accent rounded-lg hover:bg-accent-hover transition-colors font-medium"
            >
              Set up OpenRouter (Recommended)
            </button>

            <button
              onClick={() => navigate('/welcome', { replace: true })}
              className="w-full px-6 py-3 bg-background-muted text-text-standard rounded-lg hover:bg-background-hover transition-colors font-medium"
            >
              Configure Other Providers
            </button>
          </div>

          <p className="text-sm text-text-muted mt-6">
            OpenRouter provides access to multiple AI models with a single API key.
          </p>
        </div>
      </div>
    );
  }

  if (!hasProvider) {
    // This shouldn't happen, but just in case
    return null;
  }

  return <>{children}</>;
}
