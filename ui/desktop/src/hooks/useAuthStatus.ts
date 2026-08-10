import { useCallback, useEffect, useState } from 'react';
import type { AuthStatus } from '../auth';

export function isAuthApiAvailable(): boolean {
  return typeof window !== 'undefined' && Boolean(window.electron?.getAuthStatus);
}

export interface UseAuthStatus {
  status: AuthStatus | null;
  busy: boolean;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
  refresh: () => Promise<void>;
}

/**
 * Subscribes to Zitadel auth state from the main process. Returns a `disabled`
 * status when the auth IPC surface is absent, so callers can render normally in
 * builds without Zitadel.
 */
export function useAuthStatus(): UseAuthStatus {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (!isAuthApiAvailable()) {
      setStatus({ state: 'disabled' });
      return;
    }
    setStatus((await window.electron.getAuthStatus()) as AuthStatus);
  }, []);

  useEffect(() => {
    void refresh();
    if (!isAuthApiAvailable()) return;
    const handler = (_event: unknown, next: unknown) => {
      setStatus(next as AuthStatus);
    };
    window.electron.on('auth:on-changed', handler);
    return () => {
      window.electron.off('auth:on-changed', handler);
    };
  }, [refresh]);

  const signIn = useCallback(async () => {
    if (!isAuthApiAvailable()) return;
    setBusy(true);
    try {
      setStatus((await window.electron.authLogin()) as AuthStatus);
    } catch (error) {
      console.error('Login failed', error);
      await refresh();
    } finally {
      setBusy(false);
    }
  }, [refresh]);

  const signOut = useCallback(async () => {
    if (!isAuthApiAvailable()) return;
    setBusy(true);
    try {
      await window.electron.authLogout();
      await refresh();
    } finally {
      setBusy(false);
    }
  }, [refresh]);

  return { status, busy, signIn, signOut, refresh };
}
