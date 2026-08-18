import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Loader2 } from 'lucide-react';
import { useConfig } from '../ConfigContext';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { acpListProviderDetails, acpReadDefaults, acpSaveDefaults } from '../../acp/providers';
import { Avocado } from '../icons';
import { Button } from '../ui/button';
import ProviderSelector from './ProviderSelector';
import OnboardingSuccess from './OnboardingSuccess';
import { PROVIDER_MANAGEMENT_ENABLED } from '../../updates';
import {
  trackOnboardingStarted,
  trackOnboardingCompleted,
  trackOnboardingProviderSelected,
  trackTelemetryPreference,
  setTelemetryEnabled as setAnalyticsTelemetryEnabled,
} from '../../utils/analytics';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  welcomeTitle: {
    id: 'onboardingGuard.welcomeTitle',
    defaultMessage: 'Welcome to Avocado Work',
  },
  welcomeDescription: {
    id: 'onboardingGuard.welcomeDescription',
    defaultMessage: 'Your local AI agent. Connect a provider to get started.',
  },
  checkProviderErrorTitle: {
    id: 'onboardingGuard.checkProviderErrorTitle',
    defaultMessage: 'Unable to connect to Avocado Work server',
  },
  checkProviderErrorDescription: {
    id: 'onboardingGuard.checkProviderErrorDescription',
    defaultMessage: 'The server may be starting up or temporarily unavailable.',
  },
  retry: {
    id: 'onboardingGuard.retry',
    defaultMessage: 'Retry',
  },
  connecting: {
    id: 'onboardingGuard.connecting',
    defaultMessage: 'Connecting to Avocado Work...',
  },
  accessDeniedTitle: {
    id: 'onboardingGuard.accessDeniedTitle',
    defaultMessage: 'Access denied',
  },
  accessDeniedDescription: {
    id: 'onboardingGuard.accessDeniedDescription',
    defaultMessage:
      'Your account does not have the agent-access role. Ask an Avocado admin to grant access, then try again.',
  },
  tryAgain: {
    id: 'onboardingGuard.tryAgain',
    defaultMessage: 'Try again',
  },
});

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

interface OnboardingGuardProps {
  children: React.ReactNode;
}

function isAccessDeniedError(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes('agent-access') ||
    lower.includes('forbidden') ||
    lower.includes('access denied') ||
    lower.includes('403')
  );
}

export default function OnboardingGuard({ children }: OnboardingGuardProps) {
  const intl = useIntl();
  const navigate = useNavigate();
  const { upsert } = useConfig();
  const { refreshCurrentModelAndProvider } = useModelAndProvider();

  const [isCheckingProvider, setIsCheckingProvider] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);
  const [checkProviderError, setCheckProviderError] = useState(false);
  const [hasSelection, setHasSelection] = useState(false);
  const [configuredProvider, setConfiguredProvider] = useState<string | null>(null);
  const [configuredProviderDisplayName, setConfiguredProviderDisplayName] = useState<string | null>(
    null
  );
  const [configuredModel, setConfiguredModel] = useState<string | null>(null);
  const [accessDenied, setAccessDenied] = useState(false);
  const hasTrackedOnboardingStart = useRef(false);

  const checkProvider = async (retries = 12, delay = 1000) => {
    setIsCheckingProvider(true);
    setCheckProviderError(false);
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const { providerId, modelId } = await acpReadDefaults();
        const providers = await acpListProviderDetails();

        if (providerId?.trim() && modelId?.trim()) {
          // AC-6: baked defaults are not enough — provider must actually be configured
          // (virtual key present after avocado OAuth).
          const match = providers.find((p) => p.name === providerId);
          if (match?.is_configured) {
            await refreshCurrentModelAndProvider();
            setHasProvider(true);
            setIsCheckingProvider(false);
            return;
          }
        }

        // Backend answered — show Sign in immediately (do not spin on retries).
        setHasProvider(false);
        setIsCheckingProvider(false);
        return;
      } catch (error) {
        console.error(`Error checking provider (attempt ${attempt + 1}/${retries + 1}):`, error);
        if (attempt < retries) {
          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }
    setCheckProviderError(true);
    setIsCheckingProvider(false);
  };

  useEffect(() => {
    checkProvider();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (
      !isCheckingProvider &&
      !hasProvider &&
      !checkProviderError &&
      !hasTrackedOnboardingStart.current
    ) {
      trackOnboardingStarted();
      hasTrackedOnboardingStart.current = true;
    }
  }, [isCheckingProvider, hasProvider, checkProviderError]);

  const handleConfigured = async (providerName: string, modelId?: string) => {
    trackOnboardingProviderSelected({ provider: providerName });
    const providers = await acpListProviderDetails();
    const matchedProvider = providers.find((p) => p.name === providerName);
    const resolvedModel = modelId ?? matchedProvider?.metadata.default_model ?? null;
    await acpSaveDefaults(providerName, resolvedModel);
    setConfiguredModel(resolvedModel);
    await refreshCurrentModelAndProvider();
    setConfiguredProvider(providerName);
    setConfiguredProviderDisplayName(matchedProvider?.metadata.display_name || providerName);
    setAccessDenied(false);
  };

  const handleConfigError = (message: string) => {
    if (isAccessDeniedError(message)) {
      setAccessDenied(true);
    }
  };

  const finishOnboarding = async (telemetryEnabled: boolean) => {
    try {
      await upsert(TELEMETRY_CONFIG_KEY, telemetryEnabled, false);
    } catch (error) {
      console.error('Failed to save telemetry preference:', error);
    }
    trackTelemetryPreference(telemetryEnabled, 'onboarding');
    if (configuredProvider) {
      trackOnboardingCompleted(configuredProvider, configuredModel ?? undefined);
    }
    if (!telemetryEnabled) {
      setAnalyticsTelemetryEnabled(false);
    }
    navigate('/', { replace: true });
    setHasProvider(true);
  };

  if (isCheckingProvider) {
    return (
      <div
        className="h-screen w-full bg-background-default flex flex-col items-center justify-center"
        data-testid="onboarding-connecting"
      >
        <div className="text-center">
          <Avocado className="size-8 mx-auto mb-4" />
          <p className="text-text-muted flex items-center gap-2">
            <Loader2 className="h-4 w-4 animate-spin" />
            {intl.formatMessage(i18n.connecting)}
          </p>
        </div>
      </div>
    );
  }

  if (checkProviderError) {
    return (
      <div className="h-screen w-full bg-background-default flex flex-col items-center justify-center">
        <div className="text-center max-w-md">
          <div className="mb-4">
            <Avocado className="size-8 mx-auto" />
          </div>
          <h1 className="text-xl font-light mb-3">
            {intl.formatMessage(i18n.checkProviderErrorTitle)}
          </h1>
          <p className="text-text-muted mb-6">
            {intl.formatMessage(i18n.checkProviderErrorDescription)}
          </p>
          <Button onClick={() => checkProvider()}>{intl.formatMessage(i18n.retry)}</Button>
        </div>
      </div>
    );
  }

  if (hasProvider) {
    return <>{children}</>;
  }

  if (accessDenied) {
    return (
      <div
        className="h-screen w-full bg-background-default flex flex-col items-center justify-center"
        data-testid="onboarding-access-denied"
      >
        <div className="text-center max-w-md">
          <div className="mb-4">
            <Avocado className="size-8 mx-auto" />
          </div>
          <h1 className="text-xl font-light mb-3">
            {intl.formatMessage(i18n.accessDeniedTitle)}
          </h1>
          <p className="text-text-muted mb-6">
            {intl.formatMessage(i18n.accessDeniedDescription)}
          </p>
          <Button
            onClick={() => {
              setAccessDenied(false);
              setHasSelection(true);
            }}
          >
            {intl.formatMessage(i18n.tryAgain)}
          </Button>
        </div>
      </div>
    );
  }

  if (configuredProviderDisplayName) {
    return (
      <OnboardingSuccess providerName={configuredProviderDisplayName} onFinish={finishOnboarding} />
    );
  }

  // Distro mode auto-selects Avocado, so the welcome header would only flash before
  // collapsing. Show the sign-in on its own, vertically centred.
  if (!PROVIDER_MANAGEMENT_ENABLED) {
    return (
      <div
        className="h-screen w-full bg-background-default overflow-hidden"
        data-testid="onboarding-signin"
      >
        <div className="h-full overflow-y-auto flex items-center justify-center p-4">
          <div className="max-w-sm w-full mx-auto">
            <ProviderSelector
              onConfigured={handleConfigured}
              onFirstSelection={() => setHasSelection(true)}
              onConfigError={handleConfigError}
            />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className="h-screen w-full bg-background-default overflow-hidden"
      data-testid="onboarding-signin"
    >
      <div className="h-full overflow-y-auto">
        <div
          className={`flex flex-col items-center p-4 pb-8 transition-all duration-500 ease-in-out ${hasSelection ? 'pt-8' : 'pt-[15vh]'}`}
        >
          <div className="max-w-2xl w-full mx-auto">
            <div
              className={`text-left transition-all duration-500 ease-in-out overflow-hidden ${hasSelection ? 'max-h-0 opacity-0 mb-0' : 'max-h-60 opacity-100 mb-8'}`}
            >
              <div className="mb-4">
                <Avocado className="size-8" />
              </div>
              <h1 className="text-2xl sm:text-4xl font-light mb-3">
                {intl.formatMessage(i18n.welcomeTitle)}
              </h1>
              <p className="text-text-secondary text-base sm:text-lg">
                {intl.formatMessage(i18n.welcomeDescription)}
              </p>
            </div>

            <ProviderSelector
              onConfigured={handleConfigured}
              onFirstSelection={() => setHasSelection(true)}
              onConfigError={handleConfigError}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
