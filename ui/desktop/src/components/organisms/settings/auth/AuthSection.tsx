import { useCallback, useEffect, useState } from 'react';
import { addOidcProvider, listOidcProviders, removeOidcProvider } from '@/api';
import { useAuth } from '@/hooks/useAuth';
import { Button } from '@/components/atoms/button';
import { Input } from '@/components/atoms/input';
import { Separator } from '@/components/atoms/separator';

/* ───────── Auth method presets ───────── */

const AUTH_PRESETS = [
  {
    id: 'none',
    label: 'No Auth',
    description: 'Open access, no authentication required',
  },
  {
    id: 'github',
    label: 'GitHub',
    description: 'Sign in with GitHub SSO',
    issuer: 'https://token.actions.githubusercontent.com',
    docs: "Use your GitHub OAuth App's client ID and secret.",
  },
  {
    id: 'gitlab',
    label: 'GitLab',
    description: 'Sign in with GitLab SSO',
    issuer: 'https://gitlab.com',
    docs: "Use your GitLab Application's client ID and secret.",
  },
  {
    id: 'azure',
    label: 'Azure AD',
    description: 'Microsoft Entra ID / Azure AD',
    issuer: '',
    issuerPlaceholder: 'https://login.microsoftonline.com/{tenant}/v2.0',
    docs: 'Enter your Azure AD tenant issuer URL, application (client) ID, and client secret.',
  },
  {
    id: 'google',
    label: 'Google',
    description: 'Sign in with Google Workspace',
    issuer: 'https://accounts.google.com',
    docs: 'Use your Google OAuth 2.0 client ID and secret.',
  },
  {
    id: 'custom',
    label: 'Custom OIDC',
    description: 'Any OpenID Connect provider',
    issuer: '',
    docs: 'Enter the OIDC discovery issuer URL for your provider.',
  },
  {
    id: 'apikey',
    label: 'API Key',
    description: 'Shared secret via X-Api-Key header',
  },
] as const;

type PresetId = (typeof AUTH_PRESETS)[number]['id'];

interface OidcFields {
  issuer: string;
  audience: string;
  clientSecret: string;
  tenantClaim: string;
  groupClaim: string;
  requiredGroups: string;
}

const EMPTY_OIDC: OidcFields = {
  issuer: '',
  audience: '',
  clientSecret: '',
  tenantClaim: '',
  groupClaim: '',
  requiredGroups: '',
};

/* ───────── Preset Card ───────── */

function PresetCard({
  preset,
  selected,
  configured,
  onSelect,
}: {
  preset: (typeof AUTH_PRESETS)[number];
  selected: boolean;
  configured: boolean;
  onSelect: () => void;
}) {
  return (
    <button type="button"
      onClick={onSelect}
      className={`relative flex flex-col items-center justify-center gap-1 rounded-lg border p-4 text-center transition-all duration-200 h-[100px]
        ${
          selected
            ? 'border-primary bg-primary/10 ring-1 ring-primary/30'
            : 'border-border/50 bg-card hover:border-border hover:bg-accent/30'
        }`}
    >
      {configured && (
        <div className="absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-emerald-400" />
      )}
      <span className="text-sm font-medium text-card-foreground">{preset.label}</span>
      <span className="text-[10px] text-muted-foreground leading-tight">{preset.description}</span>
    </button>
  );
}

/* ───────── Field helper ───────── */

function Field({
  label,
  required,
  hint,
  children,
}: {
  label: string;
  required?: boolean;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <label className="text-xs font-medium text-muted-foreground">
        {label}
        {required && <span className="text-red-400 ml-0.5">*</span>}
      </label>
      {children}
      {hint && <p className="text-[10px] text-muted-foreground/60">{hint}</p>}
    </div>
  );
}

/* ───────── Main component ───────── */

export default function AuthSection() {
  const { isAuthenticated, user, loginWithApiKey, loginWithOidc, logout } = useAuth();

  const [selected, setSelected] = useState<PresetId>('none');
  const [oidc, setOidc] = useState<OidcFields>(EMPTY_OIDC);
  const [apiKey, setApiKey] = useState('');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentIssuer, setCurrentIssuer] = useState<string | null>(null);

  // Load existing configuration
  useEffect(() => {
    (async () => {
      try {
        const resp = await listOidcProviders();
        const providers = resp.data?.providers ?? [];
        if (providers.length > 0) {
          const issuer = providers[0].issuer;
          setCurrentIssuer(issuer);

          // Match to a preset
          const match = AUTH_PRESETS.find(
            (p) =>
              'issuer' in p &&
              p.issuer &&
              issuer.toLowerCase().includes(p.issuer.replace('https://', ''))
          );
          setSelected(match ? (match.id as PresetId) : 'custom');
          setOidc((prev) => ({
            ...prev,
            issuer,
            audience: providers[0].audience ?? '',
          }));
        } else if (isAuthenticated && user?.auth_method === 'api_key') {
          setSelected('apikey');
        }
      } catch {
        // No providers — keep defaults
      }
    })();
  }, [isAuthenticated, user?.auth_method]);

  const handleSelect = useCallback((id: PresetId) => {
    setSelected(id);
    setError(null);
    setSaved(false);

    const preset = AUTH_PRESETS.find((p) => p.id === id);
    if (preset && 'issuer' in preset && preset.issuer) {
      setOidc((prev) => ({ ...prev, issuer: preset.issuer as string }));
    } else if (id === 'none' || id === 'apikey') {
      setOidc(EMPTY_OIDC);
    }
  }, []);

  const handleSave = useCallback(async () => {
    setError(null);
    setSaving(true);
    setSaved(false);

    try {
      if (selected === 'none') {
        if (currentIssuer) {
          await removeOidcProvider({ body: { issuer: currentIssuer } });
          setCurrentIssuer(null);
        }
        setSaved(true);
        return;
      }

      if (selected === 'apikey') {
        if (!apiKey.trim()) {
          setError('API key is required.');
          return;
        }
        await loginWithApiKey(apiKey.trim());
        setSaved(true);
        return;
      }

      // OIDC provider
      if (!oidc.issuer.trim()) {
        setError('Issuer URL is required.');
        return;
      }
      if (!oidc.audience.trim()) {
        setError('Client ID is required.');
        return;
      }

      // Remove previous if different
      if (currentIssuer && currentIssuer !== oidc.issuer.trim()) {
        await removeOidcProvider({ body: { issuer: currentIssuer } });
      }

      await addOidcProvider({
        body: {
          issuer: oidc.issuer.trim(),
          audience: oidc.audience.trim(),
          client_secret: oidc.clientSecret.trim() || null,
          tenant_claim: oidc.tenantClaim.trim() || null,
          group_claim: oidc.groupClaim.trim() || null,
          required_groups: oidc.requiredGroups.trim()
            ? oidc.requiredGroups.split(',').map((g) => g.trim())
            : undefined,
        },
      });

      setCurrentIssuer(oidc.issuer.trim());
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save.');
    } finally {
      setSaving(false);
    }
  }, [selected, apiKey, oidc, currentIssuer, loginWithApiKey]);

  const handleSignIn = useCallback(async () => {
    if (oidc.issuer) {
      await loginWithOidc(oidc.issuer);
    }
  }, [oidc.issuer, loginWithOidc]);

  const preset = AUTH_PRESETS.find((p) => p.id === selected);
  const isOidc = preset && 'issuer' in preset;

  return (
    <div className="space-y-6">
      {/* Active session banner */}
      {isAuthenticated && (
        <div className="flex items-center justify-between rounded-md border border-emerald-500/20 bg-emerald-500/5 px-3 py-2">
          <span className="text-xs text-muted-foreground">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400 mr-1.5 align-middle" />
            Signed in{user?.name ? ` as ${user.name}` : ''}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={logout}
            className="h-6 text-[11px] text-red-400 hover:text-red-300"
          >
            Sign out
          </Button>
        </div>
      )}

      {/* Method grid */}
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-card-foreground">Authentication Method</h3>
        <div
          className="grid gap-2"
          style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(130px, 1fr))' }}
        >
          {AUTH_PRESETS.map((p) => (
            <PresetCard
              key={p.id}
              preset={p}
              selected={selected === p.id}
              configured={
                (p.id === 'apikey' && isAuthenticated && user?.auth_method === 'api_key') ||
                ('issuer' in p && p.issuer
                  ? (currentIssuer?.includes(p.issuer.replace('https://', '')) ?? false)
                  : false) ||
                (p.id === 'custom' &&
                  currentIssuer != null &&
                  !AUTH_PRESETS.some(
                    (q) =>
                      q.id !== 'custom' &&
                      'issuer' in q &&
                      q.issuer &&
                      currentIssuer.includes(q.issuer.replace('https://', ''))
                  ))
              }
              onSelect={() => handleSelect(p.id)}
            />
          ))}
        </div>
      </div>

      <Separator />

      {/* Configuration form */}
      {selected === 'none' && (
        <p className="text-xs text-muted-foreground">
          No authentication. Anyone with access to this machine can use Goose.
          {currentIssuer && (
            <span className="block mt-1 text-amber-400">
              Saving will remove the currently configured OIDC provider.
            </span>
          )}
        </p>
      )}

      {selected === 'apikey' && (
        <div className="space-y-3 max-w-md">
          <Field label="API Key" required hint="Clients must include this in the X-Api-Key header.">
            <Input
              type="password"
              placeholder="Enter shared secret"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </Field>
        </div>
      )}

      {isOidc && (
        <div className="space-y-4 max-w-md">
          {preset && 'docs' in preset && (
            <p className="text-xs text-muted-foreground">{preset.docs}</p>
          )}

          <Field label="Issuer URL" required hint="The OpenID Connect discovery endpoint">
            <Input
              placeholder={
                (('issuerPlaceholder' in preset!
                  ? preset.issuerPlaceholder
                  : preset?.issuer) as string) || 'https://...'
              }
              value={oidc.issuer}
              onChange={(e) => setOidc((prev) => ({ ...prev, issuer: e.target.value }))}
            />
          </Field>

          <Field label="Client ID" required hint="Also called audience or application ID">
            <Input
              placeholder="your-client-id"
              value={oidc.audience}
              onChange={(e) => setOidc((prev) => ({ ...prev, audience: e.target.value }))}
            />
          </Field>

          <Field label="Client Secret" hint="Required for confidential clients">
            <Input
              type="password"
              placeholder="Optional"
              value={oidc.clientSecret}
              onChange={(e) => setOidc((prev) => ({ ...prev, clientSecret: e.target.value }))}
            />
          </Field>

          {/* Advanced */}
          <details className="group">
            <summary className="cursor-pointer text-xs text-muted-foreground hover:text-card-foreground select-none">
              Advanced ▸
            </summary>
            <div className="mt-3 space-y-3 pl-3 border-l border-border/30">
              <Field label="Tenant Claim" hint="JWT claim for tenant ID (e.g., tid)">
                <Input
                  placeholder="tid"
                  value={oidc.tenantClaim}
                  onChange={(e) => setOidc((prev) => ({ ...prev, tenantClaim: e.target.value }))}
                />
              </Field>
              <Field label="Group Claim" hint="JWT claim for group membership">
                <Input
                  placeholder="groups"
                  value={oidc.groupClaim}
                  onChange={(e) => setOidc((prev) => ({ ...prev, groupClaim: e.target.value }))}
                />
              </Field>
              <Field label="Required Groups" hint="Comma-separated list of required groups">
                <Input
                  placeholder="admin, developers"
                  value={oidc.requiredGroups}
                  onChange={(e) => setOidc((prev) => ({ ...prev, requiredGroups: e.target.value }))}
                />
              </Field>
            </div>
          </details>
        </div>
      )}

      <Separator />

      {/* Actions */}
      <div className="flex items-center gap-3">
        <Button onClick={handleSave} disabled={saving} size="sm">
          {saving ? 'Saving…' : 'Save'}
        </Button>

        {isOidc && currentIssuer && (
          <Button variant="outline" size="sm" onClick={handleSignIn}>
            Sign in
          </Button>
        )}

        {saved && <span className="text-xs text-emerald-400">✓ Saved</span>}
        {error && <span className="text-xs text-red-400">{error}</span>}
      </div>
    </div>
  );
}
