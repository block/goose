import { useEffect, useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from './ConfigContext';
import { SetupModal } from './SetupModal';
import { startOpenRouterSetup } from '../utils/openRouterSetup';
import { startTetrateSetup } from '../utils/tetrateSetup';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { toastService } from '../toasts';
import { OllamaSetup } from './OllamaSetup';
import ApiKeyTester from './ApiKeyTester';
import { SwitchModelModal } from './settings/models/subcomponents/SwitchModelModal';
import { createNavigationHandler } from '../utils/navigationUtils';
import TelemetrySettings from './settings/app/TelemetrySettings';

import { Goose, OpenRouter, Tetrate } from './icons';

interface ProviderGuardProps {
  didSelectProvider: boolean;
  children: React.ReactNode;
}

export default function ProviderGuard({ didSelectProvider, children }: ProviderGuardProps) {
  const { read, upsert } = useConfig();
  const navigate = useNavigate();
  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);
  const [showFirstTimeSetup, setShowFirstTimeSetup] = useState(false);
  const [showOllamaSetup, setShowOllamaSetup] = useState(false);
  const [userInActiveSetup, setUserInActiveSetup] = useState(false);
  const [showSwitchModelModal, setShowSwitchModelModal] = useState(false);
  const [switchModelProvider, setSwitchModelProvider] = useState<string | null>(null);

  const setView = useMemo(() => createNavigationHandler(navigate), [navigate]);

  const [openRouterSetupState, setOpenRouterSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showRetry: boolean;
    autoClose?: number;
  } | null>(null);

  const [tetrateSetupState, setTetrateSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showRetry: boolean;
    autoClose?: number;
  } | null>(null);

  const handleTetrateSetup = async () => {
    try {
      const result = await startTetrateSetup();
      if (result.success) {
        setSwitchModelProvider('tetrate');
        setShowSwitchModelModal(true);
      } else {
        setTetrateSetupState({
          show: true,
          title: 'Setup Failed',
          message: result.message,
          showRetry: true,
        });
      }
    } catch (error) {
      console.error('Tetrate setup error:', error);
      setTetrateSetupState({
        show: true,
        title: 'Setup Error',
        message: 'An unexpected error occurred during setup.',
        showRetry: true,
      });
    }
  };

  const handleApiKeySuccess = async (provider: string, _model: string, apiKey: string) => {
    const keyName = `${provider.toUpperCase()}_API_KEY`;
    await upsert(keyName, apiKey, true);
    await upsert('GOOSE_PROVIDER', provider, false);

    setSwitchModelProvider(provider);
    setShowSwitchModelModal(true);
  };

  const handleModelSelected = () => {
    setShowSwitchModelModal(false);
    setUserInActiveSetup(false);
    setShowFirstTimeSetup(false);
    setHasProvider(true);
    navigate('/', { replace: true });
  };

  const handleSwitchModelClose = () => {
    setShowSwitchModelModal(false);
  };

  const handleOpenRouterSetup = async () => {
    try {
      const result = await startOpenRouterSetup();
      if (result.success) {
        setSwitchModelProvider('openrouter');
        setShowSwitchModelModal(true);
      } else {
        setOpenRouterSetupState({
          show: true,
          title: 'Setup Failed',
          message: result.message,
          showRetry: true,
        });
      }
    } catch (error) {
      console.error('OpenRouter setup error:', error);
      setOpenRouterSetupState({
        show: true,
        title: 'Setup Error',
        message: 'An unexpected error occurred during setup.',
        showRetry: true,
      });
    }
  };

  const handleOllamaComplete = () => {
    setShowOllamaSetup(false);
    setShowFirstTimeSetup(false);
    setHasProvider(true);
    navigate('/', { replace: true });
  };

  const handleOllamaCancel = () => {
    setShowOllamaSetup(false);
  };

  const handleRetrySetup = (setupType: 'openrouter' | 'tetrate') => {
    if (setupType === 'openrouter') {
      setOpenRouterSetupState(null);
      handleOpenRouterSetup();
    } else {
      setTetrateSetupState(null);
      handleTetrateSetup();
    }
  };

  const closeSetupModal = (setupType: 'openrouter' | 'tetrate') => {
    if (setupType === 'openrouter') {
      setOpenRouterSetupState(null);
    } else {
      setTetrateSetupState(null);
    }
  };

  useEffect(() => {
    const checkProvider = async () => {
      try {
        const provider = ((await read('GOOSE_PROVIDER', false)) as string) || '';
        const hasConfiguredProvider = provider.trim() !== '';

        // If user is actively testing keys, don't redirect
        if (userInActiveSetup) {
          setHasProvider(false);
          setShowFirstTimeSetup(true);
        } else if (hasConfiguredProvider || didSelectProvider) {
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
  }, [read, didSelectProvider, userInActiveSetup]);

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
      <div className="h-screen w-full bg-background-default overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="min-h-full flex flex-col items-center justify-center p-4 py-8">
            <div className="max-w-2xl w-full mx-auto p-8">
              {/* Header section */}
              <div className="text-left mb-8 sm:mb-12">
                <div className="space-y-3 sm:space-y-4">
                  <div className="origin-bottom-left goose-icon-animation">
                    <Goose className="size-6 sm:size-8" />
                  </div>
                  <h1 className="text-2xl sm:text-4xl font-light text-left">Welcome to Goose</h1>
                </div>
                <p className="text-text-muted text-base sm:text-lg mt-4 sm:mt-6">
                  Since it’s your first time here, let’s get you set up with an AI provider so goose
                  can work its magic.
                </p>
              </div>

              <ApiKeyTester
                onSuccess={handleApiKeySuccess}
                onStartTesting={() => {
                  setUserInActiveSetup(true);
                }}
              />

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
                {/* Tetrate Card */}
                <div
                  onClick={handleTetrateSetup}
                  className="w-full p-4 sm:p-6 bg-transparent border border-background-hover rounded-xl hover:border-text-muted transition-all duration-200 cursor-pointer group"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex-1">
                      <Tetrate className="w-5 h-5 mb-3 text-text-standard" />
                      <h3 className="font-medium text-text-standard text-sm sm:text-base">
                        Tetrate Agent Router
                      </h3>
                    </div>
                    <div className="text-text-muted group-hover:text-text-standard transition-colors">
                      <svg
                        className="w-4 h-4 sm:w-5 sm:h-5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 5l7 7-7 7"
                        />
                      </svg>
                    </div>
                  </div>
                  <p className="text-text-muted text-sm sm:text-base">
                    Secure access to multiple AI models with automatic setup. Free tier available.
                  </p>
                </div>

                {/* OpenRouter Card */}
                <div
                  onClick={handleOpenRouterSetup}
                  className="relative w-full p-4 sm:p-6 bg-transparent border border-background-hover rounded-xl hover:border-text-muted transition-all duration-200 cursor-pointer group overflow-hidden"
                >
                  {/* Subtle shimmer effect */}
                  <div className="absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r from-transparent via-white/8 to-transparent"></div>

                  <div className="relative flex items-start justify-between mb-3">
                    <div className="flex-1">
                      <OpenRouter className="w-5 h-5 mb-3 text-text-standard" />
                      <h3 className="font-medium text-text-standard text-sm sm:text-base">
                        OpenRouter
                      </h3>
                    </div>
                    <div className="text-text-muted group-hover:text-text-standard transition-colors">
                      <svg
                        className="w-4 h-4 sm:w-5 sm:h-5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 5l7 7-7 7"
                        />
                      </svg>
                    </div>
                  </div>
                  <p className="text-text-muted text-sm sm:text-base">
                    Access 200+ models with one API. Pay-per-use pricing.
                  </p>
                </div>
              </div>

              {/* Other providers section */}
              <div className="w-full p-4 sm:p-6 bg-transparent border border-background-hover rounded-xl">
                <h3 className="font-medium text-text-standard text-sm sm:text-base mb-3">
                  Other Providers
                </h3>
                <p className="text-text-muted text-sm sm:text-base mb-4">
                  Set up additional providers manually through settings.
                </p>
                <button
                  onClick={() => navigate('/welcome', { replace: true })}
                  className="text-blue-600 hover:text-blue-500 text-sm font-medium transition-colors"
                >
                  Go to Provider Settings →
                </button>
              </div>
              <div className="mt-6">
                <TelemetrySettings isWelcome />
              </div>
            </div>
          </div>
        </div>

        {/* Setup Modals */}
        {openRouterSetupState?.show && (
          <SetupModal
            title={openRouterSetupState.title}
            message={openRouterSetupState.message}
            showRetry={openRouterSetupState.showRetry}
            onRetry={() => handleRetrySetup('openrouter')}
            onClose={() => closeSetupModal('openrouter')}
            autoClose={openRouterSetupState.autoClose}
          />
        )}

        {tetrateSetupState?.show && (
          <SetupModal
            title={tetrateSetupState.title}
            message={tetrateSetupState.message}
            showRetry={tetrateSetupState.showRetry}
            onRetry={() => handleRetrySetup('tetrate')}
            onClose={() => closeSetupModal('tetrate')}
            autoClose={tetrateSetupState.autoClose}
          />
        )}

        {showSwitchModelModal && (
          <SwitchModelModal
            sessionId={null}
            onClose={handleSwitchModelClose}
            setView={setView}
            onModelSelected={handleModelSelected}
            initialProvider={switchModelProvider}
            titleOverride="Choose Model"
          />
        )}
      </div>
    );
  }

  return <>{children}</>;
}
