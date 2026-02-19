import React, { useState } from 'react';
import { useAuth } from '../../../hooks/useAuth';
import { Button } from '../../ui/atoms/button';
import { Input } from '../../ui/atoms/input';
import { Separator } from '../../ui/atoms/separator';

function formatIssuerName(issuer: string): string {
  const known: Record<string, string> = {
    'accounts.google.com': 'Google',
    'github.com': 'GitHub',
    'login.microsoftonline.com': 'Azure AD',
    'dev-': 'Okta',
    'okta.com': 'Okta',
    'auth0.com': 'Auth0',
    'cognito-idp': 'AWS Cognito',
  };
  for (const [pattern, name] of Object.entries(known)) {
    if (issuer.includes(pattern)) return name;
  }
  try {
    return new URL(issuer).hostname;
  } catch {
    return issuer;
  }
}

function ModeBadge({ mode }: { mode: string }) {
  const config: Record<string, { label: string; color: string }> = {
    local: { label: 'Local', color: 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30' },
    team: { label: 'Team', color: 'bg-blue-500/15 text-blue-400 border-blue-500/30' },
    enterprise: {
      label: 'Enterprise',
      color: 'bg-purple-500/15 text-purple-400 border-purple-500/30',
    },
  };
  const { label, color } = config[mode] ?? config.local;
  return (
    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${color}`}>
      {label}
    </span>
  );
}

function AuthStatusCard() {
  const { user, isAuthenticated, securityMode } = useAuth();

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-card-foreground">Status</h3>
        <ModeBadge mode={securityMode} />
      </div>
      <div className="rounded-lg border border-border/50 bg-muted/30 p-4 space-y-2">
        <div className="flex items-center gap-2">
          <div
            className={`h-2 w-2 rounded-full ${isAuthenticated ? 'bg-emerald-400' : 'bg-yellow-400'}`}
          />
          <span className="text-sm text-card-foreground">
            {isAuthenticated ? 'Authenticated' : 'Not authenticated'}
          </span>
        </div>
        {user && (
          <>
            <div className="text-xs text-muted-foreground">
              <span className="font-medium">Identity:</span> {user.name || user.id}
            </div>
            <div className="text-xs text-muted-foreground">
              <span className="font-medium">Method:</span> {user.auth_method}
            </div>
            {user.tenant && (
              <div className="text-xs text-muted-foreground">
                <span className="font-medium">Tenant:</span> {user.tenant}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

function ApiKeyCard() {
  const { loginWithApiKey, isAuthenticated, user, error } = useAuth();
  const [apiKey, setApiKey] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const isApiKeyAuth = user?.auth_method === 'api_key';

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) return;
    setSubmitting(true);
    try {
      await loginWithApiKey(apiKey);
      setApiKey('');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-card-foreground">API Key</h3>
      <p className="text-xs text-muted-foreground">
        Authenticate with a shared secret key. Used in all deployment modes.
      </p>
      {isAuthenticated && isApiKeyAuth ? (
        <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3">
          <div className="h-2 w-2 rounded-full bg-emerald-400" />
          <span className="text-xs text-emerald-400">Connected via API key</span>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-2">
          <Input
            type="password"
            placeholder="Enter API key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            className="text-sm"
          />
          {error && error.includes('Login failed') && (
            <p className="text-xs text-red-400">{error}</p>
          )}
          <Button type="submit" variant="outline" size="sm" disabled={!apiKey.trim() || submitting}>
            {submitting ? 'Authenticating...' : 'Authenticate'}
          </Button>
        </form>
      )}
    </div>
  );
}

function OidcCard() {
  const { oidcProviders, loginWithOidc, isAuthenticated, user, securityMode } = useAuth();

  if (securityMode === 'local') return null;

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-card-foreground">Single Sign-On (OIDC)</h3>
      <p className="text-xs text-muted-foreground">
        Sign in with your organization's identity provider.
        {securityMode === 'enterprise' && ' Managed by your enterprise admin.'}
      </p>
      {oidcProviders.length === 0 ? (
        <div className="rounded-lg border border-border/50 bg-muted/30 p-3">
          <p className="text-xs text-muted-foreground">
            No OIDC providers configured.
            {securityMode === 'team'
              ? ' Add a provider via the server configuration.'
              : ' Contact your enterprise administrator.'}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {isAuthenticated && user?.auth_method === 'oidc' && (
            <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3">
              <div className="h-2 w-2 rounded-full bg-emerald-400" />
              <span className="text-xs text-emerald-400">
                Signed in via {user.name || 'SSO'}
              </span>
            </div>
          )}
          {oidcProviders.map((provider) => (
            <Button
              key={provider.issuer}
              variant="outline"
              size="sm"
              className="w-full justify-start"
              onClick={() => loginWithOidc(provider.issuer)}
              disabled={isAuthenticated && user?.auth_method === 'oidc'}
            >
              Sign in with {formatIssuerName(provider.issuer)}
            </Button>
          ))}
        </div>
      )}
    </div>
  );
}

function EnterpriseCard() {
  const { securityMode, user } = useAuth();

  if (securityMode !== 'enterprise') return null;

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-card-foreground">Enterprise</h3>
      <p className="text-xs text-muted-foreground">
        Enterprise-managed settings. These values are set by your administrator.
      </p>
      <div className="rounded-lg border border-border/50 bg-muted/30 p-4 space-y-2">
        {user?.tenant && (
          <div className="text-xs text-muted-foreground">
            <span className="font-medium">Tenant:</span> {user.tenant}
          </div>
        )}
        <div className="text-xs text-muted-foreground">
          <span className="font-medium">Policies:</span> Managed by control plane
        </div>
        <div className="text-xs text-muted-foreground">
          <span className="font-medium">Quotas:</span> Managed by control plane
        </div>
      </div>
    </div>
  );
}

function SignOutCard() {
  const { isAuthenticated, logout } = useAuth();

  if (!isAuthenticated) return null;

  return (
    <div className="pt-2">
      <Button variant="outline" size="sm" onClick={logout} className="text-red-400 hover:text-red-300">
        Sign out
      </Button>
    </div>
  );
}

export default function AuthSection() {
  return (
    <div className="space-y-6">
      <AuthStatusCard />
      <Separator />
      <ApiKeyCard />
      <OidcCard />
      <EnterpriseCard />
      <Separator />
      <SignOutCard />
    </div>
  );
}
