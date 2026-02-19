import React, { useState, useCallback } from 'react';
import { useAuth } from '../../../hooks/useAuth';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { Separator } from '../../ui/separator';
import { Switch } from '../../ui/switch';
import { LoadingState } from '../../ui/design-system/LoadingState';
import { Shield, LogOut, Key, ExternalLink, CheckCircle, AlertCircle } from 'lucide-react';

function formatIssuerName(issuer: string): string {
  if (issuer.includes('accounts.google.com')) return 'Google';
  if (issuer.includes('github.com')) return 'GitHub';
  if (issuer.includes('login.microsoftonline.com') || issuer.includes('sts.windows.net'))
    return 'Microsoft Azure';
  if (issuer.includes('okta.com')) return 'Okta';
  if (issuer.includes('auth0.com')) return 'Auth0';
  if (issuer.includes('gitlab.com')) return 'GitLab';
  if (issuer.includes('cognito')) return 'AWS Cognito';
  try {
    return new URL(issuer).hostname;
  } catch {
    return issuer;
  }
}

export default function AuthSection() {
  const {
    user,
    isAuthenticated,
    authRequired,
    oidcProviders,
    loginWithApiKey,
    loginWithOidc,
    logout,
    isLoading,
  } = useAuth();

  const [apiKey, setApiKey] = useState('');
  const [apiKeyError, setApiKeyError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleApiKeySubmit = useCallback(async () => {
    if (!apiKey.trim()) return;
    setIsSubmitting(true);
    setApiKeyError('');
    try {
      await loginWithApiKey(apiKey.trim());
      setApiKey('');
    } catch (err) {
      setApiKeyError(err instanceof Error ? err.message : 'Authentication failed');
    } finally {
      setIsSubmitting(false);
    }
  }, [apiKey, loginWithApiKey]);

  const handleOidcLogin = useCallback(
    async (issuer: string) => {
      try {
        await loginWithOidc(issuer);
      } catch (err) {
        setApiKeyError(err instanceof Error ? err.message : 'SSO login failed');
      }
    },
    [loginWithOidc]
  );

  if (isLoading) {
    return <LoadingState variant="spinner" />;
  }

  return (
    <div className="space-y-6">
      {/* Current Auth Status */}
      <div className="space-y-3">
        <h3 className="text-sm font-medium text-text-default flex items-center gap-2">
          <Shield className="h-4 w-4" />
          Authentication Status
        </h3>
        <div className="rounded-lg border border-border-default p-4 bg-background-muted/30">
          {isAuthenticated && user ? (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <CheckCircle className="h-5 w-5 text-green-500" />
                <div>
                  <p className="text-sm font-medium text-text-default">
                    {user.name || user.id}
                  </p>
                  <p className="text-xs text-text-muted">
                    {user.auth_method === 'guest'
                      ? 'Guest (no auth)'
                      : `Authenticated via ${user.auth_method}`}
                  </p>
                </div>
              </div>
              {user.auth_method !== 'guest' && (
                <Button variant="outline" size="sm" onClick={logout}>
                  <LogOut className="h-3.5 w-3.5" />
                  Sign Out
                </Button>
              )}
            </div>
          ) : (
            <div className="flex items-center gap-3">
              <AlertCircle className="h-5 w-5 text-text-muted" />
              <div>
                <p className="text-sm font-medium text-text-default">Not authenticated</p>
                <p className="text-xs text-text-muted">
                  {authRequired
                    ? 'Authentication is required to use Goose'
                    : 'Authentication is optional — running in local mode'}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      <Separator />

      {/* SSO / OIDC Providers */}
      {oidcProviders && oidcProviders.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-text-default flex items-center gap-2">
            <ExternalLink className="h-4 w-4" />
            Single Sign-On (SSO)
          </h3>
          <p className="text-xs text-text-muted">
            Sign in with your organization&apos;s identity provider
          </p>
          <div className="grid grid-cols-2 gap-2">
            {oidcProviders.map((provider) => (
              <Button
                key={provider.issuer}
                variant="outline"
                size="default"
                className="justify-start gap-2"
                onClick={() => handleOidcLogin(provider.issuer)}
                disabled={isAuthenticated && user?.auth_method !== 'guest'}
              >
                <ExternalLink className="h-4 w-4 shrink-0" />
                <span className="truncate">{formatIssuerName(provider.issuer)}</span>
              </Button>
            ))}
          </div>
        </div>
      )}

      {/* API Key Auth */}
      <div className="space-y-3">
        <h3 className="text-sm font-medium text-text-default flex items-center gap-2">
          <Key className="h-4 w-4" />
          API Key Authentication
        </h3>
        <p className="text-xs text-text-muted">
          Authenticate with a Goose API key for programmatic access
        </p>
        <div className="flex gap-2">
          <Input
            type="password"
            placeholder="Enter API key..."
            value={apiKey}
            onChange={(e) => {
              setApiKey(e.target.value);
              setApiKeyError('');
            }}
            onKeyDown={(e) => e.key === 'Enter' && handleApiKeySubmit()}
            disabled={isSubmitting}
            className="flex-1"
          />
          <Button
            variant="default"
            size="default"
            onClick={handleApiKeySubmit}
            disabled={!apiKey.trim() || isSubmitting}
          >
            {isSubmitting ? 'Verifying...' : 'Authenticate'}
          </Button>
        </div>
        {apiKeyError && (
          <p className="text-xs text-red-500 flex items-center gap-1">
            <AlertCircle className="h-3 w-3" />
            {apiKeyError}
          </p>
        )}
      </div>

      <Separator />

      {/* Auth Requirements Info */}
      <div className="space-y-3">
        <h3 className="text-sm font-medium text-text-default">Authentication Mode</h3>
        <div className="rounded-lg border border-border-default p-3 bg-background-muted/30">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-text-default">
                {authRequired ? 'Enterprise Mode' : 'Local Mode'}
              </p>
              <p className="text-xs text-text-muted">
                {authRequired
                  ? 'Authentication is required by your organization'
                  : 'Running locally — authentication is optional'}
              </p>
            </div>
            <Switch checked={authRequired} disabled variant="mono" />
          </div>
        </div>
      </div>
    </div>
  );
}
