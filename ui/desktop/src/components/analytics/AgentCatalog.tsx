import { useCallback, useEffect, useState } from 'react';
import { client } from '../../api/client.gen';

interface CatalogMode {
  slug: string;
  name: string;
  description: string;
  when_to_use: string;
  enabled: boolean;
}

interface CatalogAgent {
  name: string;
  description: string;
  modes: CatalogMode[];
}

interface CatalogResponse {
  agents: CatalogAgent[];
}

export default function AgentCatalog() {
  const [catalog, setCatalog] = useState<CatalogAgent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchCatalog = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const baseUrl = client.getConfig().baseUrl || '';
      const headers: Record<string, string> = {};
      const rawHeaders = client.getConfig().headers;
      if (rawHeaders) {
        const h = rawHeaders as Record<string, string>;
        const secretKey =
          typeof h.get === 'function'
            ? (h as unknown as globalThis.Headers).get('X-Secret-Key')
            : h['X-Secret-Key'];
        if (secretKey) {
          headers['X-Secret-Key'] = secretKey;
        }
      }
      const resp = await fetch(`${baseUrl}/analytics/routing/catalog`, { headers });
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
      }
      const data: CatalogResponse = await resp.json();
      setCatalog(data.agents);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load catalog');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCatalog();
  }, [fetchCatalog]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12 text-text-muted text-sm">
        Loading catalog…
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-3">
        <div className="rounded-md bg-background-danger-muted border border-border-default px-3 py-2 text-sm text-text-danger">
          {error}
        </div>
        <button
          onClick={fetchCatalog}
          className="rounded-md border border-border-default px-3 py-1.5 text-sm text-text-muted hover:bg-background-muted hover:text-text-default"
        >
          Retry
        </button>
      </div>
    );
  }

  if (catalog.length === 0) {
    return (
      <div className="text-center py-12 text-text-muted text-sm">No agents found in catalog.</div>
    );
  }

  return (
    <div className="space-y-4">
      {catalog.map((agent) => (
        <div
          key={agent.name}
          className="rounded-lg border border-border-default bg-background-muted overflow-hidden"
        >
          {/* Agent header */}
          <div className="px-4 py-3 border-b border-border-default">
            <h3 className="text-base font-semibold text-text-default">{agent.name}</h3>
            {agent.description && (
              <p className="text-sm text-text-muted mt-0.5">{agent.description}</p>
            )}
            <span className="text-xs text-text-muted mt-1 inline-block">
              {agent.modes.length} mode{agent.modes.length !== 1 ? 's' : ''}
            </span>
          </div>

          {/* Modes list */}
          <div className="divide-y divide-gray-700/50">
            {agent.modes.map((mode) => (
              <div key={mode.slug} className="px-4 py-3">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm text-text-default">{mode.slug}</span>
                  {mode.name && mode.name !== mode.slug && (
                    <span className="text-sm text-text-muted">— {mode.name}</span>
                  )}
                  <span
                    className={`ml-auto rounded-full px-2 py-0.5 text-xs font-medium ${
                      mode.enabled
                        ? 'bg-background-success-muted text-text-success border border-border-default'
                        : 'bg-background-muted/40 text-text-muted border border-border-default'
                    }`}
                  >
                    {mode.enabled ? 'enabled' : 'disabled'}
                  </span>
                </div>
                {mode.description && (
                  <p className="text-sm text-text-muted mt-1">{mode.description}</p>
                )}
                {mode.when_to_use && (
                  <p className="text-xs text-text-muted mt-1 italic">
                    When to use: {mode.when_to_use}
                  </p>
                )}
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
