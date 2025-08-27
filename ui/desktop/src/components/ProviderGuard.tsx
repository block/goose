import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from './ConfigContext';
import { SetupModal } from './SetupModal';
import { startOpenRouterSetup } from '../utils/openRouterSetup';
import { startTetrateSetup } from '../utils/tetrateSetup';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { initializeSystem } from '../utils/providerUtils';
import { toastService } from '../toasts';
import { OllamaSetup } from './OllamaSetup';

import { Goose } from './icons/Goose';
import { OpenRouter } from './icons';

interface ProviderGuardProps {
  children: React.ReactNode;
}

export default function ProviderGuard({ children }: ProviderGuardProps) {
  const { read, getExtensions, addExtension } = useConfig();
  const navigate = useNavigate();
  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);
  const [showFirstTimeSetup, setShowFirstTimeSetup] = useState(false);
  const [showOllamaSetup, setShowOllamaSetup] = useState(false);

  const [openRouterSetupState, setOpenRouterSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showProgress: boolean;
    showRetry: boolean;
    autoClose?: number;
  } | null>(null);
  const [tetrateSetupState, setTetrateSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showProgress: boolean;
    showRetry: boolean;
    autoClose?: number;
  } | null>(null);

  const handleTetrateSetup = async () => {
    setTetrateSetupState({
      show: true,
      title: 'Setting up Tetrate Agent Router Service',
      message: 'A browser window will open for authentication...',
      showProgress: true,
      showRetry: false,
    });

    const result = await startTetrateSetup();
    if (result.success) {
      setTetrateSetupState({
        show: true,
        title: 'Setup Complete!',
        message: 'Tetrate Agent Router has been configured successfully. Initializing Goose...',
        showProgress: true,
        showRetry: false,
      });

      // After successful Tetrate setup, force reload config and initialize system
      try {
        // Get the latest config from disk
        const config = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          // Initialize the system with the new provider/model
          await initializeSystem(provider as string, model as string, {
            getExtensions,
            addExtension,
          });

          toastService.configure({ silent: false });
          toastService.success({
            title: 'Success!',
            msg: `Started goose with ${model} by Tetrate. You can change the model via the dropdown.`,
          });

          // Close the modal and mark as having provider
          setTetrateSetupState(null);
          setShowFirstTimeSetup(false);
          setHasProvider(true);
        } else {
          throw new Error('Provider or model not found after Tetrate setup');
        }
      } catch (error) {
        console.error('Failed to initialize after Tetrate setup:', error);
        toastService.configure({ silent: false });
        toastService.error({
          title: 'Initialization Failed',
          msg: `Failed to initialize with Tetrate: ${error instanceof Error ? error.message : String(error)}`,
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    } else {
      setTetrateSetupState({
        show: true,
        title: 'Tetrate setup pending',
        message: result.message,
        showProgress: false,
        showRetry: true,
      });
    }
  };

  const handleOpenRouterSetup = async () => {
    setOpenRouterSetupState({
      show: true,
      title: 'Setting up OpenRouter',
      message: 'A browser window will open for authentication...',
      showProgress: true,
      showRetry: false,
    });

    const result = await startOpenRouterSetup();
    if (result.success) {
      setOpenRouterSetupState({
        show: true,
        title: 'Setup Complete!',
        message: 'OpenRouter has been configured successfully. Initializing Goose...',
        showProgress: true,
        showRetry: false,
      });

      // After successful OpenRouter setup, force reload config and initialize system
      try {
        // Get the latest config from disk
        const config = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          // Initialize the system with the new provider/model
          await initializeSystem(provider as string, model as string, {
            getExtensions,
            addExtension,
          });

          toastService.configure({ silent: false });
          toastService.success({
            title: 'Success!',
            msg: `Started goose with ${model} by OpenRouter. You can change the model via the dropdown.`,
          });

          // Close the modal and mark as having provider
          setOpenRouterSetupState(null);
          setShowFirstTimeSetup(false);
          setHasProvider(true);

          // Navigate to chat after successful setup
          navigate('/', { replace: true });
        } else {
          throw new Error('Provider or model not found after OpenRouter setup');
        }
      } catch (error) {
        console.error('Failed to initialize after OpenRouter setup:', error);
        toastService.configure({ silent: false });
        toastService.error({
          title: 'Initialization Failed',
          msg: `Failed to initialize with OpenRouter: ${error instanceof Error ? error.message : String(error)}`,
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    } else {
      setOpenRouterSetupState({
        show: true,
        title: 'Openrouter setup pending',
        message: result.message,
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

        // Always check for Ollama regardless of provider status

        if (provider && model) {
          console.log('ProviderGuard - Provider and model found, continuing normally');
          setHasProvider(true);
        } else {
          console.log('ProviderGuard - No provider/model configured');
          setShowFirstTimeSetup(true);
        }
      } catch (error) {
        // On error, assume no provider and redirect to welcome
        console.error('Error checking provider configuration:', error);
        navigate('/welcome', { replace: true });
      } finally {
        setIsChecking(false);
      }
    };

    checkProvider();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [read]);

  if (
    isChecking &&
    !openRouterSetupState?.show &&
    !tetrateSetupState?.show &&
    !showFirstTimeSetup &&
    !showOllamaSetup
  ) {
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

  if (tetrateSetupState?.show) {
    return (
      <SetupModal
        title={tetrateSetupState.title}
        message={tetrateSetupState.message}
        showProgress={tetrateSetupState.showProgress}
        showRetry={tetrateSetupState.showRetry}
        onRetry={handleTetrateSetup}
        autoClose={tetrateSetupState.autoClose}
        onClose={() => setTetrateSetupState(null)}
      />
    );
  }

  if (showOllamaSetup) {
    return (
      <div className="min-h-screen w-full flex flex-col items-center justify-center p-4 bg-background-default">
        <div className="max-w-md w-full mx-auto p-8">
          <div className="mb-8 text-center">
            <WelcomeGooseLogo />
          </div>
          <OllamaSetup
            onSuccess={() => {
              setShowOllamaSetup(false);
              setHasProvider(true);
              // Navigate to chat after successful setup
              navigate('/', { replace: true });
            }}
            onCancel={() => {
              setShowOllamaSetup(false);
              setShowFirstTimeSetup(true);
            }}
          />
        </div>
      </div>
    );
  }

  if (showFirstTimeSetup) {
    return (
      <div className="h-screen w-full bg-background-default overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="min-h-full flex flex-col items-center justify-center p-4 py-8">
            <div className="max-w-lg w-full mx-auto p-8">
              {/* Header section - same width as buttons, left aligned */}
              <div className="text-left mb-8 sm:mb-12">
                <div className="space-y-3 sm:space-y-4">
                  <div className="origin-bottom-left goose-icon-animation">
                    <Goose className="size-6 sm:size-8" />
                  </div>
                  <h1 className="text-2xl sm:text-4xl font-light text-left">Welcome to Goose</h1>
                </div>
                <p className="text-text-muted text-base sm:text-lg mt-4 sm:mt-6">
                  Since it's your first time here, let's get you setup with a provider so we can
                  make incredible work together. Scroll down to see options.
                </p>
              </div>

              {/* Setup options - same width container */}

              <div className="space-y-3 sm:space-y-4">
                <div className="relative">
                  {/* Tetrate Card */}
                  {/* Recommended badge - positioned relative to wrapper */}
                  <div className="absolute -top-2 -right-2 sm:-top-3 sm:-right-3 z-20">
                    <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
                      Recommended
                    </span>
                  </div>

                  <div
                    onClick={handleTetrateSetup}
                    className="w-full p-4 sm:p-6 bg-background-muted border border-background-hover rounded-xl hover:border-text-muted transition-all duration-200 cursor-pointer group"
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <h3 className="font-medium text-text-standard text-sm sm:text-base">
                          Automatic setup with Tetrate Agent Router
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
                      Get secure access to multiple AI models, start for free. Quick setup with just
                      a few clicks.
                    </p>
                  </div>
                </div>

                {/* Primary OpenRouter Card with subtle shimmer - wrapped for badge positioning */}
                <div className="relative">
                  <div
                    onClick={handleOpenRouterSetup}
                    className="relative w-full p-4 sm:p-6 bg-background-muted border border-background-hover rounded-xl hover:border-text-muted transition-all duration-200 cursor-pointer group overflow-hidden"
                  >
                    {/* Subtle shimmer effect */}
                    <div className="absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r from-transparent via-white/8 to-transparent"></div>

                    <div className="relative flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <OpenRouter className="w-5 h-5 sm:w-6 sm:h-6 mb-12 text-text-standard" />
                        <h3 className="font-medium text-text-standard text-sm sm:text-base">
                          Automatic setup with OpenRouter
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
                    <p className="relative text-text-muted text-sm sm:text-base">
                      Get instant access to multiple AI models including GPT-4, Claude, and more.
                      Quick setup with just a few clicks.
                    </p>
                  </div>
                </div>

                {/* Other providers Card - outline style */}
                <div
                  onClick={() => navigate('/welcome', { replace: true })}
                  className="w-full p-4 sm:p-6 bg-transparent border border-background-hover rounded-xl hover:border-text-muted transition-all duration-200 cursor-pointer group"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex-1">
                      <h3 className="font-medium text-text-standard text-sm sm:text-base">
                        Other providers
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
                    If you've already signed up for providers like Anthropic, OpenAI etc, you can
                    enter your own keys.
                  </p>
                </div>
              </div>
            </div>
          </div>
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
