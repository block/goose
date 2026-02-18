import { useState } from 'react';
import { client } from '../../api/client.gen';

interface ModeScore {
  agent: string;
  mode: string;
  score: number;
  matched_keywords: string[];
}

interface InspectResult {
  chosen_agent: string;
  chosen_mode: string;
  confidence: number;
  reasoning: string;
  scores: ModeScore[];
}

export default function RoutingInspector() {
  const [message, setMessage] = useState('');
  const [result, setResult] = useState<InspectResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async () => {
    if (!message.trim()) return;
    setLoading(true);
    setError(null);
    try {
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
      const resp = await fetch(`${baseUrl}/analytics/routing/inspect`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ message: message.trim() }),
      });
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
      }
      const data: InspectResult = await resp.json();
      setResult(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Request failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <input
          type="text"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSubmit()}
          placeholder="Enter a message to inspect routing..."
          className="flex-1 rounded-md border border-border-default bg-background-muted px-3 py-2 text-sm text-text-default placeholder-text-subtle focus:border-border-accent focus:outline-none"
        />
        <button
          onClick={handleSubmit}
          disabled={loading || !message.trim()}
          className="rounded-md bg-background-accent px-4 py-2 text-sm font-medium text-text-on-accent hover:bg-background-accent disabled:opacity-50"
        >
          {loading ? 'Inspecting…' : 'Inspect'}
        </button>
      </div>

      {error && (
        <div className="rounded-md bg-background-danger-muted border border-border-default px-3 py-2 text-sm text-text-danger">
          {error}
        </div>
      )}

      {result && (
        <div className="space-y-4">
          {/* Decision summary */}
          <div className="rounded-lg border border-border-default bg-background-muted p-4">
            <h3 className="text-sm font-semibold text-text-default mb-2">Routing Decision</h3>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div>
                <span className="text-text-muted">Agent:</span>{' '}
                <span className="text-text-default font-medium">{result.chosen_agent}</span>
              </div>
              <div>
                <span className="text-text-muted">Mode:</span>{' '}
                <span className="text-text-default font-medium">{result.chosen_mode}</span>
              </div>
              <div>
                <span className="text-text-muted">Confidence:</span>{' '}
                <span className="text-text-default font-medium">
                  {(result.confidence * 100).toFixed(1)}%
                </span>
              </div>
            </div>
            {result.reasoning && <p className="mt-2 text-sm text-text-muted">{result.reasoning}</p>}
          </div>

          {/* Scores table */}
          <div className="rounded-lg border border-border-default bg-background-muted overflow-hidden">
            <h3 className="text-sm font-semibold text-text-default px-4 pt-3 pb-2">
              All Mode Scores
            </h3>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border-default text-left text-text-muted">
                  <th className="px-4 py-2">Agent</th>
                  <th className="px-4 py-2">Mode</th>
                  <th className="px-4 py-2">Score</th>
                  <th className="px-4 py-2">Keywords</th>
                </tr>
              </thead>
              <tbody>
                {[...result.scores]
                  .sort((a, b) => b.score - a.score)
                  .map((s, i) => (
                    <tr
                      key={`${s.agent}-${s.mode}-${i}`}
                      className={`border-b border-border-muted ${
                        s.agent === result.chosen_agent && s.mode === result.chosen_mode
                          ? 'bg-background-muted'
                          : ''
                      }`}
                    >
                      <td className="px-4 py-2 text-text-default">{s.agent}</td>
                      <td className="px-4 py-2 text-text-default">{s.mode}</td>
                      <td className="px-4 py-2 text-text-default font-mono">
                        {s.score.toFixed(3)}
                      </td>
                      <td className="px-4 py-2">
                        <div className="flex flex-wrap gap-1">
                          {s.matched_keywords.map((kw) => (
                            <span
                              key={kw}
                              className="rounded bg-background-muted px-1.5 py-0.5 text-xs text-text-default"
                            >
                              {kw}
                            </span>
                          ))}
                        </div>
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
