import { useCallback, useEffect, useState } from 'react';
import { client } from '@/api/client.gen';

interface CatalogMode {
  slug: string;
  name: string;
  description: string;
  whenToUse: string;
  toolGroups: string[];
}

interface CatalogAgent {
  name: string;
  description: string;
  enabled: boolean;
  modes: CatalogMode[];
}

interface CatalogResponse {
  agents: CatalogAgent[];
}

const toolGroupColors: Record<string, string> = {
  read: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  edit: 'bg-amber-500/15 text-amber-400 border-amber-500/30',
  command: 'bg-purple-500/15 text-purple-400 border-purple-500/30',
  mcp: 'bg-green-500/15 text-green-400 border-green-500/30',
};

function ToolBadge({ group }: { group: string }) {
  const label = group.replace(/\s*\(restricted\)/, '');
  const isRestricted = group.includes('(restricted)');
  const colors = toolGroupColors[label] ?? 'bg-gray-500/15 text-gray-400 border-gray-500/30';
  return (
    <span className={`rounded-full border px-1.5 py-0.5 text-[10px] font-medium ${colors}`}>
      {label}
      {isRestricted && ' ⚠'}
    </span>
  );
}

export default function AgentCatalog() {
  const [catalog, setCatalog] = useState<CatalogAgent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);

  const fetchCatalog = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const baseUrl = client.getConfig().baseUrl || '';
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
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
      const response = await fetch(`${baseUrl}/analytics/routing/catalog`, { headers });
      if (!response.ok) throw new Error(`Failed to load catalog: ${response.statusText}`);
      const data: CatalogResponse = await response.json();
      setCatalog(data.agents);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load catalog');
    } finally {
      setLoading(false);
    }
  }, []);

  const handleToggle = useCallback(async (agentName: string) => {
    try {
      setToggling(agentName);
      const baseUrl = client.getConfig().baseUrl || '';
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
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
      const response = await fetch(
        `${baseUrl}/agents/builtin/${encodeURIComponent(agentName)}/toggle`,
        { method: 'POST', headers }
      );
      if (!response.ok) throw new Error(`Toggle failed: ${response.statusText}`);
      const result: { name: string; enabled: boolean } = await response.json();
      setCatalog((prev) =>
        prev.map((agent) =>
          agent.name === result.name ? { ...agent, enabled: result.enabled } : agent
        )
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to toggle ${agentName}`);
    } finally {
      setToggling(null);
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
          type="button"
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
            <div className="flex items-center gap-2">
              <h3 className="text-base font-semibold text-text-default">{agent.name}</h3>
              <button
                type="button"
                onClick={() => handleToggle(agent.name)}
                disabled={toggling === agent.name}
                className={`rounded-full px-2 py-0.5 text-xs font-medium transition-colors cursor-pointer ${
                  toggling === agent.name
                    ? 'bg-background-muted/40 text-text-muted border border-border-default opacity-50'
                    : agent.enabled
                      ? 'bg-background-success-muted text-text-success border border-border-default hover:bg-background-danger-muted hover:text-text-danger'
                      : 'bg-background-muted/40 text-text-muted border border-border-default hover:bg-background-success-muted hover:text-text-success'
                }`}
                title={
                  toggling === agent.name
                    ? 'Toggling…'
                    : agent.enabled
                      ? 'Click to disable'
                      : 'Click to enable'
                }
              >
                {toggling === agent.name ? 'toggling…' : agent.enabled ? 'enabled' : 'disabled'}
              </button>
            </div>
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
                  {mode.toolGroups.length > 0 && (
                    <div className="flex items-center gap-1 ml-2">
                      {mode.toolGroups.map((tg) => (
                        <ToolBadge key={tg} group={tg} />
                      ))}
                    </div>
                  )}
                </div>
                {mode.description && (
                  <p className="text-sm text-text-muted mt-1">{mode.description}</p>
                )}
                {mode.whenToUse && (
                  <p className="text-xs text-text-muted mt-1 italic">
                    When to use: {mode.whenToUse}
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
