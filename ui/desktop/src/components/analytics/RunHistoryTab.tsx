import { useEffect, useState, useCallback } from "react";
import {
  listEvalRuns,
  getEvalRun,
  listEvalDatasets,
  runEval,
} from "../../api";
import type {
  EvalRunSummary,
  EvalRunDetail,
  EvalDatasetSummary,
} from "../../api";
import RunComparisonView from "./RunComparisonView";

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function AccuracyBadge({ value }: { value: number }) {
  const color =
    value >= 0.9
      ? "bg-green-900/40 text-green-300 border-green-500/30"
      : value >= 0.7
        ? "bg-amber-900/40 text-amber-300 border-amber-500/30"
        : "bg-red-900/40 text-red-300 border-red-500/30";
  return (
    <span className={`text-xs px-2 py-0.5 rounded-full border ${color}`}>
      {formatPercent(value)}
    </span>
  );
}

function RunDetailPanel({
  detail,
  onClose,
}: {
  detail: EvalRunDetail;
  onClose: () => void;
}) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <button onClick={onClose} className="text-sm text-blue-400 hover:text-blue-300 mb-2">
            ← Back to runs
          </button>
          <h3 className="text-lg font-semibold text-white">Run #{detail.id.slice(0, 8)}</h3>
          <div className="flex gap-4 mt-1 text-xs text-gray-400">
            <span>Dataset: {detail.datasetName}</span>
            <span>{formatDate(detail.startedAt)}</span>
            {detail.gooseVersion && <span>Goose: {detail.gooseVersion}</span>}
          </div>
        </div>
        <div className="flex gap-3 text-sm">
          <div className="text-center">
            <div className="text-gray-400 text-xs">Overall</div>
            <AccuracyBadge value={detail.overallAccuracy} />
          </div>
          <div className="text-center">
            <div className="text-gray-400 text-xs">Agent</div>
            <AccuracyBadge value={detail.agentAccuracy} />
          </div>
          <div className="text-center">
            <div className="text-gray-400 text-xs">Mode</div>
            <AccuracyBadge value={detail.modeAccuracy} />
          </div>
        </div>
      </div>

      {/* Per-Agent Results */}
      {detail.perAgent.length > 0 && (
        <div className="rounded-lg border border-gray-600/40 bg-gray-800/50 p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Per-Agent Results</h4>
          <div className="grid grid-cols-2 gap-3">
            {detail.perAgent.map((ar) => (
              <div key={ar.agent} className="rounded-lg border border-gray-700/50 bg-gray-900/30 p-3">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-medium text-white">{ar.agent}</span>
                  <AccuracyBadge value={ar.accuracy} />
                </div>
                <div className="flex gap-4 text-xs text-gray-400">
                  <span>Pass: <span className="text-green-400">{ar.pass}</span></span>
                  <span>Fail: <span className="text-red-400">{ar.fail}</span></span>
                </div>
                <div className="mt-2 h-1.5 bg-gray-700 rounded-full overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all"
                    style={{
                      width: `${ar.accuracy * 100}%`,
                      backgroundColor: ar.accuracy >= 0.9 ? "#22c55e" : ar.accuracy >= 0.7 ? "#f59e0b" : "#ef4444",
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
        <div className="rounded-lg border border-red-600/30 bg-red-900/10 p-4">
          <h4 className="text-sm font-medium text-red-300 mb-3">
            Failed Cases ({detail.failures.length})
          </h4>
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {detail.failures.map((f, i) => (
              <div key={i} className="rounded bg-gray-800/80 border border-gray-700/50 p-3">
                <p className="text-sm text-white mb-2 line-clamp-2">&quot;{f.input}&quot;</p>
                <div className="flex gap-6 text-xs">
                  <div>
                    <span className="text-gray-500">Expected: </span>
                    <span className="text-green-400">{f.expectedAgent} → {f.expectedMode}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">Got: </span>
                    <span className="text-red-400">{f.actualAgent} → {f.actualMode}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">Confidence: </span>
                    <span className="text-gray-300">{(f.confidence * 100).toFixed(0)}%</span>
                  </div>
                </div>
                {f.tags.length > 0 && (
                  <div className="flex gap-1 mt-2">
                    {f.tags.map((t) => (
                      <span key={t} className="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 text-gray-400">
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

      {/* Confusion Matrix */}
      {detail.confusionMatrix.labels.length > 0 && (
        <div className="rounded-lg border border-gray-600/40 bg-gray-800/50 p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Confusion Matrix</h4>
          <div className="overflow-x-auto">
            <table className="text-xs">
              <thead>
                <tr>
                  <th className="px-3 py-2 text-left text-gray-500">Expected ↓ / Actual →</th>
                  {detail.confusionMatrix.labels.map((l) => (
                    <th key={l} className="px-3 py-2 text-center text-gray-400">{l}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {detail.confusionMatrix.labels.map((expected, ri) => (
                  <tr key={expected} className="border-t border-gray-700/50">
                    <td className="px-3 py-2 text-gray-400 font-medium">{expected}</td>
                    {detail.confusionMatrix.matrix[ri]?.map((count, ci) => {
                      const isDiagonal = ri === ci;
                      return (
                        <td
                          key={ci}
                          className={`px-3 py-2 text-center ${
                            count > 0
                              ? isDiagonal
                                ? "bg-green-900/30 text-green-300"
                                : "bg-red-900/30 text-red-300"
                              : "text-gray-600"
                          }`}
                        >
                          {count || "-"}
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

export default function RunHistoryTab() {
  const [runs, setRuns] = useState<EvalRunSummary[]>([]);
  const [datasets, setDatasets] = useState<EvalDatasetSummary[]>([]);
  const [selectedDetail, setSelectedDetail] = useState<EvalRunDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [selectedDataset, setSelectedDataset] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [compareMode, setCompareMode] = useState(false);
  const [selectedForCompare, setSelectedForCompare] = useState<string[]>([]);
  const [showComparison, setShowComparison] = useState(false);

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
      setError(e instanceof Error ? e.message : "Failed to load runs");
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
      setError(e instanceof Error ? e.message : "Eval run failed");
    } finally {
      setRunning(false);
    }
  };

  const handleViewDetail = async (runId: string) => {
    try {
      const res = await getEvalRun({ path: { id: runId } });
      if (res.data) setSelectedDetail(res.data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load run detail");
    }
  };

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
        <div className="rounded-lg bg-red-900/30 border border-red-500/40 p-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-semibold text-white">Run History</h3>
          {runs.length >= 2 && (
            <button
              onClick={() => {
                setCompareMode(!compareMode);
                if (compareMode) setSelectedForCompare([]);
              }}
              className={`px-3 py-1 rounded-lg text-xs font-medium transition-colors ${
                compareMode
                  ? "bg-purple-600 text-white"
                  : "bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
              }`}
            >
              {compareMode ? "Cancel Compare" : "⇆ Compare Runs"}
            </button>
          )}
          {compareMode && selectedForCompare.length === 2 && (
            <button
              onClick={() => setShowComparison(true)}
              className="px-3 py-1 rounded-lg text-xs font-medium bg-green-600 hover:bg-green-700 text-white transition-colors"
            >
              Compare Selected →
            </button>
          )}
          {compareMode && selectedForCompare.length < 2 && (
            <span className="text-xs text-zinc-400">
              Select {2 - selectedForCompare.length} more run{selectedForCompare.length === 0 ? "s" : ""}
            </span>
          )}
        </div>
        <div className="flex gap-3 items-center">
          <select
            value={selectedDataset}
            onChange={(e) => setSelectedDataset(e.target.value)}
            className="bg-gray-700/50 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500"
          >
            <option value="">Select dataset...</option>
            {datasets.map((ds) => (
              <option key={ds.id} value={ds.id}>
                {ds.name} ({ds.caseCount} cases)
              </option>
            ))}
          </select>
          <button
            onClick={handleRunEval}
            disabled={running || !selectedDataset}
            className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors"
          >
            {running ? "Running..." : "▶ Run Eval"}
          </button>
        </div>
      </div>

      {loading ? (
        <div className="space-y-3 animate-pulse">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-14 rounded-lg bg-gray-700/50" />
          ))}
        </div>
      ) : runs.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-48 text-gray-400">
          <p className="text-lg mb-2">No evaluation runs yet</p>
          <p className="text-sm">Select a dataset and run an eval to see results here</p>
        </div>
      ) : (
        <div className="rounded-lg border border-gray-600/40 overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="bg-gray-700/50">
                {compareMode && (
                  <th className="w-10 px-2 py-3" />
                )}
                <th className="text-left px-4 py-3 text-gray-400 font-medium">Run</th>
                <th className="text-left px-4 py-3 text-gray-400 font-medium">Dataset</th>
                <th className="text-center px-4 py-3 text-gray-400 font-medium">Overall</th>
                <th className="text-center px-4 py-3 text-gray-400 font-medium">Agent</th>
                <th className="text-center px-4 py-3 text-gray-400 font-medium">Mode</th>
                <th className="text-left px-4 py-3 text-gray-400 font-medium">Version</th>
                <th className="text-left px-4 py-3 text-gray-400 font-medium">Date</th>
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
                  className={`border-t border-gray-700/50 hover:bg-gray-800/50 cursor-pointer ${
                    isSelected ? "bg-purple-900/20 border-purple-500/30" : ""
                  }`}
                  onClick={() => compareMode ? toggleRunSelection(run.id) : handleViewDetail(run.id)}
                >
                  {compareMode && (
                    <td className="px-2 py-3 text-center">
                      <div className={`w-5 h-5 rounded border-2 flex items-center justify-center text-xs font-bold ${
                        isSelected
                          ? "border-purple-400 bg-purple-600 text-white"
                          : "border-zinc-500 bg-zinc-800"
                      }`}>
                        {isSelected ? (selIndex === 0 ? "A" : "B") : ""}
                      </div>
                    </td>
                  )}
                  <td className="px-4 py-3 text-white font-mono text-xs">{run.id.slice(0, 8)}</td>
                  <td className="px-4 py-3 text-gray-300">{run.datasetName}</td>
                  <td className="px-4 py-3 text-center"><AccuracyBadge value={run.overallAccuracy} /></td>
                  <td className="px-4 py-3 text-center"><AccuracyBadge value={run.agentAccuracy} /></td>
                  <td className="px-4 py-3 text-center"><AccuracyBadge value={run.modeAccuracy} /></td>
                  <td className="px-4 py-3 text-xs text-gray-400">
                    {run.versionTag || run.gooseVersion || "-"}
                  </td>
                  <td className="px-4 py-3 text-gray-400 text-xs">{formatDate(run.startedAt)}</td>
                  <td className="px-4 py-3 text-right">
                    {!compareMode && <span className="text-blue-400 text-xs">Details →</span>}
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
