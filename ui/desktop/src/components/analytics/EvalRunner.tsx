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
        const secretKey = typeof h.get === 'function' ? (h as unknown as globalThis.Headers).get('X-Secret-Key') : h['X-Secret-Key'];
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

  const confusionKeys = result?.confusion_matrix
    ? Object.keys(result.confusion_matrix).sort()
    : [];

  return (
    <div className="space-y-4">
      {/* Input area */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium text-gray-300">
            Eval Test Set (YAML)
          </label>
          <button
            onClick={handleFileLoad}
            className="rounded-md border border-gray-600 px-3 py-1 text-xs text-gray-400 hover:bg-gray-700 hover:text-gray-200"
          >
            Load from file…
          </button>
        </div>
        <textarea
          value={yamlInput}
          onChange={(e) => setYamlInput(e.target.value)}
          rows={10}
          className="w-full rounded-md border border-gray-600 bg-gray-800 px-3 py-2 text-sm text-gray-100 font-mono placeholder-gray-500 focus:border-blue-500 focus:outline-none resize-y"
          placeholder="Paste YAML eval set here..."
        />
        <button
          onClick={handleRun}
          disabled={loading || !yamlInput.trim()}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? 'Running eval…' : 'Run Eval'}
        </button>
      </div>

      {error && (
        <div className="rounded-md bg-red-900/50 border border-red-700 px-3 py-2 text-sm text-red-300">
          {error}
        </div>
      )}

      {result && (
        <div className="space-y-4">
          {/* Overall metrics */}
          <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
            <h3 className="text-sm font-semibold text-gray-300 mb-3">Overall Results</h3>
            <div className="flex items-center gap-6 text-sm">
              <div className="text-center">
                <div className="text-2xl font-bold text-gray-100">
                  {(result.overall_accuracy * 100).toFixed(1)}%
                </div>
                <div className="text-gray-500">Accuracy</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-green-400">{result.passed}</div>
                <div className="text-gray-500">Passed</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-red-400">{result.failed}</div>
                <div className="text-gray-500">Failed</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-medium text-gray-300">{result.total_cases}</div>
                <div className="text-gray-500">Total</div>
              </div>
            </div>
            {/* Overall accuracy bar */}
            <div className="mt-3 h-2 w-full rounded-full bg-gray-700">
              <div
                className="h-2 rounded-full bg-blue-500 transition-all"
                style={{ width: `${result.overall_accuracy * 100}%` }}
              />
            </div>
          </div>

          {/* Per-agent accuracy */}
          {result.per_agent.length > 0 && (
            <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
              <h3 className="text-sm font-semibold text-gray-300 mb-3">Per-Agent Accuracy</h3>
              <div className="space-y-2">
                {result.per_agent.map((a) => (
                  <div key={a.agent} className="flex items-center gap-3 text-sm">
                    <span className="w-28 text-gray-400 truncate" title={a.agent}>
                      {a.agent}
                    </span>
                    <div className="flex-1 h-4 rounded bg-gray-700 relative overflow-hidden">
                      <div
                        className="h-4 rounded bg-green-600 transition-all"
                        style={{ width: `${a.accuracy * 100}%` }}
                      />
                    </div>
                    <span className="w-20 text-right text-gray-300 font-mono">
                      {(a.accuracy * 100).toFixed(1)}% ({a.correct}/{a.total})
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Per-mode accuracy */}
          {result.per_mode.length > 0 && (
            <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
              <h3 className="text-sm font-semibold text-gray-300 mb-3">Per-Mode Accuracy</h3>
              <div className="space-y-2">
                {result.per_mode.map((m) => (
                  <div key={m.mode} className="flex items-center gap-3 text-sm">
                    <span className="w-28 text-gray-400 truncate" title={m.mode}>
                      {m.mode}
                    </span>
                    <div className="flex-1 h-4 rounded bg-gray-700 relative overflow-hidden">
                      <div
                        className="h-4 rounded bg-purple-600 transition-all"
                        style={{ width: `${m.accuracy * 100}%` }}
                      />
                    </div>
                    <span className="w-20 text-right text-gray-300 font-mono">
                      {(m.accuracy * 100).toFixed(1)}% ({m.correct}/{m.total})
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Results table */}
          <div className="rounded-lg border border-gray-700 bg-gray-800/50 overflow-hidden">
            <h3 className="text-sm font-semibold text-gray-300 px-4 pt-3 pb-2">
              Test Case Results
            </h3>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700 text-left text-gray-500">
                    <th className="px-4 py-2 w-8"></th>
                    <th className="px-4 py-2">Input</th>
                    <th className="px-4 py-2">Expected</th>
                    <th className="px-4 py-2">Actual</th>
                  </tr>
                </thead>
                <tbody>
                  {result.results.map((r, i) => (
                    <tr key={i} className="border-b border-gray-700/50">
                      <td className="px-4 py-2">
                        {r.pass ? (
                          <span className="text-green-400" title="Pass">✓</span>
                        ) : (
                          <span className="text-red-400" title="Fail">✗</span>
                        )}
                      </td>
                      <td className="px-4 py-2 text-gray-300 max-w-xs truncate" title={r.input}>
                        {r.input}
                      </td>
                      <td className="px-4 py-2 text-gray-400">
                        {r.expected_agent}/{r.expected_mode}
                      </td>
                      <td
                        className={`px-4 py-2 ${r.pass ? 'text-gray-300' : 'text-red-300'}`}
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
            <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
              <h3 className="text-sm font-semibold text-gray-300 mb-3">
                Confusion Matrix (Expected → Actual)
              </h3>
              <div className="overflow-x-auto">
                <table className="text-sm">
                  <thead>
                    <tr>
                      <th className="px-3 py-1 text-gray-500 text-left">Expected ↓ / Actual →</th>
                      {confusionKeys.map((k) => (
                        <th key={k} className="px-3 py-1 text-gray-400 text-center">
                          {k}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {confusionKeys.map((expected) => (
                      <tr key={expected}>
                        <td className="px-3 py-1 text-gray-400 font-medium">{expected}</td>
                        {confusionKeys.map((actual) => {
                          const count = result.confusion_matrix?.[expected]?.[actual] ?? 0;
                          const isDiagonal = expected === actual;
                          return (
                            <td
                              key={actual}
                              className={`px-3 py-1 text-center font-mono ${
                                count === 0
                                  ? 'text-gray-600'
                                  : isDiagonal
                                    ? 'text-green-400 bg-green-900/20'
                                    : 'text-red-400 bg-red-900/20'
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
