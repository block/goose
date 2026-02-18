import React, { createContext, useContext, useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  authStatus as apiAuthStatus,
  getUserInfo as apiGetUserInfo,
  login as apiLogin,
  logout as apiLogout,
  oidcAuthUrl as apiOidcAuthUrl,
  oidcCodeExchange as apiOidcCodeExchange,
  listOidcProviders as apiListOidcProviders,
} from '../api';

interface User {
  id: string;
  name: string;
  auth_method: string;
  is_guest: boolean;
  tenant?: string;
}

interface OidcProvider {
  issuer: string;
  audience: string;
}

interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  authRequired: boolean;
  oidcProviders: OidcProvider[];
  error: string | null;
}

interface AuthContextValue extends AuthState {
  loginWithApiKey: (apiKey: string) => Promise<void>;
  loginWithOidc: (issuer: string) => Promise<void>;
  logout: () => Promise<void>;
  clearError: () => void;
}

const TOKEN_KEY = 'goose_auth_token';
const TOKEN_EXPIRY_KEY = 'goose_auth_token_expiry';

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const refreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [state, setState] = useState<AuthState>({
    user: null,
    token: localStorage.getItem(TOKEN_KEY),
    isAuthenticated: false,
    isLoading: true,
    authRequired: false,
    oidcProviders: [],
    error: null,
  });

  const clearRefreshTimer = useCallback(() => {
    if (refreshTimerRef.current) {
      clearTimeout(refreshTimerRef.current);
      refreshTimerRef.current = null;
    }
  }, []);

  const scheduleTokenRefresh = useCallback(
    (expiresIn: number) => {
      clearRefreshTimer();
      // Refresh 60 seconds before expiry, minimum 10 seconds
      const refreshIn = Math.max((expiresIn - 60) * 1000, 10_000);
      refreshTimerRef.current = setTimeout(async () => {
        const currentToken = localStorage.getItem(TOKEN_KEY);
        if (!currentToken) return;
        try {
          const { data } = await apiLogin({
            body: { api_key: undefined },
            headers: { Authorization: `Bearer ${currentToken}` },
          });
          if (data?.token) {
            localStorage.setItem(TOKEN_KEY, data.token);
            if (data.expires_in) {
              localStorage.setItem(
                TOKEN_EXPIRY_KEY,
                String(Date.now() + data.expires_in * 1000)
              );
              scheduleTokenRefresh(data.expires_in);
            }
          }
        } catch {
          // Token refresh failed — user will need to re-authenticate on next action
          console.warn('Token refresh failed');
        }
      }, refreshIn);
    },
    [clearRefreshTimer]
  );

  // Check auth status on mount
  useEffect(() => {
    (async () => {
      try {
        const { data: status } = await apiAuthStatus();
        if (!status?.oidc_enabled && status?.provider_count === 0) {
          // Auth not configured — skip auth
          setState((prev) => ({
            ...prev,
            isLoading: false,
            authRequired: false,
            isAuthenticated: true,
          }));
          return;
        }

        // Auth is configured — check providers
        const providers: OidcProvider[] = [];
        if (status?.oidc_enabled) {
          try {
            const { data: providerList } = await apiListOidcProviders();
            if (providerList?.providers) {
              for (const p of providerList.providers) {
                providers.push({ issuer: p.issuer, audience: p.audience });
              }
            }
          } catch {
            // Failed to list providers — continue without them
          }
        }

        // Try existing token
        const savedToken = localStorage.getItem(TOKEN_KEY);
        if (savedToken) {
          try {
            const { data: userInfo } = await apiGetUserInfo({
              headers: { Authorization: `Bearer ${savedToken}` },
            });
            if (userInfo?.is_authenticated) {
              setState({
                user: {
                  id: userInfo.id,
                  name: userInfo.name,
                  auth_method: userInfo.auth_method,
                  is_guest: userInfo.is_guest,
                  tenant: userInfo.tenant ?? undefined,
                },
                token: savedToken,
                isAuthenticated: true,
                isLoading: false,
                authRequired: true,
                oidcProviders: providers,
                error: null,
              });
              // Schedule refresh based on stored expiry
              const expiry = localStorage.getItem(TOKEN_EXPIRY_KEY);
              if (expiry) {
                const remaining = Math.floor((Number(expiry) - Date.now()) / 1000);
                if (remaining > 0) {
                  scheduleTokenRefresh(remaining);
                }
              }
              return;
            }
          } catch {
            // Token invalid — clear and continue to login
            localStorage.removeItem(TOKEN_KEY);
            localStorage.removeItem(TOKEN_EXPIRY_KEY);
          }
        }

        setState((prev) => ({
          ...prev,
          isLoading: false,
          authRequired: true,
          oidcProviders: providers,
        }));
      } catch {
        // Auth status check failed — assume no auth required (server might not support it)
        setState((prev) => ({
          ...prev,
          isLoading: false,
          authRequired: false,
          isAuthenticated: true,
        }));
      }
    })();
  }, [scheduleTokenRefresh]);

  // Listen for OIDC callback via deep link
  useEffect(() => {
    const handleOidcCallback = async (_event: unknown, ...args: unknown[]) => {
      const url = args[0] as string;
      if (!url?.includes('goose://auth/callback')) return;

      try {
        const parsed = new URL(url);
        const code = parsed.searchParams.get('code');
        const returnedState = parsed.searchParams.get('state');
        const savedState = sessionStorage.getItem('oidc_state');
        const savedIssuer = sessionStorage.getItem('oidc_issuer');
        const savedRedirectUri = sessionStorage.getItem('oidc_redirect_uri');

        if (!code || !returnedState || returnedState !== savedState) {
          setState((prev) => ({
            ...prev,
            error: 'OIDC callback state mismatch. Please try again.',
            isLoading: false,
          }));
          return;
        }

        setState((prev) => ({ ...prev, isLoading: true, error: null }));

        const { data } = await apiOidcCodeExchange({
          body: {
            code,
            issuer: savedIssuer ?? '',
            redirect_uri: savedRedirectUri ?? '',
          },
        });

        if (data?.token) {
          localStorage.setItem(TOKEN_KEY, data.token);
          if (data.expires_in) {
            localStorage.setItem(
              TOKEN_EXPIRY_KEY,
              String(Date.now() + data.expires_in * 1000)
            );
            scheduleTokenRefresh(data.expires_in);
          }

          setState({
            user: data.user
              ? {
                  id: data.user.id,
                  name: data.user.name,
                  auth_method: data.user.auth_method,
                  is_guest: data.user.is_guest,
                  tenant: data.user.tenant ?? undefined,
                }
              : null,
            token: data.token,
            isAuthenticated: true,
            isLoading: false,
            authRequired: true,
            oidcProviders: state.oidcProviders,
            error: null,
          });
          navigate('/', { replace: true });
        }
      } catch (err) {
        setState((prev) => ({
          ...prev,
          error: `OIDC login failed: ${err instanceof Error ? err.message : 'Unknown error'}`,
          isLoading: false,
        }));
      } finally {
        sessionStorage.removeItem('oidc_state');
        sessionStorage.removeItem('oidc_issuer');
        sessionStorage.removeItem('oidc_redirect_uri');
      }
    };

    if (typeof window !== 'undefined' && window.electron) {
      window.electron.on('oidc-callback', handleOidcCallback);
      return () => {
        window.electron.off('oidc-callback', handleOidcCallback);
      };
    }
    return undefined;
  }, [navigate, scheduleTokenRefresh, state.oidcProviders]);

  // Cleanup refresh timer on unmount
  useEffect(() => {
    return clearRefreshTimer;
  }, [clearRefreshTimer]);

  const loginWithApiKey = useCallback(
    async (apiKey: string) => {
      setState((prev) => ({ ...prev, isLoading: true, error: null }));
      try {
        const { data } = await apiLogin({ body: { api_key: apiKey } });
        if (data?.token) {
          localStorage.setItem(TOKEN_KEY, data.token);
          if (data.expires_in) {
            localStorage.setItem(
              TOKEN_EXPIRY_KEY,
              String(Date.now() + data.expires_in * 1000)
            );
            scheduleTokenRefresh(data.expires_in);
          }
          setState((prev) => ({
            ...prev,
            user: data.user
              ? {
                  id: data.user.id,
                  name: data.user.name,
                  auth_method: data.user.auth_method,
                  is_guest: data.user.is_guest,
                  tenant: data.user.tenant ?? undefined,
                }
              : null,
            token: data.token,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          }));
          navigate('/', { replace: true });
        } else {
          setState((prev) => ({
            ...prev,
            error: 'Login failed: no token received',
            isLoading: false,
          }));
        }
      } catch (err) {
        setState((prev) => ({
          ...prev,
          error: `Login failed: ${err instanceof Error ? err.message : 'Invalid API key'}`,
          isLoading: false,
        }));
      }
    },
    [navigate, scheduleTokenRefresh]
  );

  const loginWithOidc = useCallback(
    async (issuer: string) => {
      setState((prev) => ({ ...prev, isLoading: true, error: null }));
      try {
        const redirectUri = 'goose://auth/callback';
        const { data } = await apiOidcAuthUrl({
          body: { issuer, redirect_uri: redirectUri },
        });

        if (data?.auth_url) {
          // Store state for callback verification
          sessionStorage.setItem('oidc_state', data.state);
          sessionStorage.setItem('oidc_issuer', issuer);
          sessionStorage.setItem('oidc_redirect_uri', redirectUri);

          // Open auth URL in system browser
          await window.electron.openExternal(data.auth_url);

          setState((prev) => ({
            ...prev,
            isLoading: false, // User is now in the browser
            error: null,
          }));
        } else {
          setState((prev) => ({
            ...prev,
            error: 'Failed to generate authorization URL',
            isLoading: false,
          }));
        }
      } catch (err) {
        setState((prev) => ({
          ...prev,
          error: `OIDC login failed: ${err instanceof Error ? err.message : 'Unknown error'}`,
          isLoading: false,
        }));
      }
    },
    []
  );

  const logoutFn = useCallback(async () => {
    const currentToken = localStorage.getItem(TOKEN_KEY);
    clearRefreshTimer();

    try {
      if (currentToken) {
        await apiLogout({
          headers: { Authorization: `Bearer ${currentToken}` },
        });
      }
    } catch {
      // Logout request failed — still clear local state
    }

    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(TOKEN_EXPIRY_KEY);

    setState((prev) => ({
      ...prev,
      user: null,
      token: null,
      isAuthenticated: false,
      error: null,
    }));
    navigate('/login', { replace: true });
  }, [navigate, clearRefreshTimer]);

  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
  }, []);

  const value: AuthContextValue = {
    ...state,
    loginWithApiKey,
    loginWithOidc,
    logout: logoutFn,
    clearError,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
