import { useCallback, useEffect, useState } from 'react';
import type { CreateDatasetRequest, EvalDataset, EvalDatasetSummary } from '@/api';
import {
  createEvalDataset,
  deleteEvalDataset,
  getEvalDataset,
  listEvalDatasets,
  updateEvalDataset,
} from '@/api';

interface TestCaseRow {
  id: string;
  input: string;
  expectedAgent: string;
  expectedMode: string;
  tags: string;
}

function newRowId(): string {
  return globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2);
}

function emptyRow(): TestCaseRow {
  return { id: newRowId(), input: '', expectedAgent: '', expectedMode: '', tags: '' };
}

function DatasetEditor({
  dataset,
  onSave,
  onCancel,
}: {
  dataset: EvalDataset | null;
  onSave: (req: CreateDatasetRequest) => Promise<void>;
  onCancel: () => void;
}) {
  const [name, setName] = useState(dataset?.name ?? '');
  const [description, setDescription] = useState(dataset?.description ?? '');
  const [rows, setRows] = useState<TestCaseRow[]>(() => {
    if (dataset?.cases && dataset.cases.length > 0) {
      return dataset.cases.map((tc) => ({
        id: tc.id,
        input: tc.input,
        expectedAgent: tc.expectedAgent,
        expectedMode: tc.expectedMode,
        tags: (tc.tags ?? []).join(', '),
      }));
    }
    return [emptyRow()];
  });
  const [saving, setSaving] = useState(false);
  const [yamlMode, setYamlMode] = useState(false);
  const [yamlText, setYamlText] = useState('');

  const addRow = () => setRows([...rows, emptyRow()]);

  const removeRow = (idx: number) => {
    if (rows.length <= 1) return;
    setRows(rows.filter((_, i) => i !== idx));
  };

  const updateRow = (idx: number, field: keyof TestCaseRow, value: string) => {
    const updated = [...rows];
    updated[idx] = { ...updated[idx], [field]: value };
    setRows(updated);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const cases = rows
        .filter((r) => r.input.trim())
        .map((r) => ({
          input: r.input.trim(),
          expectedAgent: r.expectedAgent.trim(),
          expectedMode: r.expectedMode.trim(),
          tags: r.tags
            .split(',')
            .map((t) => t.trim())
            .filter(Boolean),
        }));
      await onSave({ name, description, cases });
    } finally {
      setSaving(false);
    }
  };

  const generateYaml = () => {
    const cases = rows
      .filter((r) => r.input.trim())
      .map((r) => {
        let yaml = `  - input: "${r.input.replace(/"/g, '\\"')}"\n`;
        yaml += `    expected_agent: "${r.expectedAgent}"\n`;
        yaml += `    expected_mode: "${r.expectedMode}"`;
        const tags = r.tags
          .split(',')
          .map((t) => t.trim())
          .filter(Boolean);
        if (tags.length > 0) {
          yaml += `\n    tags: [${tags.map((t) => `"${t}"`).join(', ')}]`;
        }
        return yaml;
      });
    setYamlText(`name: "${name}"\ndescription: "${description}"\ntest_cases:\n${cases.join('\n')}`);
    setYamlMode(true);
  };

  const parseYaml = () => {
    try {
      const lines = yamlText.split('\n');
      const parsed: TestCaseRow[] = [];
      let current: TestCaseRow | null = null;

      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed.startsWith('- input:')) {
          if (current) parsed.push(current);
          current = { ...emptyRow(), input: trimmed.replace(/^- input:\s*"?(.*?)"?$/, '$1') };
        } else if (current && trimmed.startsWith('expected_agent:')) {
          current.expectedAgent = trimmed.replace(/^expected_agent:\s*"?(.*?)"?$/, '$1');
        } else if (current && trimmed.startsWith('expected_mode:')) {
          current.expectedMode = trimmed.replace(/^expected_mode:\s*"?(.*?)"?$/, '$1');
        } else if (current && trimmed.startsWith('tags:')) {
          const match = trimmed.match(/\[(.*)\]/);
          if (match) current.tags = match[1].replace(/"/g, '');
        } else if (trimmed.startsWith('name:')) {
          setName(trimmed.replace(/^name:\s*"?(.*?)"?$/, '$1'));
        } else if (trimmed.startsWith('description:')) {
          setDescription(trimmed.replace(/^description:\s*"?(.*?)"?$/, '$1'));
        }
      }
      if (current) parsed.push(current);
      if (parsed.length > 0) setRows(parsed);
      setYamlMode(false);
    } catch {
      // Stay in YAML mode on parse error
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex gap-4">
        <div className="flex-1">
          <label htmlFor="dataset-name" className="text-xs text-text-muted block mb-1">
            Dataset Name
          </label>
          <input
            id="dataset-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g., Routing accuracy v2"
            className="w-full bg-background-muted border border-border-default rounded-lg px-3 py-2 text-text-default text-sm focus:outline-none focus:border-border-accent"
          />
        </div>
        <div className="flex-1">
          <label htmlFor="dataset-description" className="text-xs text-text-muted block mb-1">
            Description
          </label>
          <input
            id="dataset-description"
            type="text"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Brief description..."
            className="w-full bg-background-muted border border-border-default rounded-lg px-3 py-2 text-text-default text-sm focus:outline-none focus:border-border-accent"
          />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button type="button"
          onClick={() => (yamlMode ? parseYaml() : generateYaml())}
          className="text-xs text-text-accent hover:text-text-accent underline"
        >
          {yamlMode ? '← Back to table' : 'Edit as YAML'}
        </button>
      </div>

      {yamlMode ? (
        <textarea
          value={yamlText}
          onChange={(e) => setYamlText(e.target.value)}
          rows={20}
          className="w-full bg-background-default border border-border-default rounded-lg px-3 py-2 text-text-success text-xs font-mono focus:outline-none focus:border-border-accent"
        />
      ) : (
        <>
          <div className="rounded-lg border border-border-default overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-background-muted">
                  <th className="text-left px-3 py-2 text-text-muted font-medium w-8">#</th>
                  <th className="text-left px-3 py-2 text-text-muted font-medium">Input Prompt</th>
                  <th className="text-left px-3 py-2 text-text-muted font-medium w-40">
                    Expected Agent
                  </th>
                  <th className="text-left px-3 py-2 text-text-muted font-medium w-40">
                    Expected Mode
                  </th>
                  <th className="text-left px-3 py-2 text-text-muted font-medium w-36">Tags</th>
                  <th className="w-10" />
                </tr>
              </thead>
              <tbody>
                {rows.map((row, i) => (
                  <tr key={row.id} className="border-t border-border-muted">
                    <td className="px-3 py-1.5 text-text-muted text-xs">{i + 1}</td>
                    <td className="px-1 py-1.5">
                      <input
                        type="text"
                        value={row.input}
                        onChange={(e) => updateRow(i, 'input', e.target.value)}
                        placeholder="User message..."
                        className="w-full bg-transparent border-none text-text-default text-sm focus:outline-none placeholder:text-text-subtle"
                      />
                    </td>
                    <td className="px-1 py-1.5">
                      <input
                        type="text"
                        value={row.expectedAgent}
                        onChange={(e) => updateRow(i, 'expectedAgent', e.target.value)}
                        placeholder="agent"
                        className="w-full bg-transparent border-none text-text-default text-sm focus:outline-none placeholder:text-text-subtle"
                      />
                    </td>
                    <td className="px-1 py-1.5">
                      <input
                        type="text"
                        value={row.expectedMode}
                        onChange={(e) => updateRow(i, 'expectedMode', e.target.value)}
                        placeholder="mode"
                        className="w-full bg-transparent border-none text-text-default text-sm focus:outline-none placeholder:text-text-subtle"
                      />
                    </td>
                    <td className="px-1 py-1.5">
                      <input
                        type="text"
                        value={row.tags}
                        onChange={(e) => updateRow(i, 'tags', e.target.value)}
                        placeholder="tag1, tag2"
                        className="w-full bg-transparent border-none text-text-default text-sm focus:outline-none placeholder:text-text-subtle"
                      />
                    </td>
                    <td className="px-1 py-1.5">
                      <button type="button"
                        onClick={() => removeRow(i)}
                        className="text-text-muted hover:text-text-danger text-sm"
                        disabled={rows.length <= 1}
                      >
                        ✕
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button type="button"
            onClick={addRow}
            className="text-sm text-text-accent hover:text-text-accent flex items-center gap-1"
          >
            + Add test case
          </button>
        </>
      )}

      <div className="flex gap-3 justify-end pt-2 border-t border-border-default">
        <button type="button"
          onClick={onCancel}
          className="px-4 py-2 rounded-lg border border-border-default text-text-default text-sm hover:bg-background-muted transition-colors"
        >
          Cancel
        </button>
        <button type="button"
          onClick={handleSave}
          disabled={saving || !name.trim()}
          className="px-4 py-2 rounded-lg bg-background-accent hover:bg-background-accent disabled:bg-background-muted disabled:cursor-not-allowed text-text-on-accent text-sm font-medium transition-colors"
        >
          {saving ? 'Saving...' : dataset ? 'Update Dataset' : 'Create Dataset'}
        </button>
      </div>
    </div>
  );
}

export default function DatasetsTab() {
  const [datasets, setDatasets] = useState<EvalDatasetSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<EvalDataset | null | 'new'>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchDatasets = useCallback(async () => {
    try {
      setLoading(true);
      const res = await listEvalDatasets();
      if (res.data) setDatasets(res.data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load datasets');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchDatasets();
  }, [fetchDatasets]);

  const handleEdit = async (id: string) => {
    try {
      const res = await getEvalDataset({ path: { id } });
      if (res.data) setEditing(res.data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load dataset');
    }
  };

  const handleDelete = async (id: string) => {
    if (!window.confirm('Delete this dataset? This cannot be undone.')) return;
    try {
      await deleteEvalDataset({ path: { id } });
      await fetchDatasets();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete dataset');
    }
  };

  const handleSave = async (req: CreateDatasetRequest) => {
    if (editing && editing !== 'new' && 'id' in editing) {
      await updateEvalDataset({ path: { id: editing.id }, body: req });
    } else {
      await createEvalDataset({ body: req });
    }
    setEditing(null);
    await fetchDatasets();
  };

  if (editing) {
    return (
      <DatasetEditor
        dataset={editing === 'new' ? null : editing}
        onSave={handleSave}
        onCancel={() => setEditing(null)}
      />
    );
  }

  return (
    <div className="space-y-4">
      {error && (
        <div className="rounded-lg bg-background-danger-muted border border-border-default p-3 text-text-danger text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-text-default">Evaluation Datasets</h3>
        <button type="button"
          onClick={() => setEditing('new')}
          className="px-4 py-2 rounded-lg bg-background-accent hover:bg-background-accent text-text-on-accent text-sm font-medium transition-colors"
        >
          + New Dataset
        </button>
      </div>

      {loading ? (
        <div className="space-y-3 animate-pulse">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={`dataset-skeleton-${i + 1}`} className="h-16 rounded-lg bg-background-muted" />
          ))}
        </div>
      ) : datasets.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-48 text-text-muted">
          <p className="text-lg mb-2">No datasets yet</p>
          <p className="text-sm mb-4">
            Create a dataset to define test cases for evaluating routing accuracy
          </p>
          <button type="button"
            onClick={() => setEditing('new')}
            className="px-4 py-2 rounded-lg bg-background-accent hover:bg-background-accent text-text-on-accent text-sm font-medium transition-colors"
          >
            Create First Dataset
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {datasets.map((ds) => (
            <div
              key={ds.id}
              className="rounded-lg border border-border-default bg-background-muted p-4 flex items-center justify-between hover:border-border-default transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-3">
                  <h4 className="text-text-default font-medium">{ds.name}</h4>
                  <span className="text-xs bg-background-muted text-text-default px-2 py-0.5 rounded-full">
                    {ds.caseCount} cases
                  </span>
                  {ds.lastRunAccuracy != null && (
                    <span
                      className={`text-xs px-2 py-0.5 rounded-full border ${
                        ds.lastRunAccuracy >= 0.9
                          ? 'bg-background-success-muted text-text-success border-border-default'
                          : ds.lastRunAccuracy >= 0.7
                            ? 'bg-background-warning-muted text-text-warning border-border-default'
                            : 'bg-background-danger-muted text-text-danger border-border-default'
                      }`}
                    >
                      Last: {(ds.lastRunAccuracy * 100).toFixed(1)}%
                    </span>
                  )}
                </div>
                {ds.description && (
                  <p className="text-sm text-text-muted mt-1 truncate">{ds.description}</p>
                )}
                <p className="text-xs text-text-muted mt-1">
                  Updated {new Date(ds.updatedAt).toLocaleDateString()}
                </p>
              </div>
              <div className="flex gap-2 ml-4">
                <button type="button"
                  onClick={() => handleEdit(ds.id)}
                  className="px-3 py-1.5 rounded border border-border-default text-text-default text-xs hover:bg-background-muted transition-colors"
                >
                  Edit
                </button>
                <button type="button"
                  onClick={() => handleDelete(ds.id)}
                  className="px-3 py-1.5 rounded border border-border-default bg-background-default text-text-danger text-xs hover:bg-background-danger-muted transition-colors"
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
