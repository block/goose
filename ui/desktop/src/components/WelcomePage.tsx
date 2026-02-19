import React, { useState, useCallback, useMemo, memo } from 'react';
import { useAuth } from '../hooks/useAuth';
import { useConfig } from './ConfigContext';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { ProviderCard } from './settings/providers/subcomponents/ProviderCard';
import ProviderConfigurationModal from './settings/providers/modal/ProviderConfiguationModal';
import type { ProviderDetails } from '../api';
import { Check, ChevronRight, Key, Loader2, Shield } from 'lucide-react';

// ─── Auth Provider Card ──────────────────────────────────────────────

interface AuthProviderCardProps {
  name: string;
  icon: React.ReactNode;
  description: string;
  onClick: () => void;
  disabled?: boolean;
  authenticated?: boolean;
}

const AuthProviderCard = memo(function AuthProviderCard({
  name,
  icon,
  description,
  onClick,
  disabled = false,
  authenticated = false,
}: AuthProviderCardProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`relative group/card flex flex-col items-center gap-2 rounded-lg border p-4
        transition-all duration-200 h-[120px] w-full justify-center
        ${
          authenticated
            ? 'border-green-500/50 bg-green-500/5'
            : 'border-border-default bg-background-default hover:border-border-strong hover:bg-background-active'
        }
        ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
      `}
    >
      {authenticated && (
        <div className="absolute top-2 right-2">
          <Check className="w-4 h-4 text-green-500" />
        </div>
      )}
      <div className="w-8 h-8 flex items-center justify-center">{icon}</div>
      <span className="text-sm font-medium text-text-default">{name}</span>
      <span className="text-xs text-text-muted text-center line-clamp-1">{description}</span>
    </button>
  );
});

// ─── OIDC Provider Icons (from LoginView) ────────────────────────────

function GoogleIcon() {
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24">
      <path
        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
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
}

function GitHubIcon() {
  return (
    <svg className="w-6 h-6 text-text-default" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
    </svg>
  );
}

function MicrosoftIcon() {
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24">
      <path d="M1 1h10.5v10.5H1z" fill="#F25022" />
      <path d="M12.5 1H23v10.5H12.5z" fill="#7FBA00" />
      <path d="M1 12.5h10.5V23H1z" fill="#00A4EF" />
      <path d="M12.5 12.5H23V23H12.5z" fill="#FFB900" />
    </svg>
  );
}

function OktaIcon() {
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24">
      <circle cx="12" cy="12" r="10" fill="#007DC1" />
      <circle cx="12" cy="12" r="4" fill="white" />
    </svg>
  );
}

function Auth0Icon() {
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="#EB5424">
      <path d="M17.64 2H6.36L2 12l4.36 10h11.28L22 12 17.64 2zM12 16a4 4 0 110-8 4 4 0 010 8z" />
    </svg>
  );
}

function KeyIcon() {
  return <Key className="w-6 h-6 text-text-muted" />;
}

function issuerToIcon(issuer: string): React.ReactNode {
  try {
    const host = new URL(issuer).hostname;
    if (host.includes('google')) return <GoogleIcon />;
    if (host.includes('github')) return <GitHubIcon />;
    if (host.includes('microsoft') || host.includes('azure')) return <MicrosoftIcon />;
    if (host.includes('okta')) return <OktaIcon />;
    if (host.includes('auth0')) return <Auth0Icon />;
  } catch {
    /* fallback */
  }
  return (
    <Shield className="w-6 h-6 text-text-muted" />
  );
}

function issuerToName(issuer: string): string {
  try {
    const host = new URL(issuer).hostname;
    if (host.includes('google')) return 'Google';
    if (host.includes('github')) return 'GitHub';
    if (host.includes('microsoft') || host.includes('azure')) return 'Microsoft';
    if (host.includes('okta')) return 'Okta';
    if (host.includes('auth0')) return 'Auth0';
    if (host.includes('gitlab')) return 'GitLab';
    if (host.includes('amazon') || host.includes('cognito')) return 'AWS';
    return host;
  } catch {
    return issuer;
  }
}

// ─── Section Header ──────────────────────────────────────────────────

function SectionHeader({
  step,
  title,
  subtitle,
  completed = false,
}: {
  step: number;
  title: string;
  subtitle: string;
  completed?: boolean;
}) {
  return (
    <div className="flex items-center gap-3 mb-4">
      <div
        className={`flex items-center justify-center w-8 h-8 rounded-full text-sm font-semibold
          ${completed ? 'bg-green-500/20 text-green-500' : 'bg-background-active text-text-default'}
        `}
      >
        {completed ? <Check className="w-4 h-4" /> : step}
      </div>
      <div>
        <h2 className="text-lg font-semibold text-text-default">{title}</h2>
        <p className="text-sm text-text-muted">{subtitle}</p>
      </div>
    </div>
  );
}

// ─── Grid Layout (matches ProviderGrid) ──────────────────────────────

const AuthGrid = memo(function AuthGrid({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="grid gap-3 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(160px, 1fr))',
      }}
    >
      {children}
    </div>
  );
});

const ProviderGridLayout = memo(function ProviderGridLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div
      className="grid gap-4 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 200px))',
        justifyContent: 'center',
      }}
    >
      {children}
    </div>
  );
});

// ─── API Key Form (inline) ──────────────────────────────────────────

function ApiKeyInlineForm({
  onSubmit,
  disabled,
}: {
  onSubmit: (key: string) => void;
  disabled: boolean;
}) {
  const [apiKey, setApiKey] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (apiKey.trim()) onSubmit(apiKey.trim());
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2 mt-3">
      <input
        type="password"
        placeholder="Enter API key…"
        value={apiKey}
        onChange={(e) => setApiKey(e.target.value)}
        disabled={disabled}
        className="flex-1 rounded-md border border-border-default bg-background-default px-3 py-2
          text-sm text-text-default placeholder:text-text-muted
          focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500
          disabled:opacity-50"
      />
      <button
        type="submit"
        disabled={disabled || !apiKey.trim()}
        className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white
          hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed
          transition-colors"
      >
        Connect
      </button>
    </form>
  );
}

// ─── Main Welcome Page ──────────────────────────────────────────────

export interface WelcomePageProps {
  onComplete: () => void;
}

export default function WelcomePage({ onComplete }: WelcomePageProps) {
  const { loginWithApiKey, loginWithOidc, oidcProviders, isAuthenticated, authRequired, user } =
    useAuth();
  const { getProviders } = useConfig();

  const [authError, setAuthError] = useState<string | null>(null);
  const [authLoading, setAuthLoading] = useState(false);
  const [providers, setProviders] = useState<ProviderDetails[]>([]);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [configureProvider, setConfigureProvider] = useState<ProviderDetails | null>(null);
  const [providerConfigured, setProviderConfigured] = useState(false);

  const loadProviders = useCallback(async () => {
    setProvidersLoading(true);
    try {
      const list = await getProviders(false);
      setProviders(list);
      // Check if any is already configured
      if (list.some((p) => p.is_configured)) {
        setProviderConfigured(true);
      }
    } catch {
      /* ignore */
    } finally {
      setProvidersLoading(false);
    }
  }, [getProviders]);

  // Auth step completed?
  const authCompleted = !authRequired || isAuthenticated;

  // Provider step completed?
  const stepCompleted = authCompleted && providerConfigured;

  // ─── Auth handlers ─────────────────────────

  const handleOidcLogin = useCallback(
    async (issuer: string) => {
      setAuthLoading(true);
      setAuthError(null);
      try {
        await loginWithOidc(issuer);
      } catch (err) {
        setAuthError(err instanceof Error ? err.message : 'Login failed');
      } finally {
        setAuthLoading(false);
      }
    },
    [loginWithOidc]
  );

  const handleApiKeyLogin = useCallback(
    async (key: string) => {
      setAuthLoading(true);
      setAuthError(null);
      try {
        await loginWithApiKey(key);
      } catch (err) {
        setAuthError(err instanceof Error ? err.message : 'Login failed');
      } finally {
        setAuthLoading(false);
      }
    },
    [loginWithApiKey]
  );

  // ─── Provider handlers ─────────────────────

  const handleProviderConfigure = useCallback((provider: ProviderDetails) => {
    setConfigureProvider(provider);
  }, []);

  const handleProviderConfigured = useCallback((_provider?: ProviderDetails) => {
    setConfigureProvider(null);
    setProviderConfigured(true);
    loadProviders();
  }, [loadProviders]);

  // Sort providers: configured first, then alphabetical
  const sortedProviders = useMemo(() => {
    return [...providers].sort((a, b) => {
      if (a.is_configured && !b.is_configured) return -1;
      if (!a.is_configured && b.is_configured) return 1;
      return a.name.localeCompare(b.name);
    });
  }, [providers]);

  return (
    <div className="min-h-screen bg-background-default flex flex-col">
      {/* Header */}
      <header className="flex flex-col items-center pt-8 pb-4">
        <div className="group/logo w-20 h-20 mb-4">
          <WelcomeGooseLogo className="w-full h-full" />
        </div>
        <h1 className="text-2xl font-bold text-text-default">Welcome to Goose</h1>
        <p className="text-sm text-text-muted mt-1">
          Get started by configuring your authentication and AI provider
        </p>
      </header>

      {/* Content */}
      <main className="flex-1 overflow-y-auto px-6 pb-24 max-w-5xl mx-auto w-full">
        {/* ─── Step 1: Authentication ─────────────── */}
        <section className="mb-8">
          <SectionHeader
            step={1}
            title="Authentication"
            subtitle={
              authRequired
                ? 'Sign in with your identity provider'
                : 'Optional — sign in for personalized experience'
            }
            completed={authCompleted}
          />

          {authError && (
            <div className="rounded-md border border-red-500/30 bg-red-500/5 p-3 mb-4">
              <p className="text-sm text-red-400">{authError}</p>
            </div>
          )}

          {isAuthenticated ? (
            <div className="rounded-lg border border-green-500/30 bg-green-500/5 p-4 flex items-center gap-3">
              <Check className="w-5 h-5 text-green-500 shrink-0" />
              <div>
                <p className="text-sm font-medium text-text-default">
                  Signed in{user?.name ? ` as ${user.name}` : ''}
                </p>
                <p className="text-xs text-text-muted">Authentication configured successfully</p>
              </div>
            </div>
          ) : (
            <>
              <AuthGrid>
                {/* OIDC Providers */}
                {oidcProviders.map((provider) => (
                  <AuthProviderCard
                    key={provider.issuer}
                    name={issuerToName(provider.issuer)}
                    icon={issuerToIcon(provider.issuer)}
                    description="Single Sign-On"
                    onClick={() => handleOidcLogin(provider.issuer)}
                    disabled={authLoading}
                  />
                ))}

                {/* API Key card */}
                <AuthProviderCard
                  name="API Key"
                  icon={<KeyIcon />}
                  description="Authenticate with a key"
                  onClick={() => {
                    /* API key form is shown below */
                  }}
                  disabled={authLoading}
                />
              </AuthGrid>

              {/* API Key inline form */}
              <ApiKeyInlineForm onSubmit={handleApiKeyLogin} disabled={authLoading} />

              {authLoading && (
                <div className="flex items-center gap-2 mt-3 text-sm text-text-muted">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Authenticating…
                </div>
              )}

              {!authRequired && (
                <button
                  onClick={onComplete}
                  className="mt-3 text-sm text-text-muted hover:text-text-default transition-colors underline"
                >
                  Skip for now
                </button>
              )}
            </>
          )}
        </section>

        {/* ─── Step 2: AI Provider ──────────────── */}
        <section className="mb-8">
          <SectionHeader
            step={2}
            title="AI Provider"
            subtitle="Choose your model provider to power Goose"
            completed={providerConfigured}
          />

          {providersLoading ? (
            <div className="flex items-center gap-2 text-sm text-text-muted py-8 justify-center">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading providers…
            </div>
          ) : (
            <ProviderGridLayout>
              {sortedProviders.map((provider) => (
                <ProviderCard
                  key={provider.name}
                  provider={provider}
                  onConfigure={() => handleProviderConfigure(provider)}
                  onLaunch={() => {
                    handleProviderConfigure(provider);
                  }}
                  isOnboarding={true}
                />
              ))}
            </ProviderGridLayout>
          )}
        </section>
      </main>

      {/* Bottom bar */}
      <footer className="fixed bottom-0 inset-x-0 bg-background-default/80 backdrop-blur border-t border-border-default">
        <div className="max-w-5xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="text-sm text-text-muted">
            {authCompleted && providerConfigured
              ? '✓ All set!'
              : authCompleted && !providerConfigured
                ? 'Step 2: Configure an AI provider'
                : 'Step 1: Choose authentication method'}
          </div>
          <button
            onClick={onComplete}
            disabled={!stepCompleted && authRequired}
            className={`flex items-center gap-2 rounded-lg px-6 py-2.5 text-sm font-medium transition-all
              ${
                stepCompleted
                  ? 'bg-blue-600 text-white hover:bg-blue-700 shadow-lg shadow-blue-600/20'
                  : authRequired
                    ? 'bg-background-active text-text-muted cursor-not-allowed'
                    : 'bg-background-active text-text-default hover:bg-background-muted'
              }
            `}
          >
            {stepCompleted ? 'Get Started' : authRequired ? 'Complete setup to continue' : 'Skip'}
            {stepCompleted && <ChevronRight className="w-4 h-4" />}
          </button>
        </div>
      </footer>

      {/* Provider configuration modal */}
      {configureProvider && (
        <ProviderConfigurationModal
          provider={configureProvider}
          onClose={() => setConfigureProvider(null)}
          onConfigured={handleProviderConfigured}
        />
      )}
    </div>
  );
}
