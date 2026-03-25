import { useEffect, useState, useMemo, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from './ConfigContext';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { toastService } from '../toasts';
import { LocalModelSetup } from './LocalModelSetup';
import ApiKeyTester from './ApiKeyTester';
import { SwitchModelModal } from './settings/models/subcomponents/SwitchModelModal';
import { createNavigationHandler } from '../utils/navigationUtils';
import TelemetrySettings from './settings/app/TelemetrySettings';
import {
  trackOnboardingStarted,
  trackOnboardingProviderSelected,
  trackOnboardingCompleted,
  trackOnboardingAbandoned,
} from '../utils/analytics';

import { Goose } from './icons';

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
  const [showLocalModelSetup, setShowLocalModelSetup] = useState(false);
  const [userInActiveSetup, setUserInActiveSetup] = useState(false);
  const [showSwitchModelModal, setShowSwitchModelModal] = useState(false);
  const [switchModelProvider, setSwitchModelProvider] = useState<string | null>(null);
  const onboardingTracked = useRef(false);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [showScrollIndicator, setShowScrollIndicator] = useState(true);

  const checkScrollPosition = useCallback(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const { scrollTop, scrollHeight, clientHeight } = container;
    const isNearBottom = scrollTop + clientHeight >= scrollHeight - 50;
    const canScroll = scrollHeight > clientHeight;

    setShowScrollIndicator(canScroll && !isNearBottom);
  }, []);

  const setView = useMemo(() => createNavigationHandler(navigate), [navigate]);

  const handleApiKeySuccess = async (provider: string, _model: string, apiKey: string) => {
    trackOnboardingProviderSelected({ method: 'api_key' });
    const keyName = `${provider.toUpperCase()}_API_KEY`;
    await upsert(keyName, apiKey, true);
    await upsert('GOOSE_PROVIDER', provider, false);

    setSwitchModelProvider(provider);
    setShowSwitchModelModal(true);
  };

  const handleModelSelected = (model: string) => {
    if (switchModelProvider) {
      trackOnboardingCompleted(switchModelProvider, model);
    }
    setShowSwitchModelModal(false);
    setUserInActiveSetup(false);
    setShowFirstTimeSetup(false);
    setHasProvider(true);
    navigate('/', { replace: true });
  };

  const handleSwitchModelClose = () => {
    setShowSwitchModelModal(false);
  };

  const handleLocalModelComplete = () => {
    trackOnboardingCompleted('local');
    setShowLocalModelSetup(false);
    setShowFirstTimeSetup(false);
    setHasProvider(true);
    navigate('/', { replace: true });
  };

  const handleLocalModelCancel = () => {
    trackOnboardingAbandoned('local_model_setup');
    setShowLocalModelSetup(false);
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

  useEffect(() => {
    if (!isChecking && !hasProvider && showFirstTimeSetup && !onboardingTracked.current) {
      trackOnboardingStarted();
      onboardingTracked.current = true;
    }
  }, [isChecking, hasProvider, showFirstTimeSetup]);

  useEffect(() => {
    if (!isChecking && !hasProvider && showFirstTimeSetup) {
      // Check scroll position after content renders
      const timer = setTimeout(checkScrollPosition, 100);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [isChecking, hasProvider, showFirstTimeSetup, checkScrollPosition]);

  if (isChecking) {
    return (
      <div className="h-screen w-full bg-background-primary flex items-center justify-center">
        <WelcomeGooseLogo />
      </div>
    );
  }

  if (showLocalModelSetup) {
    return (
      <div className="h-screen w-full bg-background-default overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="min-h-full flex flex-col items-center justify-center p-4 py-8">
            <div className="max-w-2xl w-full mx-auto p-8">
              <LocalModelSetup
                onSuccess={handleLocalModelComplete}
                onCancel={handleLocalModelCancel}
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!hasProvider && showFirstTimeSetup) {
    return (
      <div className="h-screen w-full bg-background-primary overflow-hidden relative">
        <div
          ref={scrollContainerRef}
          onScroll={checkScrollPosition}
          className="h-full overflow-y-auto"
        >
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
                <p className="text-text-secondary text-base sm:text-lg mt-4 sm:mt-6">
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

              {/* Run Locally Card */}
              <div className="relative w-full mb-4">
                <div className="absolute -top-2 -right-2 sm:-top-3 sm:-right-3 z-20">
                  <span className="inline-block px-2 py-1 text-xs font-medium bg-green-600 text-white rounded-full">
                    Free &amp; Private
                  </span>
                </div>
                <div
                  onClick={() => {
                    trackOnboardingProviderSelected({ method: 'local' });
                    setShowLocalModelSetup(true);
                  }}
                  className="w-full p-4 sm:p-6 bg-transparent border rounded-xl transition-all duration-200 cursor-pointer group"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-text-default text-sm sm:text-base">
                        Run Locally
                      </span>
                    </div>
                    <div className="text-text-muted group-hover:text-text-default transition-colors">
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
                    Download a model and run entirely on your machine. No API keys, no accounts.
                  </p>
                </div>
              </div>

              {/* Other providers section */}
              <div className="w-full p-4 sm:p-6 bg-transparent border rounded-xl">
                <h3 className="font-medium text-text-primary text-sm sm:text-base mb-3">
                  Other Providers
                </h3>
                <p className="text-text-secondary text-sm sm:text-base mb-4">
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

        {/* Scroll indicator - fixed at bottom, hides when scrolled to bottom */}
        {showScrollIndicator && (
          <div className="absolute bottom-4 left-1/2 -translate-x-1/2 pointer-events-none transition-opacity duration-300 opacity-60 animate-bounce">
            <div className="flex flex-col items-center gap-1 text-text-secondary">
              <span className="text-xs">More options below</span>
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M19 9l-7 7-7-7"
                />
              </svg>
            </div>
          </div>
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
