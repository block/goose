import { useState } from 'react';
import { acpAuthenticateProvider, acpRefreshProviderDetails } from '../../acp/providers';
import type { ProviderDetails } from '../../types/providers';
import DefaultProviderSetupForm, {
  ConfigInput,
} from '../settings/providers/modal/subcomponents/forms/DefaultProviderSetupForm';
import { providerConfigSubmitHandler } from '../settings/providers/modal/subcomponents/handlers/DefaultSubmitHandler';
import ProviderLogo from '../settings/providers/modal/subcomponents/ProviderLogo';
import { SecureStorageNotice } from '../settings/providers/modal/subcomponents/SecureStorageNotice';
import { Button } from '../ui/button';
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  LoaderCircle,
  LogIn,
  RefreshCw,
} from 'lucide-react';
import { defineMessages, useIntl } from '../../i18n';
import { errorMessage } from '../../utils/conversionUtils';

type OnConfigured = (name: string) => void | Promise<void>;

const i18n = defineMessages({
  browserWindowOpen: {
    id: 'providerConfigForm.browserWindowOpen',
    defaultMessage: 'A browser window will open for you to complete the login.',
  },
  deviceCodeFlowHint: {
    id: 'providerConfigForm.deviceCodeFlowHint',
    defaultMessage:
      'A browser window will open and the verification code will be copied to your clipboard. Paste it in the browser to complete sign-in.',
  },
  signingIn: {
    id: 'providerConfigForm.signingIn',
    defaultMessage: 'Signing in...',
  },
  signInWith: {
    id: 'providerConfigForm.signInWith',
    defaultMessage: 'Sign in with {providerName}',
  },
  noApiKey: {
    id: 'providerConfigForm.noApiKey',
    defaultMessage: "Don't have an API key?",
  },
  configuring: {
    id: 'providerConfigForm.configuring',
    defaultMessage: 'Configuring...',
  },
  continue: {
    id: 'providerConfigForm.continue',
    defaultMessage: 'Continue',
  },
  adapterFound: {
    id: 'providerConfigForm.adapterFound',
    defaultMessage: 'ACP adapter found',
  },
  adapterNotFound: {
    id: 'providerConfigForm.adapterNotFound',
    defaultMessage: 'ACP adapter not found',
  },
  connected: {
    id: 'providerConfigForm.connected',
    defaultMessage: 'Connected successfully',
  },
  connectionNotChecked: {
    id: 'providerConfigForm.connectionNotChecked',
    defaultMessage: 'Check your account connection before continuing.',
  },
  authenticationHelp: {
    id: 'providerConfigForm.authenticationHelp',
    defaultMessage: 'Sign in through the provider CLI, then check again.',
  },
  checkAgain: {
    id: 'providerConfigForm.checkAgain',
    defaultMessage: 'Check again',
  },
  checking: {
    id: 'providerConfigForm.checking',
    defaultMessage: 'Checking...',
  },
});

function parseLinks(text: string) {
  return text.split(/(https?:\/\/[^\s]+)/g).map((part, i) =>
    /^https?:\/\//.test(part) ? (
      <a
        key={i}
        href="#"
        onClick={(e) => {
          e.preventDefault();
          window.electron.openExternal(part);
        }}
        className="underline hover:text-text-default cursor-pointer"
      >
        {part}
      </a>
    ) : (
      part
    )
  );
}

function OAuthForm({
  provider,
  onConfigured,
  onError,
}: {
  provider: ProviderDetails;
  onConfigured: OnConfigured;
  onError: (msg: string) => void;
}) {
  const intl = useIntl();
  const [isLoading, setIsLoading] = useState(false);

  const handleLogin = async () => {
    setIsLoading(true);
    try {
      await acpAuthenticateProvider(provider.name);
      await onConfigured(provider.name);
    } catch (err) {
      onError(`Setup failed: ${errorMessage(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const isDeviceCodeFlow = provider.metadata.config_keys.some((key) => key.device_code_flow);

  return (
    <div className="flex flex-col items-center gap-3 py-4">
      <Button
        onClick={handleLogin}
        disabled={isLoading}
        className="flex items-center gap-2 px-6 py-3"
        size="lg"
      >
        <LogIn size={20} />
        {isLoading
          ? intl.formatMessage(i18n.signingIn)
          : intl.formatMessage(i18n.signInWith, { providerName: provider.metadata.display_name })}
      </Button>
      <p className="text-xs text-text-muted text-center">
        {isDeviceCodeFlow
          ? intl.formatMessage(i18n.deviceCodeFlowHint)
          : intl.formatMessage(i18n.browserWindowOpen)}
      </p>
    </div>
  );
}

function AcpProviderForm({
  provider,
  onConfigured,
  onError,
}: {
  provider: ProviderDetails;
  onConfigured: OnConfigured;
  onError: (msg: string) => void;
}) {
  const intl = useIntl();
  const [status, setStatus] = useState(provider);
  const [isChecking, setIsChecking] = useState(false);
  const [connectionChecked, setConnectionChecked] = useState(false);
  const [readinessError, setReadinessError] = useState<string | null>(null);
  const setupSteps = provider.metadata.setup_steps ?? [];
  const canContinue = status.is_configured && connectionChecked && !readinessError;

  const check = async () => {
    setIsChecking(true);
    try {
      const result = await acpRefreshProviderDetails(provider.name);
      setStatus(result.provider);
      setConnectionChecked(result.connectionChecked);
      setReadinessError(result.readinessError);
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setIsChecking(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="rounded-md border border-border-primary p-3 space-y-2">
        <div className="flex items-center gap-2 text-sm font-medium">
          {isChecking ? (
            <LoaderCircle className="h-4 w-4 animate-spin" />
          ) : status.is_configured ? (
            <CheckCircle2 className="h-4 w-4 text-green-600" />
          ) : (
            <CircleAlert className="h-4 w-4 text-yellow-600" />
          )}
          {isChecking
            ? intl.formatMessage(i18n.checking)
            : intl.formatMessage(status.is_configured ? i18n.adapterFound : i18n.adapterNotFound)}
        </div>
        {connectionChecked && !readinessError && (
          <div className="text-sm text-text-secondary">{intl.formatMessage(i18n.connected)}</div>
        )}
        {status.is_configured && !connectionChecked && !readinessError && (
          <div className="text-sm text-text-secondary">
            {intl.formatMessage(i18n.connectionNotChecked)}
          </div>
        )}
        {readinessError && (
          <div className="space-y-1 text-sm text-red-600 break-words">
            <div>{readinessError}</div>
            <div>{intl.formatMessage(i18n.authenticationHelp)}</div>
          </div>
        )}
        {connectionChecked && !readinessError && status.last_refresh_error && (
          <div className="text-sm text-yellow-600 break-words">{status.last_refresh_error}</div>
        )}
      </div>

      {(!status.is_configured || readinessError) && setupSteps.length > 0 && (
        <ol className="ml-5 list-decimal text-sm text-text-muted space-y-1">
          {setupSteps.map((step, i) => (
            <li key={i}>{parseLinks(step)}</li>
          ))}
        </ol>
      )}

      <div className="flex gap-2">
        <Button variant="outline" onClick={check} disabled={isChecking} className="flex-1">
          <RefreshCw className={`mr-2 h-4 w-4 ${isChecking ? 'animate-spin' : ''}`} />
          {intl.formatMessage(isChecking ? i18n.checking : i18n.checkAgain)}
        </Button>
        {canContinue && (
          <Button onClick={() => onConfigured(status.name)} className="flex-1">
            {intl.formatMessage(i18n.continue)}
          </Button>
        )}
      </div>
    </div>
  );
}

function ApiKeyForm({
  provider,
  onConfigured,
  onError,
}: {
  provider: ProviderDetails;
  onConfigured: OnConfigured;
  onError: (msg: string) => void;
}) {
  const intl = useIntl();
  const [configValues, setConfigValues] = useState<Record<string, ConfigInput>>({});
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showSetupHelp, setShowSetupHelp] = useState(false);
  const setupSteps = provider.metadata.setup_steps;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setValidationErrors({});

    const parameters = provider.metadata.config_keys || [];
    const errors: Record<string, string> = {};
    parameters.forEach((param) => {
      if (
        param.required &&
        !configValues[param.name]?.value &&
        !configValues[param.name]?.serverValue
      ) {
        errors[param.name] = `${param.name} is required`;
      }
    });

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    const toSubmit = Object.fromEntries(
      Object.entries(configValues)
        .filter(([, entry]) => !!entry.value)
        .map(([k, entry]) => [k, entry.value || ''])
    );

    setIsSubmitting(true);
    try {
      await providerConfigSubmitHandler(provider, toSubmit);
      await onConfigured(provider.name);
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      <DefaultProviderSetupForm
        configValues={configValues}
        setConfigValues={setConfigValues}
        provider={provider}
        validationErrors={validationErrors}
        showOptions={false}
      />
      {provider.metadata.config_keys.some((k) => k.required && k.secret) && <SecureStorageNotice />}
      {setupSteps && setupSteps.length > 0 && (
        <div className="mt-3">
          <button
            type="button"
            onClick={() => setShowSetupHelp(!showSetupHelp)}
            className="flex items-center gap-1 text-sm text-text-muted hover:text-text-default transition-colors"
          >
            <ChevronRight
              size={14}
              className={`transition-transform duration-200 ${showSetupHelp ? 'rotate-90' : ''}`}
            />
            {intl.formatMessage(i18n.noApiKey)}
          </button>
          {showSetupHelp && (
            <ol className="mt-2 ml-5 list-decimal text-sm text-text-muted space-y-1">
              {setupSteps.map((step, i) => (
                <li key={i}>{parseLinks(step)}</li>
              ))}
            </ol>
          )}
        </div>
      )}
      <div className="mt-4">
        <Button type="submit" disabled={isSubmitting} className="w-full">
          {isSubmitting ? intl.formatMessage(i18n.configuring) : intl.formatMessage(i18n.continue)}
        </Button>
      </div>
    </form>
  );
}

interface ProviderConfigFormProps {
  provider: ProviderDetails;
  onConfigured: OnConfigured;
}

export default function ProviderConfigForm({ provider, onConfigured }: ProviderConfigFormProps) {
  const [error, setError] = useState<string | null>(null);

  const isOAuthProvider = provider.metadata.config_keys.some((key) => key.oauth_flow);

  const renderForm = () => {
    if (provider.name.endsWith('-acp')) {
      return <AcpProviderForm provider={provider} onConfigured={onConfigured} onError={setError} />;
    }
    if (isOAuthProvider) {
      return <OAuthForm provider={provider} onConfigured={onConfigured} onError={setError} />;
    }
    return <ApiKeyForm provider={provider} onConfigured={onConfigured} onError={setError} />;
  };

  return (
    <div>
      <div className="p-4 border rounded-xl bg-background-muted">
        <div className="flex items-center gap-3 mb-4">
          <ProviderLogo providerName={provider.name} />
          <div>
            <h3 className="font-medium text-text-default">{provider.metadata.display_name}</h3>
            <p className="text-xs text-text-muted">{provider.metadata.description}</p>
          </div>
        </div>

        {renderForm()}

        {error && (
          <div className="mt-3 p-3 rounded-lg bg-red-50 text-red-800 border border-red-200 dark:bg-red-900/20 dark:text-red-200 dark:border-red-800 text-sm">
            {error}
          </div>
        )}
      </div>
    </div>
  );
}
