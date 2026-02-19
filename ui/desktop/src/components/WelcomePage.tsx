import React, { useState, useCallback, useEffect } from 'react';
import { useAuth } from '../hooks/useAuth';
import { useConfig } from './ConfigContext';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { ProviderCard } from './settings/providers/subcomponents/ProviderCard';
import ProviderConfigurationModal from './settings/providers/modal/ProviderConfiguationModal';
import type { ProviderDetails } from '../api';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Separator } from './ui/separator';
import { Shield, Key, ChevronRight, Check, LogIn, Loader2 } from 'lucide-react';

/* ─── Auth provider helpers ────────────────────────────────────── */

function formatIssuerName(issuer: string): string {
  if (issuer.includes('google')) return 'Google';
  if (issuer.includes('github')) return 'GitHub';
  if (issuer.includes('microsoft') || issuer.includes('azure'))
    return 'Microsoft';
  if (issuer.includes('okta')) return 'Okta';
  if (issuer.includes('auth0')) return 'Auth0';
  if (issuer.includes('gitlab')) return 'GitLab';
  if (issuer.includes('amazon') || issuer.includes('cognito')) return 'AWS';
  try {
    return new URL(issuer).hostname;
  } catch {
    return issuer;
  }
}

function AuthProviderIcon({ issuer }: { issuer: string }) {
  const name = formatIssuerName(issuer).toLowerCase();
  const size = 20;

  if (name === 'google')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <path
          d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
          fill="#4285F4"
        />
        <path
          d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
          fill="#34A853"
        />
        <path
          d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
          fill="#FBBC05"
        />
        <path
          d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
          fill="#EA4335"
        />
      </svg>
    );
  if (name === 'github')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
      </svg>
    );
  if (name === 'microsoft')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <rect x="1" y="1" width="10" height="10" fill="#F25022" />
        <rect x="13" y="1" width="10" height="10" fill="#7FBA00" />
        <rect x="1" y="13" width="10" height="10" fill="#00A4EF" />
        <rect x="13" y="13" width="10" height="10" fill="#FFB900" />
      </svg>
    );
  if (name === 'okta')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" fill="none" stroke="#007DC1" strokeWidth="3" />
        <circle cx="12" cy="12" r="4" fill="#007DC1" />
      </svg>
    );
  if (name === 'auth0')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <path
          d="M17.2 21.5L12 17.2l-5.2 4.3 2-6.1L3.6 11h6.5L12 4.5l1.9 6.5h6.5l-5.2 4.4 2 6.1z"
          fill="#EB5424"
        />
      </svg>
    );
  return <LogIn size={size} className="text-muted-foreground" />;
}

/* ─── Main WelcomePage ─────────────────────────────────────────── */

interface WelcomePageProps {
  onComplete?: () => void;
}

export default function WelcomePage({ onComplete }: WelcomePageProps) {
  const { isAuthenticated, authRequired, oidcProviders, loginWithApiKey, loginWithOidc, isLoading } =
    useAuth();
  const { getProviders } = useConfig();

  const [activePanel, setActivePanel] = useState<'auth' | 'provider'>('provider');
  const [apiKey, setApiKey] = useState('');
  const [authError, setAuthError] = useState('');
  const [providers, setProviders] = useState<ProviderDetails[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<ProviderDetails | null>(null);
  const [providerConfigured, setProviderConfigured] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);

  const authComplete = !authRequired || isAuthenticated;

  const loadProviders = useCallback(async () => {
    try {
      const result = await getProviders(false);
      setProviders(result);
      if (result.some((p: ProviderDetails) => p.is_configured)) {
        setProviderConfigured(true);
      }
    } catch {
      // providers will load later
    }
  }, [getProviders]);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  useEffect(() => {
    if (!authRequired) setActivePanel('provider');
  }, [authRequired]);

  const handleOidcLogin = useCallback(
    async (issuer: string) => {
      setIsAuthenticating(true);
      setAuthError('');
      try {
        await loginWithOidc(issuer);
        setActivePanel('provider');
      } catch (err) {
        setAuthError(err instanceof Error ? err.message : 'Login failed');
      } finally {
        setIsAuthenticating(false);
      }
    },
    [loginWithOidc]
  );

  const handleApiKeyLogin = useCallback(async () => {
    if (!apiKey.trim()) return;
    setIsAuthenticating(true);
    setAuthError('');
    try {
      await loginWithApiKey(apiKey.trim());
      setActivePanel('provider');
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : 'Invalid API key');
    } finally {
      setIsAuthenticating(false);
    }
  }, [apiKey, loginWithApiKey]);

  const handleProviderConfigure = useCallback((provider: ProviderDetails) => {
    setSelectedProvider(provider);
  }, []);

  const handleProviderConfigured = useCallback(
    async (_provider?: ProviderDetails) => {
      setSelectedProvider(null);
      setProviderConfigured(true);
      await loadProviders();
    },
    [loadProviders]
  );

  const handleGetStarted = useCallback(() => {
    if (onComplete) onComplete();
  }, [onComplete]);

  /* ─── Sidebar nav items ──────────────────────────────────────── */
  const navItems = [
    ...(authRequired
      ? [
          {
            id: 'auth' as const,
            label: 'Authentication',
            icon: Shield,
            complete: authComplete,
          },
        ]
      : []),
    {
      id: 'provider' as const,
      label: 'Model Provider',
      icon: Key,
      complete: providerConfigured,
    },
  ];

  /* ─── Render ─────────────────────────────────────────────────── */
  return (
    <div className="flex h-screen w-screen bg-background">
      {/* ─── Left sidebar ───────────────────────────────────────── */}
      <aside className="flex w-80 shrink-0 flex-col bg-card border-r border-border">
        <div className="flex flex-1 flex-col items-center justify-center gap-8 p-8">
          <WelcomeGooseLogo className="h-24 w-24" />
          <div className="text-center">
            <h1 className="text-2xl font-semibold text-foreground">
              Welcome to Goose
            </h1>
            <p className="mt-2 text-sm text-muted-foreground">
              {activePanel === 'auth'
                ? 'Sign in to get started'
                : 'Choose your model provider'}
            </p>
          </div>
        </div>

        <Separator />

        <nav className="p-4 space-y-1">
          {navItems.map((item) => (
            <Button
              key={item.id}
              variant={activePanel === item.id ? 'secondary' : 'ghost'}
              className="w-full justify-start gap-3"
              onClick={() => setActivePanel(item.id)}
            >
              <item.icon className="h-4 w-4" />
              <span className="flex-1 text-left">{item.label}</span>
              {item.complete && (
                <Check className="h-4 w-4 text-green-500" />
              )}
            </Button>
          ))}
        </nav>

        <div className="p-4">
          <Button
            className="w-full"
            disabled={!providerConfigured}
            onClick={handleGetStarted}
          >
            Get Started
            <ChevronRight className="ml-2 h-4 w-4" />
          </Button>
        </div>
      </aside>

      {/* ─── Right content panel ────────────────────────────────── */}
      <main className="flex-1 overflow-y-auto p-8">
        {activePanel === 'auth' && (
          <div className="mx-auto max-w-2xl space-y-8">
            <div>
              <h2 className="text-xl font-semibold text-foreground">Sign In</h2>
              <p className="mt-1 text-sm text-muted-foreground">
                Authenticate with your organization&apos;s identity provider
              </p>
            </div>

            {/* SSO Providers */}
            {oidcProviders.length > 0 && (
              <div className="space-y-4">
                <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
                  SSO Providers
                </h3>
                <div className="grid grid-cols-2 gap-3">
                  {oidcProviders.map((provider) => (
                    <Button
                      key={provider.issuer}
                      variant="outline"
                      className="justify-start gap-3 h-12"
                      disabled={isAuthenticating || isLoading}
                      onClick={() => handleOidcLogin(provider.issuer)}
                    >
                      {isAuthenticating ? (
                        <Loader2 className="h-5 w-5 animate-spin" />
                      ) : (
                        <AuthProviderIcon issuer={provider.issuer} />
                      )}
                      <span>Continue with {formatIssuerName(provider.issuer)}</span>
                    </Button>
                  ))}
                </div>
              </div>
            )}

            {oidcProviders.length > 0 && <Separator />}

            {/* API Key */}
            <div className="space-y-4">
              <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
                API Key
              </h3>
              <div className="flex gap-3">
                <Input
                  type="password"
                  placeholder="Enter your API key"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleApiKeyLogin()}
                  className="flex-1"
                />
                <Button
                  onClick={handleApiKeyLogin}
                  disabled={!apiKey.trim() || isAuthenticating}
                >
                  {isAuthenticating ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    'Sign In'
                  )}
                </Button>
              </div>
            </div>

            {authError && (
              <p className="text-sm text-destructive">{authError}</p>
            )}

            {isAuthenticated && (
              <div className="flex items-center gap-2 rounded-md border border-green-500/20 bg-green-500/10 p-3">
                <Check className="h-4 w-4 text-green-500" />
                <span className="text-sm text-green-700 dark:text-green-400">
                  Authenticated successfully
                </span>
              </div>
            )}
          </div>
        )}

        {activePanel === 'provider' && (
          <div className="space-y-6">
            <div>
              <h2 className="text-xl font-semibold text-foreground">
                Choose a Model Provider
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                Select and configure an AI model provider to power Goose
              </p>
            </div>

            <div
              className="grid gap-4"
              style={{
                gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 200px))',
              }}
            >
              {providers
                .sort((a, b) => a.name.localeCompare(b.name))
                .map((provider) => (
                  <ProviderCard
                    key={provider.name}
                    provider={provider}
                    onConfigure={() => handleProviderConfigure(provider)}
                    onLaunch={() => handleProviderConfigure(provider)}
                    isOnboarding={true}
                  />
                ))}
            </div>

            {providers.length === 0 && (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            )}
          </div>
        )}
      </main>

      {/* ─── Provider config modal ──────────────────────────────── */}
      {selectedProvider && (
        <ProviderConfigurationModal
          provider={selectedProvider}
          onClose={() => setSelectedProvider(null)}
          onConfigured={handleProviderConfigured}
        />
      )}
    </div>
  );
}
