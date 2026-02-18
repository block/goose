import { useState } from 'react';
import { configureProviderOauth, ProviderDetails } from '../../api';
import { useConfig } from '../ConfigContext';
import DefaultProviderSetupForm, {
  ConfigInput,
} from '../settings/providers/modal/subcomponents/forms/DefaultProviderSetupForm';
import { providerConfigSubmitHandler } from '../settings/providers/modal/subcomponents/handlers/DefaultSubmitHandler';
import ProviderLogo from '../settings/providers/modal/subcomponents/ProviderLogo';
import { SecureStorageNotice } from '../settings/providers/modal/subcomponents/SecureStorageNotice';
import { Button } from '../ui/button';
import { LogIn, ChevronRight } from 'lucide-react';

function OllamaForm({ onSetup }: { onSetup: () => void }) {
  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-text-muted">Ollama runs AI models locally on your computer.</p>
      <Button onClick={onSetup}>Set up Ollama</Button>
    </div>
  );
}

function OAuthForm({
  provider,
  onConfigured,
  onError,
}: {
  provider: ProviderDetails;
  onConfigured: (name: string) => void;
  onError: (msg: string) => void;
}) {
  const [isLoading, setIsLoading] = useState(false);

  const handleLogin = async () => {
    setIsLoading(true);
    try {
      await configureProviderOauth({
        path: { name: provider.name },
        throwOnError: true,
      });
      onConfigured(provider.name);
    } catch (err) {
      onError(`Sign-in failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center gap-3 py-4">
      <Button
        onClick={handleLogin}
        disabled={isLoading}
        className="flex items-center gap-2 px-6 py-3"
        size="lg"
      >
        <LogIn size={20} />
        {isLoading ? 'Signing in...' : `Sign in with ${provider.metadata.display_name}`}
      </Button>
      <p className="text-xs text-text-muted text-center">
        A browser window will open for you to complete the login.
      </p>
    </div>
  );
}

function ApiKeyForm({
  provider,
  onConfigured,
  onError,
}: {
  provider: ProviderDetails;
  onConfigured: (name: string) => void;
  onError: (msg: string) => void;
}) {
  const { upsert } = useConfig();
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
      await providerConfigSubmitHandler(upsert, provider, toSubmit);
      await upsert('GOOSE_PROVIDER', provider.name, false);
      onConfigured(provider.name);
    } catch (err) {
      onError(`Configuration failed: ${err instanceof Error ? err.message : String(err)}`);
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
            Don't have an API key?
          </button>
          {showSetupHelp && (
            <ol className="mt-2 ml-5 list-decimal text-sm text-text-muted space-y-1">
              {setupSteps.map((step, i) => (
                <li key={i}>{step}</li>
              ))}
            </ol>
          )}
        </div>
      )}
      <div className="mt-4">
        <Button type="submit" disabled={isSubmitting} className="w-full">
          {isSubmitting ? 'Configuring...' : 'Continue'}
        </Button>
      </div>
    </form>
  );
}

interface ProviderConfigFormProps {
  provider: ProviderDetails;
  onConfigured: (providerName: string) => void;
  onOllamaSetup: () => void;
}

export default function ProviderConfigForm({
  provider,
  onConfigured,
  onOllamaSetup,
}: ProviderConfigFormProps) {
  const [error, setError] = useState<string | null>(null);

  const isOAuthProvider = provider.metadata.config_keys.some((key) => key.oauth_flow);
  const isOllama = provider.name === 'ollama';

  const renderForm = () => {
    if (isOllama) {
      return <OllamaForm onSetup={onOllamaSetup} />;
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
