import { useState } from 'react';
import { client } from '../../api/client.gen';

interface TestCaseResult {
  input: string;
  expected_agent: string;
  expected_mode: string;
  actual_agent: string;
  actual_mode: string;
  pass: boolean;
}

interface ModeAccuracy {
  mode: string;
  accuracy: number;
  total: number;
  correct: number;
}

interface AgentAccuracy {
  agent: string;
  accuracy: number;
  total: number;
  correct: number;
}

interface EvalResult {
  overall_accuracy: number;
  total_cases: number;
  passed: number;
  failed: number;
  per_agent: AgentAccuracy[];
  per_mode: ModeAccuracy[];
  results: TestCaseResult[];
  confusion_matrix?: Record<string, Record<string, number>>;
}

const EXAMPLE_YAML = `# Eval test set — YAML format
# Each entry has an input message and the expected routing
- input: "Write a Python script to sort a list"
  expected_agent: developer
  expected_mode: code
- input: "Summarize this document for me"
  expected_agent: default
  expected_mode: chat
- input: "Create a REST API with Express"
  expected_agent: developer
  expected_mode: code
`;

export default function EvalRunner() {
  const yamlInputId = 'eval-runner-yaml-input';
  const [yamlInput, setYamlInput] = useState(EXAMPLE_YAML);
  const [result, setResult] = useState<EvalResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRun = async () => {
    if (!yamlInput.trim()) return;
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
      const resp = await fetch(`${baseUrl}/analytics/routing/eval`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ yaml_content: yamlInput.trim() }),
      });
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
      }
      const data: EvalResult = await resp.json();
      setResult(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Request failed');
    } finally {
      setLoading(false);
    }
  };

  const handleFileLoad = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.yaml,.yml';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        const text = await file.text();
        setYamlInput(text);
      }
    };
    input.click();
  };

  const confusionKeys = result?.confusion_matrix ? Object.keys(result.confusion_matrix).sort() : [];

  return (
    <div className="space-y-4">
      {/* Input area */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label htmlFor={yamlInputId} className="text-sm font-medium text-text-default">
            Eval Test Set (YAML)
          </label>
          <button
            type="button"
            onClick={handleFileLoad}
            className="rounded-md border border-border-default px-3 py-1 text-xs text-text-muted hover:bg-background-muted hover:text-text-default"
          >
            Load from file…
          </button>
        </div>
        <textarea
          id={yamlInputId}
          value={yamlInput}
          onChange={(e) => setYamlInput(e.target.value)}
          rows={10}
          className="w-full rounded-md border border-border-default bg-background-muted px-3 py-2 text-sm text-text-default font-mono placeholder-text-subtle focus:border-border-accent focus:outline-none resize-y"
          placeholder="Paste YAML eval set here..."
        />
        <button
          type="button"
          onClick={handleRun}
          disabled={loading || !yamlInput.trim()}
          className="rounded-md bg-background-accent px-4 py-2 text-sm font-medium text-text-on-accent hover:bg-background-accent disabled:opacity-50"
        >
          {loading ? 'Running eval…' : 'Run Eval'}
        </button>
      </div>

      {error && (
        <div className="rounded-md bg-background-danger-muted border border-border-default px-3 py-2 text-sm text-text-danger">
          {error}
        </div>
      )}

      {result && (
        <div className="space-y-4">
          {/* Overall metrics */}
          <div className="rounded-lg border border-border-default bg-background-muted p-4">
            <h3 className="text-sm font-semibold text-text-default mb-3">Overall Results</h3>
            <div className="flex items-center gap-6 text-sm">
              <div className="text-center">
                <div className="text-2xl font-bold text-text-default">
                  {(result.overall_accuracy * 100).toFixed(1)}%
                </div>
                <div className="text-text-muted">Accuracy</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-text-success">{result.passed}</div>
                <div className="text-text-muted">Passed</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-text-danger">{result.failed}</div>
                <div className="text-text-muted">Failed</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-text-default">{result.total_cases}</div>
                <div className="text-text-muted">Total</div>
              </div>
            </div>
            {/* Overall accuracy bar */}
            <div className="mt-3 h-2 w-full rounded-full bg-background-muted">
              <div
                className="h-2 rounded-full bg-background-accent transition-all"
                style={{ width: `${result.overall_accuracy * 100}%` }}
              />
            </div>
          </div>

          {/* Per-agent accuracy */}
          {result.per_agent.length > 0 && (
            <div className="rounded-lg border border-border-default bg-background-muted p-4">
              <h3 className="text-sm font-semibold text-text-default mb-3">Per-Agent Accuracy</h3>
              <div className="space-y-2">
                {result.per_agent.map((a) => (
                  <div key={a.agent} className="flex items-center gap-3 text-sm">
                    <span className="w-28 text-text-muted truncate" title={a.agent}>
                      {a.agent}
                    </span>
                    <div className="flex-1 h-4 rounded bg-background-muted relative overflow-hidden">
                      <div
                        className="h-4 rounded bg-background-success-muted transition-all"
                        style={{ width: `${a.accuracy * 100}%` }}
                      />
                    </div>
                    <span className="w-20 text-right text-text-default font-mono">
                      {(a.accuracy * 100).toFixed(1)}% ({a.correct}/{a.total})
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Per-mode accuracy */}
          {result.per_mode.length > 0 && (
            <div className="rounded-lg border border-border-default bg-background-muted p-4">
              <h3 className="text-sm font-semibold text-text-default mb-3">Per-Mode Accuracy</h3>
              <div className="space-y-2">
                {result.per_mode.map((m) => (
                  <div key={m.mode} className="flex items-center gap-3 text-sm">
                    <span className="w-28 text-text-muted truncate" title={m.mode}>
                      {m.mode}
                    </span>
                    <div className="flex-1 h-4 rounded bg-background-muted relative overflow-hidden">
                      <div
                        className="h-4 rounded bg-purple-600 transition-all"
                        style={{ width: `${m.accuracy * 100}%` }}
                      />
                    </div>
                    <span className="w-20 text-right text-text-default font-mono">
                      {(m.accuracy * 100).toFixed(1)}% ({m.correct}/{m.total})
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Results table */}
          <div className="rounded-lg border border-border-default bg-background-muted overflow-hidden">
            <h3 className="text-sm font-semibold text-text-default px-4 pt-3 pb-2">
              Test Case Results
            </h3>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border-default text-left text-text-muted">
                    <th className="px-4 py-2 w-8"></th>
                    <th className="px-4 py-2">Input</th>
                    <th className="px-4 py-2">Expected</th>
                    <th className="px-4 py-2">Actual</th>
                  </tr>
                </thead>
                <tbody>
                  {result.results.map((r, i) => (
                    <tr key={i} className="border-b border-border-muted">
                      <td className="px-4 py-2">
                        {r.pass ? (
                          <span className="text-text-success" title="Pass">
                            ✓
                          </span>
                        ) : (
                          <span className="text-text-danger" title="Fail">
                            ✗
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-2 text-text-default max-w-xs truncate" title={r.input}>
                        {r.input}
                      </td>
                      <td className="px-4 py-2 text-text-muted">
                        {r.expected_agent}/{r.expected_mode}
                      </td>
                      <td
                        className={`px-4 py-2 ${r.pass ? 'text-text-default' : 'text-text-danger'}`}
                      >
                        {r.actual_agent}/{r.actual_mode}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Confusion matrix */}
          {confusionKeys.length > 0 && result.confusion_matrix && (
            <div className="rounded-lg border border-border-default bg-background-muted p-4">
              <h3 className="text-sm font-semibold text-text-default mb-3">
                Confusion Matrix (Expected → Actual)
              </h3>
              <div className="overflow-x-auto">
                <table className="text-sm">
                  <thead>
                    <tr>
                      <th className="px-3 py-1 text-text-muted text-left">Expected ↓ / Actual →</th>
                      {confusionKeys.map((k) => (
                        <th key={k} className="px-3 py-1 text-text-muted text-center">
                          {k}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {confusionKeys.map((expected) => (
                      <tr key={expected}>
                        <td className="px-3 py-1 text-text-muted font-medium">{expected}</td>
                        {confusionKeys.map((actual) => {
                          const count = result.confusion_matrix?.[expected]?.[actual] ?? 0;
                          const isDiagonal = expected === actual;
                          return (
                            <td
                              key={actual}
                              className={`px-3 py-1 text-center font-mono ${
                                count === 0
                                  ? 'text-text-subtle'
                                  : isDiagonal
                                    ? 'text-text-success bg-background-success-muted'
                                    : 'text-text-danger bg-background-danger-muted'
                              }`}
                            >
                              {count}
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
