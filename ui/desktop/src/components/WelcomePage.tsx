import React, { useState, useCallback, useEffect } from 'react';
import { useAuth } from '../hooks/useAuth';
import { useConfig } from './ConfigContext';
import WelcomeGooseLogo from './WelcomeGooseLogo';
import { ProviderCard } from './settings/providers/subcomponents/ProviderCard';
import ProviderConfigurationModal from './settings/providers/modal/ProviderConfiguationModal';
import type { ProviderDetails } from '../api';
import { Shield, Key, ChevronRight } from 'lucide-react';

/* ─── Auth provider icon ─────────────────────────────────────── */

function formatIssuerName(issuer: string): string {
  if (issuer.includes('google')) return 'Google';
  if (issuer.includes('github')) return 'GitHub';
  if (issuer.includes('microsoft') || issuer.includes('azure'))
    return 'Microsoft';
  if (issuer.includes('okta')) return 'Okta';
  if (issuer.includes('auth0')) return 'Auth0';
  if (issuer.includes('gitlab')) return 'GitLab';
  if (issuer.includes('amazon') || issuer.includes('cognito')) return 'AWS';
  return new URL(issuer).hostname;
}

function AuthProviderIcon({ issuer }: { issuer: string }) {
  const name = formatIssuerName(issuer).toLowerCase();
  const size = 20;

  if (name === 'google')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <path
          fill="#4285F4"
          d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
        />
        <path
          fill="#34A853"
          d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
        />
        <path
          fill="#FBBC05"
          d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
        />
        <path
          fill="#EA4335"
          d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
        />
      </svg>
    );

  if (name === 'github')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
      </svg>
    );

  if (name === 'microsoft')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <rect fill="#F25022" x="1" y="1" width="10" height="10" />
        <rect fill="#7FBA00" x="13" y="1" width="10" height="10" />
        <rect fill="#00A4EF" x="1" y="13" width="10" height="10" />
        <rect fill="#FFB900" x="13" y="13" width="10" height="10" />
      </svg>
    );

  if (name === 'okta')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24">
        <circle fill="#007DC1" cx="12" cy="12" r="11" />
        <circle fill="white" cx="12" cy="12" r="5" />
      </svg>
    );

  if (name === 'auth0')
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" fill="#EB5424">
        <path d="M17.77 22.34L14.75 14l7.94-5.79h-9.84L9.84 0h9.84l3.01 8.21a10.82 10.82 0 01-4.92 14.13zM6.23 22.34l-3.02-8.21A10.82 10.82 0 018.13.01L5.12 8.22l7.94 5.79-6.83 8.33z" />
      </svg>
    );

  // Fallback
  return <Shield size={size} />;
}

/* ─── Welcome Page ───────────────────────────────────────────── */

interface WelcomePageProps {
  onComplete: () => void;
}

export default function WelcomePage({ onComplete }: WelcomePageProps) {
  const { authRequired, isAuthenticated, oidcProviders, loginWithOidc, loginWithApiKey } =
    useAuth();
  const { getProviders } = useConfig();

  // Auth state
  const [apiKey, setApiKey] = useState('');
  const [authError, setAuthError] = useState('');
  const [authLoading, setAuthLoading] = useState(false);

  // Provider state
  const [providers, setProviders] = useState<ProviderDetails[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<ProviderDetails | null>(null);
  const [providerConfigured, setProviderConfigured] = useState(false);
  const [loadingProviders, setLoadingProviders] = useState(true);

  // Which panel to show: 'auth' or 'providers'
  const needsAuth = authRequired && !isAuthenticated;
  const [activePanel, setActivePanel] = useState<'auth' | 'providers'>(
    needsAuth ? 'auth' : 'providers'
  );

  // Load providers
  const loadProviders = useCallback(async () => {
    try {
      setLoadingProviders(true);
      const list = await getProviders(false);
      setProviders(list);
      if (list.some((p) => p.is_configured)) {
        setProviderConfigured(true);
      }
    } catch {
      // ignore
    } finally {
      setLoadingProviders(false);
    }
  }, [getProviders]);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  // Auto-switch to providers panel when auth completes
  useEffect(() => {
    if (!needsAuth && activePanel === 'auth') {
      setActivePanel('providers');
    }
  }, [needsAuth, activePanel]);

  /* ── Auth handlers ─────────────────────────────────── */

  const handleOidcLogin = async (issuer: string) => {
    try {
      setAuthError('');
      setAuthLoading(true);
      await loginWithOidc(issuer);
      setActivePanel('providers');
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setAuthLoading(false);
    }
  };

  const handleApiKeyLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) return;
    try {
      setAuthError('');
      setAuthLoading(true);
      await loginWithApiKey(apiKey.trim());
      setActivePanel('providers');
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setAuthLoading(false);
    }
  };

  /* ── Provider handlers ─────────────────────────────── */

  const handleProviderConfigure = (provider: ProviderDetails) => {
    setSelectedProvider(provider);
  };

  const handleProviderConfigured = (_provider?: ProviderDetails) => {
    setSelectedProvider(null);
    setProviderConfigured(true);
    loadProviders();
  };

  const handleGetStarted = () => {
    onComplete();
  };

  /* ── Left Sidebar ──────────────────────────────────── */

  const renderSidebar = () => (
    <div className="flex flex-col items-center justify-center h-full px-8 text-center">
      {/* Logo */}
      <div className="group/logo w-32 h-32 mb-8">
        <WelcomeGooseLogo className="w-full h-full" />
      </div>

      {/* Title */}
      <h1 className="text-2xl font-bold text-white mb-3">Welcome to Goose</h1>

      {/* Subtitle based on panel */}
      {activePanel === 'auth' ? (
        <p className="text-sm text-gray-400 max-w-[240px]">
          Sign in to get started with your AI-powered development assistant
        </p>
      ) : (
        <p className="text-sm text-gray-400 max-w-[240px]">
          Choose your model provider to power your AI assistant
        </p>
      )}

      {/* Panel switcher (only when auth is available) */}
      {authRequired && (
        <div className="mt-8 flex flex-col gap-2 w-full max-w-[200px]">
          <button
            onClick={() => setActivePanel('auth')}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-colors
              ${activePanel === 'auth' ? 'bg-white/10 text-white' : 'text-gray-500 hover:text-gray-300'}`}
          >
            <Shield size={16} />
            Authentication
            {isAuthenticated && <span className="ml-auto text-green-400">✓</span>}
          </button>
          <button
            onClick={() => setActivePanel('providers')}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-colors
              ${activePanel === 'providers' ? 'bg-white/10 text-white' : 'text-gray-500 hover:text-gray-300'}`}
          >
            <Key size={16} />
            Model Provider
            {providerConfigured && <span className="ml-auto text-green-400">✓</span>}
          </button>
        </div>
      )}
    </div>
  );

  /* ── Auth Panel ────────────────────────────────────── */

  const renderAuthPanel = () => (
    <div className="flex flex-col h-full">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-foreground">Sign In</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Choose your authentication method
        </p>
      </div>

      {authError && (
        <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
          {authError}
        </div>
      )}

      {/* OIDC Providers Grid */}
      {oidcProviders.length > 0 && (
        <div className="mb-6">
          <div
            className="grid gap-3"
            style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))' }}
          >
            {oidcProviders.map((provider) => (
              <button
                key={provider.issuer}
                onClick={() => handleOidcLogin(provider.issuer)}
                disabled={authLoading}
                className="flex items-center gap-3 px-4 py-3 rounded-lg
                  bg-background-muted border border-border-default
                  hover:border-border-active hover:bg-background-muted/80
                  transition-all duration-200 text-left
                  disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <AuthProviderIcon issuer={provider.issuer} />
                <div className="flex flex-col">
                  <span className="text-sm font-medium text-foreground">
                    {formatIssuerName(provider.issuer)}
                  </span>
                  <span className="text-xs text-muted-foreground">SSO</span>
                </div>
                <ChevronRight size={14} className="ml-auto text-muted-foreground" />
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Divider */}
      {oidcProviders.length > 0 && (
        <div className="flex items-center gap-3 mb-6">
          <div className="flex-1 h-px bg-border-default" />
          <span className="text-xs text-muted-foreground">or use an API key</span>
          <div className="flex-1 h-px bg-border-default" />
        </div>
      )}

      {/* API Key Form */}
      <form onSubmit={handleApiKeyLogin} className="flex gap-2">
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="Enter your API key"
          className="flex-1 px-4 py-2.5 rounded-lg text-sm
            bg-background-muted border border-border-default
            text-foreground placeholder:text-muted-foreground
            focus:outline-none focus:border-border-active
            transition-colors"
        />
        <button
          type="submit"
          disabled={!apiKey.trim() || authLoading}
          className="px-6 py-2.5 rounded-lg text-sm font-medium
            bg-accent-primary text-white
            hover:bg-accent-primary/90
            disabled:opacity-50 disabled:cursor-not-allowed
            transition-colors"
        >
          {authLoading ? 'Signing in...' : 'Sign In'}
        </button>
      </form>

      {/* Skip auth */}
      <div className="mt-auto pt-6">
        <button
          onClick={() => setActivePanel('providers')}
          className="text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          Skip authentication →
        </button>
      </div>
    </div>
  );

  /* ── Provider Panel ────────────────────────────────── */

  const renderProviderPanel = () => (
    <div className="flex flex-col h-full">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-foreground">Choose a Model Provider</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Select and configure an AI model provider to get started
        </p>
      </div>

      {loadingProviders ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-accent-primary" />
        </div>
      ) : (
        <>
          {/* Provider Grid */}
          <div
            className="grid gap-3 flex-1 auto-rows-min"
            style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))' }}
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

          {/* Get Started button */}
          <div className="mt-6 pt-4 border-t border-border-default flex items-center justify-between">
            {needsAuth && (
              <button
                onClick={() => setActivePanel('auth')}
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                ← Back to sign in
              </button>
            )}
            <div className="ml-auto">
              <button
                onClick={handleGetStarted}
                disabled={!providerConfigured}
                className="px-6 py-2.5 rounded-lg text-sm font-medium
                  bg-accent-primary text-white
                  hover:bg-accent-primary/90
                  disabled:opacity-50 disabled:cursor-not-allowed
                  transition-colors"
              >
                Get Started
              </button>
            </div>
          </div>
        </>
      )}

      {/* Provider configuration modal */}
      {selectedProvider && (
        <ProviderConfigurationModal
          provider={selectedProvider}
          onClose={() => setSelectedProvider(null)}
          onConfigured={handleProviderConfigured}
        />
      )}
    </div>
  );

  /* ── Layout ────────────────────────────────────────── */

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      {/* Left sidebar — dark panel with logo */}
      <div className="w-[320px] min-w-[320px] bg-[#1a1a2e] flex-shrink-0">
        {renderSidebar()}
      </div>

      {/* Right content — provider/auth grid */}
      <div className="flex-1 bg-background-default overflow-y-auto p-8">
        {activePanel === 'auth' ? renderAuthPanel() : renderProviderPanel()}
      </div>
    </div>
  );
}
