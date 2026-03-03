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
import { AlertTriangle, LogIn } from 'lucide-react';
import { ProviderDetails, removeCustomProvider, configureProviderOauth } from '../../../../api';
import { Button } from '../../../../components/ui/button';
import { errorMessage } from '../../../../utils/conversionUtils';
import { useTranslation } from 'react-i18next';

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
  const { t } = useTranslation();
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const { upsert, remove } = useConfig();
  const { getCurrentModelAndProvider } = useModelAndProvider();
  const [configValues, setConfigValues] = useState<Record<string, ConfigInput>>({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [isActiveProvider, setIsActiveProvider] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOAuthLoading, setIsOAuthLoading] = useState(false);

  let primaryParameters = provider.metadata.config_keys.filter((param) => param.primary);
  if (primaryParameters.length === 0) {
    primaryParameters = provider.metadata.config_keys;
  }

  // Check if this provider uses OAuth for configuration
  const isOAuthProvider = provider.metadata.config_keys.some((key) => key.oauth_flow);

  const isConfigured = provider.is_configured;
  const headerText = showDeleteConfirmation
    ? t('providerModal.deleteConfigurationFor', { provider: provider.metadata.display_name })
    : t('providerModal.configureProvider', { provider: provider.metadata.display_name });

  const descriptionText = showDeleteConfirmation
    ? isActiveProvider
      ? t('providerModal.cannotDeleteActive')
      : t('providerModal.deletePermanent')
    : isOAuthProvider
      ? t('providerModal.oauth.signInDescription', { provider: provider.metadata.display_name })
      : t('providerModal.apiKeyDescription');

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
      setError(t('providerModal.oauth.loginFailed', { error: errorMessage(err) }));
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
        errors[parameter.name] = t('providerModal.fieldRequired', { field: parameter.name });
      }
    });

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    const toSubmit = Object.fromEntries(
      Object.entries(configValues)
        .filter(
          ([_k, entry]) =>
            !!entry.value || (entry.serverValue != null && typeof entry.serverValue === 'string')
        )
        .map(([k, entry]) => [
          k,
          entry.value ?? (typeof entry.serverValue === 'string' ? entry.serverValue : ''),
        ])
    );

    try {
      await providerConfigSubmitHandler(upsert, provider, toSubmit);
      if (onConfigured) {
        onConfigured(provider);
      } else {
        onClose();
      }
    } catch (error) {
      setError(errorMessage(error));
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
          <DialogTitle className="flex items-center gap-2">{t('providerModal.error.title')}</DialogTitle>
          <DialogDescription className="text-inherit text-base">
            {t('providerModal.error.description')}
          </DialogDescription>
          <pre className="ml-2">{error}</pre>
          <div>{t('providerModal.error.guidance')}</div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setError(null)}>
              {t('providerModal.error.goBack')}
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
                    data-testid="provider-oauth-sign-in-button"
                  >
                    <LogIn size={20} />
                    {isOAuthLoading
                      ? t('providerModal.oauth.signingIn')
                      : t('providerModal.oauth.signInWith', {
                          provider: provider.metadata.display_name,
                        })}
                  </Button>
                  <p className="text-sm text-text-secondary text-center">
                    {t('providerModal.oauth.browserInstruction')}
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

                  {primaryParameters.length > 0 &&
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
                  {t('common.cancel')}
                </Button>
                {isConfigured && (
                  <Button variant="destructive" onClick={handleDelete}>
                    {t('providerModal.removeConfiguration')}
                  </Button>
                )}
              </div>
            ) : (
              <ProviderSetupActions
                primaryParameters={primaryParameters}
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
