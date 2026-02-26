import { useState } from 'react';
import { useAuth } from '@/hooks/useAuth';

export default function LoginView() {
  const { loginWithApiKey, loginWithOidc, oidcProviders, isLoading } = useAuth();
  const [apiKey, setApiKey] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loggingIn, setLoggingIn] = useState(false);

  const handleApiKeyLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) return;

    setLoggingIn(true);
    setError(null);
    try {
      await loginWithApiKey(apiKey.trim());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoggingIn(false);
    }
  };

  const handleOidcLogin = async (issuer: string) => {
    setLoggingIn(true);
    setError(null);
    try {
      await loginWithOidc(issuer);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'OIDC login failed');
    } finally {
      setLoggingIn(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background-default">
        <div className="animate-pulse text-text-muted">Checking authentication…</div>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-screen bg-background-default">
      <div className="w-full max-w-sm space-y-6 p-8">
        {/* Logo */}
        <div className="text-center space-y-2">
          <div className="text-4xl">🪿</div>
          <h1 className="text-2xl font-semibold text-text-default">Sign in to Goose</h1>
          <p className="text-sm text-text-muted">Choose a sign-in method to continue</p>
        </div>

        {/* Error */}
        {error && (
          <div className="rounded-md border border-border-danger bg-background-danger/10 p-3">
            <p className="text-sm text-text-danger">{error}</p>
          </div>
        )}

        {/* OIDC providers */}
        {oidcProviders.length > 0 && (
          <div className="space-y-2">
            {oidcProviders.map((provider) => (
              <button type="button"
                key={provider.issuer}
                onClick={() => handleOidcLogin(provider.issuer)}
                disabled={loggingIn}
                className="w-full flex items-center justify-center gap-2 rounded-lg border border-border-default bg-background-default px-4 py-3 text-sm font-medium text-text-default transition-colors hover:bg-background-active hover:border-border-strong disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <OidcProviderIcon issuer={provider.issuer} />
                Continue with {formatIssuerName(provider.issuer)}
              </button>
            ))}
          </div>
        )}

        {/* Divider */}
        {oidcProviders.length > 0 && (
          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <div className="w-full border-t border-border-default" />
            </div>
            <div className="relative flex justify-center text-xs">
              <span className="bg-background-default px-2 text-text-muted">or</span>
            </div>
          </div>
        )}

        {/* API key form */}
        <form onSubmit={handleApiKeyLogin} className="space-y-3">
          <div>
            <label htmlFor="api-key" className="block text-sm font-medium text-text-default mb-1">
              API Key
            </label>
            <input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Enter your API key"
              disabled={loggingIn}
              className="w-full rounded-lg border border-border-default bg-background-default px-3 py-2 text-sm text-text-default placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-border-accent focus:border-transparent disabled:opacity-50"
            />
          </div>
          <button
            type="submit"
            disabled={loggingIn || !apiKey.trim()}
            className="w-full rounded-lg bg-background-accent px-4 py-2.5 text-sm font-medium text-text-on-accent transition-colors hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loggingIn ? 'Signing in…' : 'Sign in with API Key'}
          </button>
        </form>
      </div>
    </div>
  );
}

function formatIssuerName(issuer: string): string {
  try {
    const url = new URL(issuer);
    const host = url.hostname;
    if (host.includes('google')) return 'Google';
    if (host.includes('github')) return 'GitHub';
    if (host.includes('microsoft') || host.includes('azure')) return 'Microsoft';
    if (host.includes('okta')) return 'Okta';
    if (host.includes('auth0')) return 'Auth0';
    return host;
  } catch {
    return issuer;
  }
}

function OidcProviderIcon({ issuer }: { issuer: string }) {
  const name = formatIssuerName(issuer).toLowerCase();

  // Simple SVG icons for common providers
  if (name === 'google') {
    return (
      <svg className="w-5 h-5" viewBox="0 0 24 24">
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

  if (name === 'github') {
    return (
      <svg className="w-5 h-5 text-text-default" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
      </svg>
    );
  }

  if (name === 'microsoft') {
    return (
      <svg className="w-5 h-5" viewBox="0 0 24 24">
        <path d="M1 1h10.5v10.5H1z" fill="#F25022" />
        <path d="M12.5 1H23v10.5H12.5z" fill="#7FBA00" />
        <path d="M1 12.5h10.5V23H1z" fill="#00A4EF" />
        <path d="M12.5 12.5H23V23H12.5z" fill="#FFB900" />
      </svg>
    );
  }

  // Fallback lock icon
  return (
    <svg
      className="w-5 h-5 text-text-muted"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
    >
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0110 0v4" />
    </svg>
  );
}
