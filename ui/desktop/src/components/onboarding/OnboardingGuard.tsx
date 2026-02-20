import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from '../ConfigContext';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { Goose } from '../icons';
import ProviderSelector from './ProviderSelector';
import OnboardingSuccess from './OnboardingSuccess';
import {
  trackOnboardingStarted,
  trackOnboardingCompleted,
  trackTelemetryPreference,
  setTelemetryEnabled as setAnalyticsTelemetryEnabled,
} from '../../utils/analytics';

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

interface OnboardingGuardProps {
  children: React.ReactNode;
}

export default function OnboardingGuard({ children }: OnboardingGuardProps) {
  const navigate = useNavigate();
  const { read, upsert, getProviders } = useConfig();
  const { refreshCurrentModelAndProvider } = useModelAndProvider();

  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);
  const [hasSelection, setHasSelection] = useState(false);
  const [configuredProvider, setConfiguredProvider] = useState<string | null>(null);
  const onboardingTracked = useRef(false);

  useEffect(() => {
    const checkProvider = async () => {
      try {
        const provider = ((await read('GOOSE_PROVIDER', false)) as string) || '';
        setHasProvider(provider.trim() !== '');
      } catch (error) {
        console.error('Error checking provider:', error);
        setHasProvider(false);
      } finally {
        setIsChecking(false);
      }
    };
    checkProvider();
  }, [read]);

  useEffect(() => {
    if (!isChecking && !hasProvider && !onboardingTracked.current) {
      trackOnboardingStarted();
      onboardingTracked.current = true;
    }
  }, [isChecking, hasProvider]);

  const handleConfigured = async (providerName: string) => {
    const providers = await getProviders(true);
    const match = providers.find((p) => p.name === providerName);
    await upsert('GOOSE_PROVIDER', providerName, false);
    if (match) {
      await upsert('GOOSE_MODEL', match.metadata.default_model, false);
    }
    await refreshCurrentModelAndProvider();
    setConfiguredProvider(providerName);
  };

  const finishOnboarding = async (telemetryEnabled: boolean) => {
    try {
      await upsert(TELEMETRY_CONFIG_KEY, telemetryEnabled, false);
    } catch (error) {
      console.error('Failed to save telemetry preference:', error);
    }
    if (telemetryEnabled) {
      trackTelemetryPreference(true, 'onboarding');
      if (configuredProvider) {
        trackOnboardingCompleted(configuredProvider);
      }
    } else {
      setAnalyticsTelemetryEnabled(false);
    }
    navigate('/', { replace: true });
    setHasProvider(true);
  };

  if (isChecking) {
    return null;
  }

  if (hasProvider) {
    return <>{children}</>;
  }

  if (configuredProvider) {
    return <OnboardingSuccess providerName={configuredProvider} onFinish={finishOnboarding} />;
  }

  return (
    <div className="h-screen w-full bg-background-default overflow-hidden">
      <div className="h-full overflow-y-auto">
        <div
          className={`flex flex-col items-center p-4 pb-8 transition-all duration-500 ease-in-out ${hasSelection ? 'pt-8' : 'pt-[15vh]'}`}
        >
          <div className="max-w-2xl w-full mx-auto">
            <div
              className={`text-left transition-all duration-500 ease-in-out overflow-hidden ${hasSelection ? 'max-h-0 opacity-0 mb-0' : 'max-h-60 opacity-100 mb-8'}`}
            >
              <div className="mb-4">
                <Goose className="size-8" />
              </div>
              <h1 className="text-2xl sm:text-4xl font-light mb-3">Welcome to Goose</h1>
              <p className="text-text-muted text-base sm:text-lg">
                Your local AI agent. Connect an AI model provider to get started.
              </p>
            </div>

            <ProviderSelector
              onConfigured={handleConfigured}
              onFirstSelection={() => setHasSelection(true)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
