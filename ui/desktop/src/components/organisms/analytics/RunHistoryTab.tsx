import { useCallback, useEffect, useRef, useState } from 'react';
import { getEvalRun, listEvalDatasets, listEvalRuns, runEval, type EvalDatasetSummary, type EvalRunDetail, type EvalRunSummary } from '@/api';
import RunComparisonView from './RunComparisonView';
import SankeyDiagram from './SankeyDiagram';

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function AccuracyBadge({ value }: { value: number }) {
  const color =
    value >= 0.9
      ? 'bg-background-success-muted text-text-success border-border-default'
      : value >= 0.7
        ? 'bg-background-warning-muted text-text-warning border-border-default'
        : 'bg-background-danger-muted text-text-danger border-border-default';
  return (
    <span className={`text-xs px-2 py-0.5 rounded-full border ${color}`}>
      {formatPercent(value)}
    </span>
  );
}

function RunDetailPanel({ detail, onClose }: { detail: EvalRunDetail; onClose: () => void }) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <button type="button"
            onClick={onClose}
            className="text-sm text-text-accent hover:text-text-accent mb-2"
          >
            ← Back to runs
          </button>
          <h3 className="text-lg font-semibold text-text-default">Run #{detail.id.slice(0, 8)}</h3>
          <div className="flex gap-4 mt-1 text-xs text-text-muted">
            <span>Dataset: {detail.datasetName}</span>
            <span>{formatDate(detail.startedAt)}</span>
            {detail.gooseVersion && <span>Goose: {detail.gooseVersion}</span>}
          </div>
        </div>
        <div className="flex gap-3 text-sm">
          <div className="text-center">
            <div className="text-text-muted text-xs">Overall</div>
            <AccuracyBadge value={detail.overallAccuracy} />
          </div>
          <div className="text-center">
            <div className="text-text-muted text-xs">Agent</div>
            <AccuracyBadge value={detail.agentAccuracy} />
          </div>
          <div className="text-center">
            <div className="text-text-muted text-xs">Mode</div>
            <AccuracyBadge value={detail.modeAccuracy} />
          </div>
        </div>
      </div>

      {/* Per-Agent Results */}
      {detail.perAgent.length > 0 && (
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-3">Per-Agent Results</h4>
          <div className="grid grid-cols-2 gap-3">
            {detail.perAgent.map((ar) => (
              <div
                key={ar.agent}
                className="rounded-lg border border-border-muted bg-background-default/30 p-3"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-medium text-text-default">{ar.agent}</span>
                  <AccuracyBadge value={ar.accuracy} />
                </div>
                <div className="flex gap-4 text-xs text-text-muted">
                  <span>
                    Pass: <span className="text-text-success">{ar.pass}</span>
                  </span>
                  <span>
                    Fail: <span className="text-text-danger">{ar.fail}</span>
                  </span>
                </div>
                <div className="mt-2 h-1.5 bg-background-muted rounded-full overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all"
                    style={{
                      width: `${ar.accuracy * 100}%`,
                      backgroundColor:
                        ar.accuracy >= 0.9 ? '#22c55e' : ar.accuracy >= 0.7 ? '#f59e0b' : '#ef4444',
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Failures */}
      {detail.failures.length > 0 && (
        <div className="rounded-lg border border-border-default bg-background-danger-muted p-4">
          <h4 className="text-sm font-medium text-text-danger mb-3">
            Failed Cases ({detail.failures.length})
          </h4>
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {detail.failures.map((f) => (
              <div
                key={`${f.input}-${f.expectedAgent}-${f.expectedMode}-${f.actualAgent}-${f.actualMode}`}
                className="rounded bg-background-muted/80 border border-border-muted p-3"
              >
                <p className="text-sm text-text-default mb-2 line-clamp-2">&quot;{f.input}&quot;</p>
                <div className="flex gap-6 text-xs">
                  <div>
                    <span className="text-text-muted">Expected: </span>
                    <span className="text-text-success">
                      {f.expectedAgent} → {f.expectedMode}
                    </span>
                  </div>
                  <div>
                    <span className="text-text-muted">Got: </span>
                    <span className="text-text-danger">
                      {f.actualAgent} → {f.actualMode}
                    </span>
                  </div>
                  <div>
                    <span className="text-text-muted">Confidence: </span>
                    <span className="text-text-default">{(f.confidence * 100).toFixed(0)}%</span>
                  </div>
                </div>
                {f.tags.length > 0 && (
                  <div className="flex gap-1 mt-2">
                    {f.tags.map((t) => (
                      <span
                        key={t}
                        className="text-[10px] px-1.5 py-0.5 rounded bg-background-muted text-text-muted"
                      >
                        {t}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Routing Flow (Sankey) */}
      {detail.confusionMatrix.labels.length > 0 && (
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-3">Routing Flow</h4>
          <SankeyDiagram
            labels={detail.confusionMatrix.labels}
            matrix={detail.confusionMatrix.matrix}
          />
        </div>
      )}

      {/* Confusion Matrix */}
      {detail.confusionMatrix.labels.length > 0 && (
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-3">Confusion Matrix</h4>
          <div className="overflow-x-auto">
            <table className="text-xs">
              <thead>
                <tr>
                  <th className="px-3 py-2 text-left text-text-muted">Expected ↓ / Actual →</th>
                  {detail.confusionMatrix.labels.map((l) => (
                    <th key={l} className="px-3 py-2 text-center text-text-muted">
                      {l}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {detail.confusionMatrix.labels.map((expected, ri) => (
                  <tr key={expected} className="border-t border-border-muted">
                    <td className="px-3 py-2 text-text-muted font-medium">{expected}</td>
                    {detail.confusionMatrix.matrix[ri]?.map((count, ci) => {
                      const isDiagonal = ri === ci;
                      const actual = detail.confusionMatrix.labels[ci] ?? String(ci);
                      return (
                        <td
                          key={actual}
                          className={`px-3 py-2 text-center ${
                            count > 0
                              ? isDiagonal
                                ? 'bg-background-success-muted text-text-success'
                                : 'bg-background-danger-muted text-text-danger'
                              : 'text-text-subtle'
                          }`}
                        >
                          {count || '-'}
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
  );
}

export default function RunHistoryTab({ initialRunId }: { initialRunId?: string }) {
  const [runs, setRuns] = useState<EvalRunSummary[]>([]);
  const [datasets, setDatasets] = useState<EvalDatasetSummary[]>([]);
  const [selectedDetail, setSelectedDetail] = useState<EvalRunDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [selectedDataset, setSelectedDataset] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [compareMode, setCompareMode] = useState(false);
  const [selectedForCompare, setSelectedForCompare] = useState<string[]>([]);
  const [showComparison, setShowComparison] = useState(false);

  const didOpenInitialRun = useRef(false);

  const toggleRunSelection = (runId: string) => {
    setSelectedForCompare((prev) => {
      if (prev.includes(runId)) return prev.filter((id) => id !== runId);
      if (prev.length >= 2) return [prev[1], runId];
      return [...prev, runId];
    });
  };

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      const [runsRes, dsRes] = await Promise.all([listEvalRuns(), listEvalDatasets()]);
      if (runsRes.data) setRuns(runsRes.data);
      if (dsRes.data) {
        setDatasets(dsRes.data);
        if (dsRes.data.length > 0 && !selectedDataset) {
          setSelectedDataset(dsRes.data[0].id);
        }
      }
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load runs');
    } finally {
      setLoading(false);
    }
  }, [selectedDataset]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleRunEval = async () => {
    if (!selectedDataset) return;
    setRunning(true);
    try {
      await runEval({ body: { datasetId: selectedDataset } });
      await fetchData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Eval run failed');
    } finally {
      setRunning(false);
    }
  };

  const handleViewDetail = useCallback(async (runId: string) => {
    try {
      const res = await getEvalRun({ path: { id: runId } });
      if (res.data) setSelectedDetail(res.data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load run detail');
    }
  }, []);

  const openInitialRun = useCallback(async () => {
    if (!initialRunId || didOpenInitialRun.current) {
      return;
    }

    didOpenInitialRun.current = true;
    await handleViewDetail(initialRunId);
  }, [handleViewDetail, initialRunId]);

  useEffect(() => {
    void openInitialRun();
  }, [openInitialRun]);

  if (showComparison && selectedForCompare.length === 2) {
    return (
      <RunComparisonView
        baselineId={selectedForCompare[0]}
        candidateId={selectedForCompare[1]}
        onClose={() => {
          setShowComparison(false);
          setSelectedForCompare([]);
          setCompareMode(false);
        }}
      />
    );
  }

  if (selectedDetail) {
    return <RunDetailPanel detail={selectedDetail} onClose={() => setSelectedDetail(null)} />;
  }

  return (
    <div className="space-y-4">
      {error && (
        <div className="rounded-lg bg-background-danger-muted border border-border-default p-3 text-text-danger text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-semibold text-text-default">Run History</h3>
          {runs.length >= 2 && (
            <button type="button"
              onClick={() => {
                setCompareMode(!compareMode);
                if (compareMode) setSelectedForCompare([]);
              }}
              className={`px-3 py-1 rounded-lg text-xs font-medium transition-colors ${
                compareMode
                  ? 'bg-purple-600 text-text-default'
                  : 'bg-background-muted text-text-default hover:bg-background-muted'
              }`}
            >
              {compareMode ? 'Cancel Compare' : '⇆ Compare Runs'}
            </button>
          )}
          {compareMode && selectedForCompare.length === 2 && (
            <button type="button"
              onClick={() => setShowComparison(true)}
              className="px-3 py-1 rounded-lg text-xs font-medium bg-background-success-muted hover:bg-background-success-muted text-text-default transition-colors"
            >
              Compare Selected →
            </button>
          )}
          {compareMode && selectedForCompare.length < 2 && (
            <span className="text-xs text-text-muted">
              Select {2 - selectedForCompare.length} more run
              {selectedForCompare.length === 0 ? 's' : ''}
            </span>
          )}
        </div>
        <div className="flex gap-3 items-center">
          <select
            value={selectedDataset}
            onChange={(e) => setSelectedDataset(e.target.value)}
            className="bg-background-muted border border-border-default rounded-lg px-3 py-2 text-text-default text-sm focus:outline-none focus:border-border-accent"
          >
            <option value="">Select dataset...</option>
            {datasets.map((ds) => (
              <option key={ds.id} value={ds.id}>
                {ds.name} ({ds.caseCount} cases)
              </option>
            ))}
          </select>
          <button type="button"
            onClick={handleRunEval}
            disabled={running || !selectedDataset}
            className="px-4 py-2 rounded-lg bg-background-accent hover:bg-background-accent disabled:bg-background-muted disabled:cursor-not-allowed text-text-on-accent text-sm font-medium transition-colors"
          >
            {running ? 'Running...' : '▶ Run Eval'}
          </button>
        </div>
      </div>

      {loading ? (
        <div className="space-y-3 animate-pulse">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={`run-skeleton-${i + 1}`} className="h-14 rounded-lg bg-background-muted" />
          ))}
        </div>
      ) : runs.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-48 text-text-muted">
          <p className="text-lg mb-2">No evaluation runs yet</p>
          <p className="text-sm">Select a dataset and run an eval to see results here</p>
        </div>
      ) : (
        <div className="rounded-lg border border-border-default overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="bg-background-muted">
                {compareMode && <th className="w-10 px-2 py-3" />}
                <th className="text-left px-4 py-3 text-text-muted font-medium">Run</th>
                <th className="text-left px-4 py-3 text-text-muted font-medium">Dataset</th>
                <th className="text-center px-4 py-3 text-text-muted font-medium">Overall</th>
                <th className="text-center px-4 py-3 text-text-muted font-medium">Agent</th>
                <th className="text-center px-4 py-3 text-text-muted font-medium">Mode</th>
                <th className="text-left px-4 py-3 text-text-muted font-medium">Version</th>
                <th className="text-left px-4 py-3 text-text-muted font-medium">Date</th>
                <th className="w-20" />
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => {
                const isSelected = selectedForCompare.includes(run.id);
                const selIndex = selectedForCompare.indexOf(run.id);
                return (
                  <tr
                    key={run.id}
                    className={`border-t border-border-muted hover:bg-background-muted cursor-pointer ${
                      isSelected ? 'bg-purple-900/20 border-purple-500/30' : ''
                    }`}
                    onClick={() =>
                      compareMode ? toggleRunSelection(run.id) : handleViewDetail(run.id)
                    }
                  >
                    {compareMode && (
                      <td className="px-2 py-3 text-center">
                        <div
                          className={`w-5 h-5 rounded border-2 flex items-center justify-center text-xs font-bold ${
                            isSelected
                              ? 'border-purple-400 bg-purple-600 text-text-default'
                              : 'border-border-muted bg-background-default'
                          }`}
                        >
                          {isSelected ? (selIndex === 0 ? 'A' : 'B') : ''}
                        </div>
                      </td>
                    )}
                    <td className="px-4 py-3 text-text-default font-mono text-xs">
                      {run.id.slice(0, 8)}
                    </td>
                    <td className="px-4 py-3 text-text-default">{run.datasetName}</td>
                    <td className="px-4 py-3 text-center">
                      <AccuracyBadge value={run.overallAccuracy} />
                    </td>
                    <td className="px-4 py-3 text-center">
                      <AccuracyBadge value={run.agentAccuracy} />
                    </td>
                    <td className="px-4 py-3 text-center">
                      <AccuracyBadge value={run.modeAccuracy} />
                    </td>
                    <td className="px-4 py-3 text-xs text-text-muted">
                      {run.versionTag || run.gooseVersion || '-'}
                    </td>
                    <td className="px-4 py-3 text-text-muted text-xs">
                      {formatDate(run.startedAt)}
                    </td>
                    <td className="px-4 py-3 text-right">
                      {!compareMode && <span className="text-text-accent text-xs">Details →</span>}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
