import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { Avocado } from '../icons';
import { Button } from '../ui/button';
import { defineMessages, useIntl } from '../../i18n';
import { AccessDenied } from './AccessDenied';
import type { AuthStatus } from '../../auth';

const i18n = defineMessages({
  welcomeTitle: {
    id: 'loginGuard.welcomeTitle',
    defaultMessage: 'Welcome to Avocado Work',
  },
  welcomeDescription: {
    id: 'loginGuard.welcomeDescription',
    defaultMessage:
      'Sign in with your Avocado account to continue. Sessions and credentials stay private to your user.',
  },
  signIn: {
    id: 'loginGuard.signIn',
    defaultMessage: 'Sign in with Avocado',
  },
  signingInTitle: {
    id: 'loginGuard.signingInTitle',
    defaultMessage: 'Complete sign-in in your browser',
  },
  signingInDescription: {
    id: 'loginGuard.signingInDescription',
    defaultMessage:
      'We opened Zitadel in your system browser. Finish login there, then return to this window.',
  },
  waitingCallback: {
    id: 'loginGuard.waitingCallback',
    defaultMessage: 'Waiting for callback on 127.0.0.1:47821',
  },
  cancel: {
    id: 'loginGuard.cancel',
    defaultMessage: 'Cancel',
  },
  authDisabledPassThrough: {
    id: 'loginGuard.authDisabled',
    defaultMessage: 'Auth disabled',
  },
});

type LoginGuardProps = {
  children: ReactNode;
};

function isAuthApiAvailable(): boolean {
  return typeof window !== 'undefined' && Boolean(window.electron?.getAuthStatus);
}

export function LoginGuard({ children }: LoginGuardProps) {
  const intl = useIntl();
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (!isAuthApiAvailable()) {
      setStatus({ state: 'disabled' });
      return;
    }
    const next = (await window.electron.getAuthStatus()) as AuthStatus;
    setStatus(next);
  }, []);

  useEffect(() => {
    void refresh();
    if (!isAuthApiAvailable()) return;
    const handler = (_event: Electron.IpcRendererEvent, next: unknown) => {
      setStatus(next as AuthStatus);
    };
    window.electron.on('auth:on-changed', handler);
    return () => {
      window.electron.off('auth:on-changed', handler);
    };
  }, [refresh]);

  const signIn = async () => {
    if (!isAuthApiAvailable()) return;
    setBusy(true);
    try {
      const next = (await window.electron.authLogin()) as AuthStatus;
      setStatus(next);
    } catch (error) {
      console.error('Login failed', error);
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const signOut = async () => {
    if (!isAuthApiAvailable()) return;
    setBusy(true);
    try {
      await window.electron.authLogout();
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  if (!status || status.state === 'disabled') {
    return <>{children}</>;
  }

  if (status.state === 'signedIn') {
    return <>{children}</>;
  }

  if (status.state === 'accessDenied') {
    return (
      <AccessDenied
        email={status.email}
        onSwitchAccount={() => void signOut()}
        onRetry={() => void signIn()}
      />
    );
  }

  if (status.state === 'signingIn') {
    return (
      <div className="h-screen w-full bg-background-default flex flex-col items-center justify-center">
        <div className="max-w-md w-full px-4">
          <div className="mb-4">
            <Avocado className="size-8" />
          </div>
          <h1 className="text-2xl font-light mb-3">{intl.formatMessage(i18n.signingInTitle)}</h1>
          <p className="text-text-muted mb-4">{intl.formatMessage(i18n.signingInDescription)}</p>
          <p className="text-xs text-text-muted mb-6">{intl.formatMessage(i18n.waitingCallback)}</p>
          <Button variant="outline" disabled={busy} onClick={() => void signOut()}>
            {intl.formatMessage(i18n.cancel)}
          </Button>
        </div>
      </div>
    );
  }

  // signedOut
  return (
    <div className="h-screen w-full bg-background-default flex flex-col items-center justify-center">
      <div className="max-w-md w-full px-4">
        <div className="mb-4">
          <Avocado className="size-8" />
        </div>
        <h1 className="text-2xl sm:text-4xl font-light mb-3">
          {intl.formatMessage(i18n.welcomeTitle)}
        </h1>
        <p className="text-text-muted text-base sm:text-lg mb-8">
          {intl.formatMessage(i18n.welcomeDescription)}
        </p>
        <Button onClick={() => void signIn()} disabled={busy}>
          {intl.formatMessage(i18n.signIn)}
        </Button>
      </div>
    </div>
  );
}

export default LoginGuard;
