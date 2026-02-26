import { AlertTriangle, LogIn } from 'lucide-react';
import { useState } from 'react';
import type { ProviderDetails } from '@/api';
import { configureProviderOauth, removeCustomProvider } from '@/api';
import { Button } from '@/components/atoms/button';
import { useConfig } from '@/contexts/ConfigContext';
import { useModelAndProvider } from '@/contexts/ModelAndProviderContext';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/molecules/ui/dialog';
import type { ConfigInput } from './subcomponents/forms/DefaultProviderSetupForm';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import { providerConfigSubmitHandler } from './subcomponents/handlers/DefaultSubmitHandler';
import ProviderLogo from './subcomponents/ProviderLogo';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import { SecureStorageNotice } from './subcomponents/SecureStorageNotice';

const formatErrorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
};

interface ProviderConfigurationModalProps {
  provider: ProviderDetails;
  onClose: () => void;
  onConfigured?: (provider: ProviderDetails) => void;
}

export default function ProviderConfigurationModal({
  provider,
  onClose,
  onConfigured,
}: ProviderConfigurationModalProps) {
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const { upsert, remove } = useConfig();
  const { getCurrentModelAndProvider } = useModelAndProvider();
  const [configValues, setConfigValues] = useState<Record<string, ConfigInput>>({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [isActiveProvider, setIsActiveProvider] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOAuthLoading, setIsOAuthLoading] = useState(false);

  const requiredParameters = provider.metadata.config_keys.filter(
    (param) => param.required === true
  );

  // Check if this provider uses OAuth for configuration
  const isOAuthProvider = provider.metadata.config_keys.some((key) => key.oauth_flow);

  const isConfigured = provider.is_configured;
  const headerText = showDeleteConfirmation
    ? `Delete configuration for ${provider.metadata.display_name}`
    : `Configure ${provider.metadata.display_name}`;

  const descriptionText = showDeleteConfirmation
    ? isActiveProvider
      ? `You cannot delete this provider while it's currently in use. Please switch to a different model first.`
      : 'This will permanently delete the current provider configuration.'
    : isOAuthProvider
      ? `Sign in with your ${provider.metadata.display_name} account to use this provider`
      : `Add your API key(s) for this provider to integrate into goose`;

  const handleOAuthLogin = async () => {
    setIsOAuthLoading(true);
    setError(null);
    try {
      await configureProviderOauth({
        path: { name: provider.name },
      });
      if (onConfigured) {
        onConfigured(provider);
      } else {
        onClose();
      }
    } catch (err) {
      setError(`OAuth login failed: ${formatErrorMessage(err)}`);
    } finally {
      setIsOAuthLoading(false);
    }
  };

  const handleSubmitForm = async (e: React.FormEvent) => {
    e.preventDefault();

    setValidationErrors({});

    const parameters = provider.metadata.config_keys || [];
    const errors: Record<string, string> = {};

    parameters.forEach((parameter) => {
      if (
        parameter.required &&
        !configValues[parameter.name]?.value &&
        !configValues[parameter.name]?.serverValue
      ) {
        errors[parameter.name] = `${parameter.name} is required`;
      }
    });

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    const toSubmit = Object.fromEntries(
      Object.entries(configValues)
        .filter(([_k, entry]) => !!entry.value)
        .map(([k, entry]) => [k, entry.value || ''])
    );

    try {
      await providerConfigSubmitHandler(upsert, provider, toSubmit);
      if (onConfigured) {
        onConfigured(provider);
      } else {
        onClose();
      }
    } catch (error) {
      setError(formatErrorMessage(error));
    }
  };

  const handleCancel = () => {
    onClose();
  };

  const handleDelete = async () => {
    try {
      const providerModel = await getCurrentModelAndProvider();
      if (provider.name === providerModel.provider) {
        setIsActiveProvider(true);
        setShowDeleteConfirmation(true);
        return;
      }
    } catch (error) {
      console.error('Failed to check current provider:', error);
    }

    setIsActiveProvider(false);
    setShowDeleteConfirmation(true);
  };

  const handleConfirmDelete = async () => {
    if (isActiveProvider) {
      return;
    }

    const isCustomProvider = provider.provider_type === 'Custom';

    if (isCustomProvider) {
      await removeCustomProvider({
        path: { id: provider.name },
      });
    } else {
      const params = provider.metadata.config_keys;
      for (const param of params) {
        await remove(param.name, param.secret);
      }
    }

    onClose();
  };

  const getModalIcon = () => {
    if (showDeleteConfirmation) {
      return (
        <AlertTriangle
          className={isActiveProvider ? 'text-yellow-500' : 'text-red-500'}
          size={24}
        />
      );
    }
    return <ProviderLogo providerName={provider.name} />;
  };

  return (
    <>
      <Dialog open={!!error} onOpenChange={(open) => !open && setError(null)}>
        <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
          <DialogTitle className="flex items-center gap-2">Error</DialogTitle>
          <DialogDescription className="text-inherit text-base">
            There was an error checking this provider configuration.
          </DialogDescription>
          <pre className="ml-2">{error}</pre>
          <div>Check your configuration again to use this provider.</div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setError(null)}>
              Go Back
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog open={!error} onOpenChange={(open) => !open && onClose()}>
        <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              {getModalIcon()}
              {headerText}
            </DialogTitle>
            <DialogDescription>{descriptionText}</DialogDescription>
          </DialogHeader>

          <div className="py-4">
            {/* Contains information used to set up each provider */}
            {/* Only show the form when NOT in delete confirmation mode */}
            {!showDeleteConfirmation ? (
              isOAuthProvider ? (
                <div className="flex flex-col items-center gap-4 py-6">
                  <Button
                    onClick={handleOAuthLogin}
                    disabled={isOAuthLoading}
                    className="flex items-center gap-2 px-6 py-3"
                    size="lg"
                  >
                    <LogIn size={20} />
                    {isOAuthLoading
                      ? 'Signing in...'
                      : `Sign in with ${provider.metadata.display_name}`}
                  </Button>
                  <p className="text-sm text-text-muted text-center">
                    A browser window will open for you to complete the login.
                  </p>
                </div>
              ) : (
                <>
                  {/* Contains information used to set up each provider */}
                  <DefaultProviderSetupForm
                    configValues={configValues}
                    setConfigValues={setConfigValues}
                    provider={provider}
                    validationErrors={validationErrors}
                  />

                  {requiredParameters.length > 0 &&
                    provider.metadata.config_keys &&
                    provider.metadata.config_keys.length > 0 && <SecureStorageNotice />}
                </>
              )
            ) : null}
          </div>

          <DialogFooter>
            {isOAuthProvider && !showDeleteConfirmation ? (
              <div className="flex gap-2">
                <Button variant="outline" onClick={handleCancel}>
                  Cancel
                </Button>
                {isConfigured && (
                  <Button variant="destructive" onClick={handleDelete}>
                    Remove Configuration
                  </Button>
                )}
              </div>
            ) : (
              <ProviderSetupActions
                requiredParameters={requiredParameters}
                onCancel={handleCancel}
                onSubmit={handleSubmitForm}
                onDelete={handleDelete}
                showDeleteConfirmation={showDeleteConfirmation}
                onConfirmDelete={handleConfirmDelete}
                onCancelDelete={() => {
                  setIsActiveProvider(false);
                  setShowDeleteConfirmation(false);
                }}
                canDelete={isConfigured && !isActiveProvider}
                providerName={provider.metadata.display_name}
                isActiveProvider={isActiveProvider}
              />
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
