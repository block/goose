import { useCallback, useEffect, useState } from 'react';
import { Loader2, LogIn } from 'lucide-react';
import { toast } from 'react-toastify';
import { configureProviderOauth, listProviderSecrets } from '../../../api';
import { errorMessage } from '../../../utils/conversionUtils';
import { defineMessages, useIntl } from '../../../i18n';
import { Button } from '../../ui/button';

const HUGGINGFACE_PROVIDER = 'huggingface';
const HUGGINGFACE_OAUTH_SECRET_ID = 'provider_cache:huggingface';

const i18n = defineMessages({
  title: {
    id: 'huggingFaceSignInPrompt.title',
    defaultMessage: 'Hugging Face',
  },
  signIn: {
    id: 'huggingFaceSignInPrompt.signIn',
    defaultMessage: 'Sign in',
  },
  signingIn: {
    id: 'huggingFaceSignInPrompt.signingIn',
    defaultMessage: 'Signing in...',
  },
  signedIn: {
    id: 'huggingFaceSignInPrompt.signedIn',
    defaultMessage: 'Hugging Face signed in',
  },
  failedToConfigure: {
    id: 'huggingFaceSignInPrompt.failedToConfigure',
    defaultMessage: 'Failed to sign in to Hugging Face: {error}',
  },
});

interface HuggingFaceSignInPromptProps {
  description: string;
  className?: string;
  onSignedIn?: () => void;
}

export default function HuggingFaceSignInPrompt({
  description,
  className,
  onSignedIn,
}: HuggingFaceSignInPromptProps) {
  const intl = useIntl();
  const [loading, setLoading] = useState(true);
  const [loggedIn, setLoggedIn] = useState(false);
  const [signingIn, setSigningIn] = useState(false);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    try {
      const response = await listProviderSecrets({ throwOnError: true });
      const huggingFaceSecret = response.data?.secrets.find(
        (secret) => secret.id === HUGGINGFACE_OAUTH_SECRET_ID
      );
      setLoggedIn(Boolean(huggingFaceSecret?.has_secret && huggingFaceSecret.status !== 'expired'));
    } catch {
      setLoggedIn(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const signIn = async () => {
    setSigningIn(true);
    try {
      await configureProviderOauth({
        path: { name: HUGGINGFACE_PROVIDER },
        throwOnError: true,
      });
      toast.success(intl.formatMessage(i18n.signedIn));
      setLoggedIn(true);
      onSignedIn?.();
    } catch (error) {
      toast.error(
        intl.formatMessage(i18n.failedToConfigure, {
          error: errorMessage(error, 'Unknown error'),
        })
      );
      await loadStatus();
    } finally {
      setSigningIn(false);
    }
  };

  if (loading || loggedIn) {
    return null;
  }

  return (
    <div
      className={`flex flex-col gap-3 rounded-lg border border-border-subtle bg-background-default p-3 sm:flex-row sm:items-center sm:justify-between ${className ?? ''}`}
    >
      <div className="min-w-0">
        <h4 className="text-sm font-medium text-text-default">{intl.formatMessage(i18n.title)}</h4>
        <p className="mt-1 text-xs text-text-muted">{description}</p>
      </div>
      <Button
        variant="outline"
        size="sm"
        className="gap-2 self-start sm:self-auto"
        disabled={signingIn}
        onClick={signIn}
      >
        {signingIn ? <Loader2 className="h-4 w-4 animate-spin" /> : <LogIn className="h-4 w-4" />}
        {signingIn ? intl.formatMessage(i18n.signingIn) : intl.formatMessage(i18n.signIn)}
      </Button>
    </div>
  );
}
