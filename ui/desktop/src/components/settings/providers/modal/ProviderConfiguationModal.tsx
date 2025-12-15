import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../ui/dialog';
import DefaultProviderSetupForm, {
  ConfigInput,
} from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import { SecureStorageNotice } from './subcomponents/SecureStorageNotice';
import { providerConfigSubmitHandler } from './subcomponents/handlers/DefaultSubmitHandler';
import { useConfig } from '../../../ConfigContext';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { AlertTriangle } from 'lucide-react';
import { ProviderDetails, removeCustomProvider } from '../../../../api';
import { Button } from '../../../../components/ui/button';

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

  const requiredParameters = provider.metadata.config_keys.filter(
    (param) => param.required === true
  );

  const isConfigured = provider.is_configured;
  const headerText = showDeleteConfirmation
    ? `Delete configuration for ${provider.metadata.display_name}`
    : `Configure ${provider.metadata.display_name}`;

  const descriptionText = showDeleteConfirmation
    ? isActiveProvider
      ? `You cannot delete this provider while it's currently in use. Please switch to a different model first.`
      : 'This will permanently delete the current provider configuration.'
    : `Add your API key(s) for this provider to integrate into Goose`;

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
      setError(`${error}`);
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
            ) : null}
          </div>

          <DialogFooter>
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
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
