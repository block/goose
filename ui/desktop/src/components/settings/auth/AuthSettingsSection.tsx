import { useCallback, useEffect, useState } from 'react';
import { KeyRound, Loader2, LogIn, RefreshCw, Trash2 } from 'lucide-react';
import { toast } from 'react-toastify';
import {
  configureProviderOauth,
  deleteProviderSecret,
  listProviderSecrets,
  ProviderSecret,
} from '../../../api';
import { errorMessage } from '../../../utils/conversionUtils';
import { useModelAndProvider } from '../../ModelAndProviderContext';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { ConfirmationModal } from '../../ui/ConfirmationModal';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  title: {
    id: 'authSettings.title',
    defaultMessage: 'Provider Credentials',
  },
  description: {
    id: 'authSettings.description',
    defaultMessage: 'Manage provider credentials stored locally by goose.',
  },
  loading: {
    id: 'authSettings.loading',
    defaultMessage: 'Loading credentials...',
  },
  empty: {
    id: 'authSettings.empty',
    defaultMessage: 'No locally stored provider credentials were found.',
  },
  failedToLoad: {
    id: 'authSettings.failedToLoad',
    defaultMessage: 'Failed to load provider credentials',
  },
  deleteTitle: {
    id: 'authSettings.deleteTitle',
    defaultMessage: 'Delete credential',
  },
  deleteMessage: {
    id: 'authSettings.deleteMessage',
    defaultMessage: 'Delete the {name} credential for {provider}?',
  },
  activeProviderWarning: {
    id: 'authSettings.activeProviderWarning',
    defaultMessage: 'This is the active provider. New requests may fail until you configure another credential.',
  },
  delete: {
    id: 'authSettings.delete',
    defaultMessage: 'Delete',
  },
  cancel: {
    id: 'authSettings.cancel',
    defaultMessage: 'Cancel',
  },
  deleted: {
    id: 'authSettings.deleted',
    defaultMessage: 'Credential deleted',
  },
  failedToDelete: {
    id: 'authSettings.failedToDelete',
    defaultMessage: 'Failed to delete credential: {error}',
  },
  storageSecretStore: {
    id: 'authSettings.storageSecretStore',
    defaultMessage: 'Secret store',
  },
  storageProviderCache: {
    id: 'authSettings.storageProviderCache',
    defaultMessage: 'Provider cache',
  },
  expiresAt: {
    id: 'authSettings.expiresAt',
    defaultMessage: 'Expires {date}',
  },
  deleteCredential: {
    id: 'authSettings.deleteCredential',
    defaultMessage: 'Delete credential',
  },
  signIn: {
    id: 'authSettings.signIn',
    defaultMessage: 'Sign in',
  },
  reauthorize: {
    id: 'authSettings.reauthorize',
    defaultMessage: 'Reauthorize',
  },
  signedIn: {
    id: 'authSettings.signedIn',
    defaultMessage: 'Credential configured',
  },
  failedToConfigure: {
    id: 'authSettings.failedToConfigure',
    defaultMessage: 'Failed to configure credential: {error}',
  },
});

function storageLabel(secret: ProviderSecret, intl: ReturnType<typeof useIntl>) {
  if (secret.storage === 'provider_cache') {
    return intl.formatMessage(i18n.storageProviderCache);
  }
  return intl.formatMessage(i18n.storageSecretStore);
}

function expiryLabel(secret: ProviderSecret, intl: ReturnType<typeof useIntl>) {
  if (!secret.expires_at) {
    return null;
  }
  return intl.formatMessage(i18n.expiresAt, {
    date: intl.formatDate(new Date(secret.expires_at), {
      dateStyle: 'medium',
      timeStyle: 'short',
    }),
  });
}

function expiryClass(secret: ProviderSecret) {
  if (secret.status === 'expired') {
    return 'border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-300';
  }
  return 'border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-300';
}

export default function AuthSettingsSection() {
  const intl = useIntl();
  const { currentProvider } = useModelAndProvider();
  const [secrets, setSecrets] = useState<ProviderSecret[]>([]);
  const [loading, setLoading] = useState(true);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [configuringId, setConfiguringId] = useState<string | null>(null);
  const [secretToDelete, setSecretToDelete] = useState<ProviderSecret | null>(null);

  const loadSecrets = useCallback(async () => {
    setLoading(true);
    try {
      const response = await listProviderSecrets({ throwOnError: true });
      setSecrets(response.data?.secrets ?? []);
    } catch {
      toast.error(intl.formatMessage(i18n.failedToLoad));
      setSecrets([]);
    } finally {
      setLoading(false);
    }
  }, [intl]);

  useEffect(() => {
    loadSecrets();
  }, [loadSecrets]);

  const confirmDelete = async () => {
    if (!secretToDelete) {
      return;
    }

    setDeletingId(secretToDelete.id);
    try {
      await deleteProviderSecret({
        path: { id: secretToDelete.id },
        throwOnError: true,
      });
      toast.success(intl.formatMessage(i18n.deleted));
      setSecretToDelete(null);
      await loadSecrets();
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.failedToDelete, {
          error: errorMessage(error, 'Unknown error'),
        })
      );
    } finally {
      setDeletingId(null);
    }
  };

  const configureSecret = async (secret: ProviderSecret) => {
    if (!secret.configure_provider) {
      return;
    }

    setConfiguringId(secret.id);
    try {
      await configureProviderOauth({
        path: { name: secret.configure_provider },
        throwOnError: true,
      });
      toast.success(intl.formatMessage(i18n.signedIn));
      await loadSecrets();
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.failedToConfigure, {
          error: errorMessage(error, 'Unknown error'),
        })
      );
    } finally {
      setConfiguringId(null);
    }
  };

  const isActiveProvider = secretToDelete?.provider === currentProvider;

  return (
    <section id="auth" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="h-4 w-4" />
            {intl.formatMessage(i18n.title)}
          </CardTitle>
          <CardDescription>{intl.formatMessage(i18n.description)}</CardDescription>
        </CardHeader>
        <CardContent className="px-4 py-2">
          {loading ? (
            <div className="flex items-center gap-2 py-6 text-sm text-text-secondary">
              <Loader2 className="h-4 w-4 animate-spin" />
              {intl.formatMessage(i18n.loading)}
            </div>
          ) : secrets.length === 0 ? (
            <div className="py-6 text-sm text-text-secondary">{intl.formatMessage(i18n.empty)}</div>
          ) : (
            <div className="divide-y divide-border-primary">
              {secrets.map((secret) => (
                <div
                  key={secret.id}
                  className="flex flex-col gap-3 py-3 sm:flex-row sm:items-center sm:justify-between"
                  data-testid="auth-secret-row"
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-sm font-medium text-text-primary">
                        {secret.provider_display_name}
                      </h3>
                      <span className="rounded border border-border-primary bg-background-secondary px-2 py-0.5 text-xs text-text-secondary">
                        {storageLabel(secret, intl)}
                      </span>
                      {expiryLabel(secret, intl) && (
                        <span
                          className={`rounded border px-2 py-0.5 text-xs ${expiryClass(secret)}`}
                        >
                          {expiryLabel(secret, intl)}
                        </span>
                      )}
                    </div>
                    <p className="mt-1 break-all font-mono text-xs text-text-secondary">
                      {secret.name}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 self-start sm:self-auto">
                    {secret.can_configure && secret.configure_provider && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="gap-2"
                        disabled={configuringId === secret.id}
                        onClick={() => configureSecret(secret)}
                      >
                        {configuringId === secret.id ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : secret.has_secret || secret.configured ? (
                          <RefreshCw className="h-4 w-4" />
                        ) : (
                          <LogIn className="h-4 w-4" />
                        )}
                        {secret.has_secret || secret.configured
                          ? intl.formatMessage(i18n.reauthorize)
                          : intl.formatMessage(i18n.signIn)}
                      </Button>
                    )}
                    {secret.can_delete && (
                      <Button
                        variant="ghost"
                        size="sm"
                        shape="round"
                        className="text-text-secondary hover:text-text-primary"
                        disabled={deletingId === secret.id}
                        onClick={() => setSecretToDelete(secret)}
                        aria-label={intl.formatMessage(i18n.deleteCredential)}
                        title={intl.formatMessage(i18n.deleteCredential)}
                      >
                        {deletingId === secret.id ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Trash2 className="h-4 w-4" />
                        )}
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <ConfirmationModal
        isOpen={!!secretToDelete}
        title={intl.formatMessage(i18n.deleteTitle)}
        message={
          secretToDelete
            ? intl.formatMessage(i18n.deleteMessage, {
                name: secretToDelete.name,
                provider: secretToDelete.provider_display_name,
              })
            : ''
        }
        detail={isActiveProvider ? intl.formatMessage(i18n.activeProviderWarning) : undefined}
        onConfirm={confirmDelete}
        onCancel={() => setSecretToDelete(null)}
        confirmLabel={intl.formatMessage(i18n.delete)}
        cancelLabel={intl.formatMessage(i18n.cancel)}
        confirmVariant="destructive"
        isSubmitting={!!deletingId}
      />
    </section>
  );
}
