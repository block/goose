import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Eye, Pencil, Trash2, Filter, Search, Tag, RefreshCw, ChevronLeft, ChevronRight, ChevronDown, X, Upload, Link2 as LinkIcon, Layers } from 'lucide-react';

type Message = {
  role: 'user' | 'assistant' | 'system';
  content?: string;
};

export interface TrainingExampleSummary {
  id: string;
  conversation_id: string;
  created_at: string;
  quality_score: number;
  message_count: number;
  domain_tags: string[];
  provider_used: string;
  model_used: string;
}

export default function TrainingDataTable({ backendUrl, secretKey }: { backendUrl: string; secretKey: string }) {
  const [items, setItems] = useState<TrainingExampleSummary[]>([]);
  const [count, setCount] = useState(0);
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(25);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // grouping state
  const [groupByDataset, setGroupByDataset] = useState(true);
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({});

  // filters
  const [search, setSearch] = useState('');
  const [minScore, setMinScore] = useState<number | ''>('');
  // derive grouping by dataset tag
  const groupedBy = useMemo(() => {
    if (!groupByDataset) return null as Record<string, TrainingExampleSummary[]> | null;
    const g: Record<string, TrainingExampleSummary[]> = {};
    for (const it of items) {
      const dsTag = (it.domain_tags || []).find((t) => typeof t === 'string' && t.startsWith('dataset:'));
      const key = dsTag ? dsTag.slice('dataset:'.length) : 'ungrouped';
      if (!g[key]) g[key] = [];
      g[key].push(it);
    }
    return g;
  }, [items, groupByDataset]);

  // ensure new groups default to expanded
  useEffect(() => {
    if (!groupByDataset || !groupedBy) return;
    setExpandedGroups((prev) => {
      const next = { ...prev } as Record<string, boolean>;
      for (const k of Object.keys(groupedBy)) {
        if (next[k] === undefined) next[k] = true;
      }
      return next;
    });
  }, [groupByDataset, groupedBy]);
  // per-group pagination state/cache
  const [groupPerPage, setGroupPerPage] = useState(10);
  const [groupPages, setGroupPages] = useState<Record<string, number>>({});
  const [groupCounts, setGroupCounts] = useState<Record<string, number>>({});
  const [groupItems, setGroupItems] = useState<Record<string, TrainingExampleSummary[]>>({});
  const [groupLoading, setGroupLoading] = useState<Record<string, boolean>>({});
  // grouped mode controls
  const [groupFetchSize, setGroupFetchSize] = useState(1000);
  const [groupsVisibleCount, setGroupsVisibleCount] = useState(10);
  const [groupPerGroupCap, setGroupPerGroupCap] = useState(300);
  const [groupSamplingNextPage, setGroupSamplingNextPage] = useState(1);
  const [groupSamplingExhausted, setGroupSamplingExhausted] = useState(false);
  const GROUPS_STEP = 10;


  const renderRow = useCallback((row: TrainingExampleSummary, onView: (id: string) => void, onEdit: (id: string) => void, onDelete: (id: string) => void) => (
    <tr key={row.id} className="border-t border-border-default hover:bg-background-subtle">
      <td className="p-2 whitespace-nowrap">{new Date(row.created_at).toLocaleString()}</td>
      <td className="p-2">{row.conversation_id}</td>
      <td className="p-2">{row.quality_score.toFixed(2)}</td>
      <td className="p-2">
        <div className="flex flex-wrap gap-1">
          {row.domain_tags?.length ? row.domain_tags.map((t) => (
            <span key={t} className="px-2 py-0.5 rounded bg-background-muted text-text-muted text-xs">{t}</span>
          )) : <span className="text-text-muted text-xs">—</span>}
        </div>
      </td>
      <td className="p-2">{row.message_count}</td>
      <td className="p-2">
        <div className="flex flex-col">
          <span className="text-xs text-text-muted">{row.provider_used}</span>
          <span className="text-xs text-text-default">{row.model_used}</span>
        </div>
      </td>
      <td className="p-2">
        <div className="flex items-center gap-2 justify-end">
          <button className="p-1 hover:bg-background-muted rounded" title="View" onClick={() => onView(row.id)}>
            <Eye className="w-4 h-4" />
          </button>
          <button className="p-1 hover:bg-background-muted rounded" title="Edit" onClick={() => onEdit(row.id)}>
            <Pencil className="w-4 h-4" />
          </button>
          <button className="p-1 hover:bg-red-50 rounded" title="Delete" onClick={() => onDelete(row.id)}>
            <Trash2 className="w-4 h-4 text-red-500" />
          </button>
        </div>
      </td>
    </tr>
  ), []);
  // compute per-group slices and counts
  useEffect(() => {
    if (!groupByDataset || !groupedBy) {
      setGroupItems({});
      setGroupCounts({});
      return;
    }
    const nextCounts: Record<string, number> = {};
    const nextItems: Record<string, TrainingExampleSummary[]> = {};
    for (const [k, rows] of Object.entries(groupedBy)) {
      nextCounts[k] = rows.length;
      const page = groupPages[k] ?? 1;
      const start = (page - 1) * groupPerPage;
      const end = start + groupPerPage;
      nextItems[k] = rows.slice(start, end);
    }
    setGroupCounts(nextCounts);
    setGroupItems(nextItems);
  }, [groupByDataset, groupedBy, groupPages, groupPerPage]);

  const [tags, setTags] = useState(''); // comma separated

  // modals
  const [viewId, setViewId] = useState<string | null>(null);
  const [editId, setEditId] = useState<string | null>(null);
  const [deleteId, setDeleteId] = useState<string | null>(null);

  // view payload
  const [viewData, setViewData] = useState<any | null>(null);
  const [viewLoading, setViewLoading] = useState(false);

  // edit payload fields
  const [editTags, setEditTags] = useState('');
  const [editQuality, setEditQuality] = useState<string>('');
  const [editCorrection, setEditCorrection] = useState('');
  const [editComments, setEditComments] = useState('');
  const [editSaving, setEditSaving] = useState(false);

  const query = useMemo(() => {
    const params = new URLSearchParams();
    // When grouping, fetch a larger slice once so multiple groups can appear on the same view
    const effectivePage = groupByDataset ? 1 : page;
    const effectivePerPage = groupByDataset ? groupFetchSize : perPage; // adjust if needed
    params.set('page', String(effectivePage));
    params.set('per_page', String(effectivePerPage));
    if (search.trim()) params.set('search', search.trim());
    if (minScore !== '' && !isNaN(Number(minScore))) params.set('min_quality_score', String(minScore));
    if (tags.trim()) params.set('tags', tags.trim());
    return params.toString();
  }, [page, perPage, search, minScore, tags, groupByDataset, groupFetchSize]);

  const fetchList = useCallback(async () => {
    if (!backendUrl || !secretKey) return;
    setLoading(true);
    setError(null);
    try {
      if (groupByDataset) {
        // Multi-page sampling to surface multiple groups even if one dominates early pages
        const baseParams = new URLSearchParams();
        if (search.trim()) baseParams.set('search', search.trim());
        if (minScore !== '' && !isNaN(Number(minScore))) baseParams.set('min_quality_score', String(minScore));
        if (tags.trim()) baseParams.set('tags', tags.trim());

        const targetGroups = groupsVisibleCount;
        const perPageEff = groupFetchSize; // items per page to sample
        const maxPages = 10; // hard cap to avoid excessive requests

        const collected: TrainingExampleSummary[] = [];
        const seenIds = new Set<string>();
        const seenGroups = new Set<string>();
        let totalCount = 0;
        for (let p = 1; p <= maxPages; p++) {
          const params = new URLSearchParams(baseParams);
          params.set('page', String(p));
          params.set('per_page', String(perPageEff));
          const res = await fetch(`${backendUrl}/training/examples/list?${params.toString()}`, {
            headers: { 'X-Secret-Key': secretKey },
          });
          if (!res.ok) throw new Error(`${res.status}`);
          const data = await res.json();
          if (p === 1) totalCount = data.count || 0;
          const pageItems: TrainingExampleSummary[] = data.examples || [];
          for (const it of pageItems) {
            if (!seenIds.has(it.id)) {
              collected.push(it);
              seenIds.add(it.id);
              const ds = (it.domain_tags || []).find(t => typeof t === 'string' && t.startsWith('dataset:')) || 'dataset:ungrouped';
              seenGroups.add(ds);
            }
          }
          // Stop if we have enough distinct groups or this page returned fewer items than perPageEff
          if (seenGroups.size >= targetGroups || pageItems.length < perPageEff) break;
        }
        setItems(collected);
        setCount(totalCount);
      } else {
        const res = await fetch(`${backendUrl}/training/examples/list?${query}`, {
          headers: { 'X-Secret-Key': secretKey },
        });
        if (!res.ok) throw new Error(`${res.status}`);
        const data = await res.json();
        setItems(data.examples || []);
        setCount(data.count || 0);
      }
    } catch (e) {
      setError(`Failed to load training data: ${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  }, [backendUrl, secretKey, query, groupByDataset, groupFetchSize, groupsVisibleCount, search, minScore, tags]);

  useEffect(() => { fetchList(); }, [fetchList]);

  const totalPages = Math.max(1, Math.ceil(count / perPage));

  const openView = useCallback(async (id: string) => {
    setViewId(id);
    setViewData(null);
    setViewLoading(true);
    try {
      const res = await fetch(`${backendUrl}/training/examples/${id}`, {
        headers: { 'X-Secret-Key': secretKey },
      });
      if (!res.ok) throw new Error(`${res.status}`);
      const data = await res.json();
      setViewData(data.example);
    } catch (e) {
      setViewData({ error: (e as Error).message });
    } finally {
      setViewLoading(false);
    }
  }, [backendUrl, secretKey]);

  const openEdit = useCallback(async (id: string) => {
    setEditId(id);
    // Preload current details for convenience
    try {
      const res = await fetch(`${backendUrl}/training/examples/${id}`, {
        headers: { 'X-Secret-Key': secretKey },
      });
      if (res.ok) {
        const data = await res.json();
        const ex = data.example;
        setEditTags((ex.domain_tags || []).join(','));
        setEditQuality(String(ex.quality_metrics?.overall_score ?? ''));
        const corr = ex?.metadata?.custom_fields?.correction ?? '';
        const comm = ex?.metadata?.custom_fields?.comments ?? '';
        setEditCorrection(typeof corr === 'string' ? corr : JSON.stringify(corr));
        setEditComments(typeof comm === 'string' ? comm : JSON.stringify(comm));
      }
    } catch {}
  }, [backendUrl, secretKey]);

  const saveEdit = useCallback(async () => {
    if (!editId) return;
    setEditSaving(true);
    try {
      const body: any = {};
      body.domain_tags = editTags.split(',').map(t => t.trim()).filter(Boolean);
      if (editQuality && !isNaN(Number(editQuality))) body.overall_quality_score = Number(editQuality);
      if (editCorrection.trim()) body.correction = editCorrection.trim();
      if (editComments.trim()) body.comments = editComments.trim();

      const res = await fetch(`${backendUrl}/training/examples/${editId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'X-Secret-Key': secretKey },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error(`${res.status}`);
      setEditId(null);
      await fetchList();
    } catch (e) {
      alert(`Failed to save: ${(e as Error).message}`);
    } finally {
      setEditSaving(false);
    }
  }, [backendUrl, secretKey, editId, editTags, editQuality, editCorrection, editComments, fetchList]);

  const confirmDelete = useCallback(async () => {
    if (!deleteId) return;
    try {
      const res = await fetch(`${backendUrl}/training/examples/${deleteId}`, {
        method: 'DELETE',
        headers: { 'X-Secret-Key': secretKey },
      });
      if (!res.ok) throw new Error(`${res.status}`);
      setDeleteId(null);
      await fetchList();
    } catch (e) {
      alert(`Failed to delete: ${(e as Error).message}`);
    }
  }, [backendUrl, secretKey, deleteId, fetchList]);

  // Upload state and handlers
  const [importOpen, setImportOpen] = useState(false);
  const [importUrl, setImportUrl] = useState('');
  const importBtnRef = useRef<HTMLButtonElement | null>(null);
  const importPopoverRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadDone, setUploadDone] = useState(0);
  const [uploadTotal, setUploadTotal] = useState(0);
  const [uploadErrors, setUploadErrors] = useState<string[]>([]);
  const [uploadGroup, setUploadGroup] = useState('');

  // Import popover auto-dismiss like filters
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (
        importOpen &&
        importPopoverRef.current &&
        !importPopoverRef.current.contains(t) &&
        importBtnRef.current &&
        !importBtnRef.current.contains(t)
      ) {
        setImportOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setImportOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [importOpen]);

  const handleOpenFilePicker = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  // Filters popover state
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filterBtnRef = useRef<HTMLButtonElement | null>(null);
  const popoverRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (
        filtersOpen &&
        popoverRef.current &&
        !popoverRef.current.contains(t) &&
        filterBtnRef.current &&
        !filterBtnRef.current.contains(t)
      ) {
        setFiltersOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setFiltersOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [filtersOpen]);

  const submitTrainingExample = useCallback(async (payload: any) => {
    const res = await fetch(`${backendUrl}/training/submit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Secret-Key': secretKey },
      body: JSON.stringify(payload),
    });
    if (!res.ok) {
      const text = await res.text().catch(() => '');
      throw new Error(`${res.status} ${text}`);
    }
  }, [backendUrl, secretKey]);

  const handleFilesSelected = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    setUploading(true);
    setUploadDone(0);
    setUploadErrors([]);
    try {
      let total = 0;
      // First pass: count JSONL lines for a progress total
      for (const file of Array.from(files)) {
        const text = await file.text();
        const lines = text.split(/\r?\n/).filter(Boolean);
        total += lines.length;
      }
      setUploadTotal(total);

      // Second pass: process and submit
      for (const file of Array.from(files)) {
        const text = await file.text();
        const lines = text.split(/\r?\n/).filter(Boolean);
        const baseName = file.name.replace(/\.[^/.]+$/, '');
        const chosen = (uploadGroup || '').trim();
        const datasetName = chosen ? chosen : baseName;
        const datasetTag = `dataset:${datasetName}`;
        for (let i = 0; i < lines.length; i++) {
          const line = lines[i];
          try {
            const obj = JSON.parse(line);
            // Expecting at least messages array; fall back defaults

            const conversation_id = obj.conversation_id || `${file.name}#${i}`;
            const messages = Array.isArray(obj.messages) ? obj.messages : [];
            if (!Array.isArray(messages) || messages.length === 0) {
              throw new Error('missing messages[]');
            }
            // Normalize and ensure dataset tag
            let domainTags: string[] = [];
            if (Array.isArray(obj.domain_tags)) domainTags = obj.domain_tags.filter((t: any) => typeof t === 'string');
            else if (Array.isArray(obj.tags)) domainTags = obj.tags.filter((t: any) => typeof t === 'string');
            if (!domainTags.includes(datasetTag)) domainTags.push(datasetTag);

            const payload = {
              conversation_id,
              session_id: obj.session_id || null,
              messages: messages.map((m: any) => {
                const roleLc = (m?.role ?? '').toString().toLowerCase();
                const role = roleLc === 'assistant' ? 'assistant' : 'user';
                const contentRaw = m?.content ?? m?.text ?? m?.value ?? '';
                const content = typeof contentRaw === 'string' ? contentRaw : JSON.stringify(contentRaw);
                return { role, content: [{ type: 'text', text: content }] };
              }),
              provider_used: obj.provider_used || obj.provider || undefined,
              model_used: obj.model_used || obj.model || undefined,
              response_time: obj.response_time || undefined,
              rating: obj.rating || undefined,
              correction: obj.correction || undefined,
              comments: obj.comments || undefined,
              domain_tags: domainTags,
            };
            await submitTrainingExample(payload);
            setUploadDone((d) => d + 1);
          } catch (err: any) {
            setUploadErrors((errs) => [...errs, `${file.name}: ${err?.message || String(err)}`]);
            setUploadDone((d) => d + 1);
          }
        }
      }
      await fetchList();
    } finally {
      setUploading(false);
      // reset input so same file can be selected again
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  }, [fetchList, submitTrainingExample]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <h3 className="text-lg font-medium text-text-default">Training Data</h3>
        <div className="flex items-center gap-2 relative">
          <input
            ref={fileInputRef}
            type="file"
            accept=".jsonl"
            multiple
            onChange={handleFilesSelected}
            className="hidden"
          />
          <button
            onClick={fetchList}
            className="p-2 bg-background-subtle hover:bg-background-medium rounded-lg"
            disabled={loading}
            title="Refresh"
            aria-label="Refresh"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
          <button
            ref={filterBtnRef}
            onClick={() => setFiltersOpen((o) => !o)}
            className="p-2 bg-background-subtle hover:bg-background-medium rounded-lg"
            title="Filters"
            aria-label="Filters"
          >
            <Filter className="w-4 h-4" />
          </button>
          {filtersOpen && (
            <div ref={popoverRef} className="absolute right-0 top-full mt-2 w-80 p-3 rounded-lg border border-border-default bg-background-default shadow-lg z-50">
              <div className="space-y-3">
                <div>
                  <label className="text-xs text-text-muted">Search</label>
                  <div className="relative">
                    <Search className="w-4 h-4 absolute left-2 top-2.5 text-text-muted" />
                    <input
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                      placeholder="Conversation ID or message contains..."
                      className="w-full pl-8 pr-3 py-2 rounded-md border border-border-default bg-background-default"
                    />
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="flex-1">
                    <label className="text-xs text-text-muted">Min quality</label>
                    <input
                      type="number"
                      min={0}
                      max={1}
                      step={0.1}
                      value={minScore}
                      onChange={(e) => setMinScore(e.target.value === '' ? '' : Number(e.target.value))}
                      className="w-full px-2 py-2 rounded-md border border-border-default bg-background-default"
                    />
                  </div>
                </div>
                <div>
                  <label className="text-xs text-text-muted">Tags (comma)</label>
                  <div className="relative">
                    <Tag className="w-4 h-4 absolute left-2 top-2.5 text-text-muted" />
                    <input
                      value={tags}
                      onChange={(e) => setTags(e.target.value)}
                      placeholder="e.g. finance, code"
                      className="w-full pl-8 pr-3 py-2 rounded-md border border-border-default bg-background-default"
                    />
                  </div>
                </div>
                <div className="flex items-center justify-end gap-2 pt-1">
                  <button
                    onClick={() => { setSearch(''); setMinScore(''); setTags(''); }}
                    className="px-3 py-2 text-sm rounded border border-border-default"
                  >
                    Reset
                  </button>
                  <button
                    onClick={() => { setPage(1); fetchList(); setFiltersOpen(false); }}
                    className="px-3 py-2 text-sm rounded bg-background-accent text-text-on-accent hover:bg-background-accent/90"
                  >
                    Apply
                  </button>
                </div>
              </div>
            </div>
          )}
          <button
            className={`p-2 rounded-lg ${groupByDataset ? 'bg-background-accent text-text-on-accent' : 'bg-background-subtle hover:bg-background-medium'}`}
            onClick={() => setGroupByDataset((g) => !g)}
            title="Group by dataset"
            aria-label="Group by dataset"
          >
            <Layers className="w-4 h-4" />
          </button>
          {groupByDataset && (
            <>
              <button
                className="p-2 bg-background-subtle hover:bg-background-medium rounded-lg"
                title="Expand all groups"
                aria-label="Expand all groups"
                onClick={() => setExpandedGroups(Object.fromEntries(Object.keys(groupedBy || {}).map(k => [k, true])) as any)}
              >
                <ChevronDown className="w-4 h-4" />
              </button>
              <button
                className="p-2 bg-background-subtle hover:bg-background-medium rounded-lg"
                title="Collapse all groups"
                aria-label="Collapse all groups"
                onClick={() => setExpandedGroups(Object.fromEntries(Object.keys(groupedBy || {}).map(k => [k, false])) as any)}
              >
                <ChevronDown className="w-4 h-4 -rotate-90" />
              </button>
            </>
          )}
          <button
            ref={importBtnRef}
            onClick={() => setImportOpen((o) => !o)}
            className="p-2 bg-background-subtle hover:bg-background-medium rounded-lg"
            title="Import from URL"
            aria-label="Import from URL"
          >
            <LinkIcon className="w-4 h-4" />
          </button>
          <input
            value={uploadGroup}
            onChange={(e) => setUploadGroup(e.target.value)}
            placeholder="Group name (optional)"
            className="px-2 py-1 rounded border border-border-default bg-background-default text-sm w-44"
          />
          <button
            onClick={handleOpenFilePicker}
            className="p-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90"
            disabled={uploading}
            title="Upload JSONL"
            aria-label="Upload JSONL"
          >
            <Upload className="w-4 h-4" />
          </button>
        </div>
      </div>

      {uploading && (
        <div className="p-3 rounded-lg bg-background-muted border border-border-default text-sm">
          Uploading… {uploadDone}/{uploadTotal}
          {uploadErrors.length > 0 && (
            <details className="mt-2">
              <summary className="cursor-pointer">Errors ({uploadErrors.length})</summary>
              <ul className="list-disc ml-5 mt-1 text-red-600">
                {uploadErrors.slice(0, 10).map((err, i) => (
                  <li key={i}>{err}</li>
                ))}
                {uploadErrors.length > 10 && <li>…and {uploadErrors.length - 10} more</li>}
              </ul>
            </details>
          )}
        </div>
      )}

      {/* Import popover */}
      {importOpen && (
        <div ref={importPopoverRef} className="absolute right-2 top-16 z-50 w-96 p-3 rounded-lg border border-border-default bg-background-default shadow-lg">
          <div className="flex items-start justify-between mb-2">
            <div className="text-sm font-medium">Import from URL</div>
            <button className="p-1 rounded hover:bg-background-muted" onClick={() => setImportOpen(false)}><X className="w-4 h-4" /></button>
          </div>
          <div className="space-y-3">
            <div>
              <label className="text-xs text-text-muted">JSONL URL</label>
              <input
                value={importUrl}
                onChange={(e) => setImportUrl(e.target.value)}
                placeholder="https://huggingface.co/datasets/.../resolve/main/data.jsonl"
                className="w-full px-3 py-2 rounded border border-border-default bg-background-default"
              />
            </div>
            <div className="flex items-center justify-end gap-2">
              <button className="px-3 py-2 text-sm rounded border border-border-default" onClick={() => setImportOpen(false)}>Cancel</button>
              <button
                className="px-3 py-2 text-sm rounded bg-background-accent text-text-on-accent hover:bg-background-accent/90 disabled:opacity-50"
                disabled={!importUrl.trim()}
                onClick={async () => {
                  try {
                    const res = await fetch(`${backendUrl}/training/import/jsonl`, {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json', 'X-Secret-Key': secretKey },
                      body: JSON.stringify({ url: importUrl.trim() })
                    });
                    if (!res.ok) throw new Error(`${res.status}`);
                    const data = await res.json();
                    alert(`Imported ${data.imported} items, ${data.errors} errors`);
                    setImportOpen(false);
                    setImportUrl('');
                    await fetchList();
                  } catch (e) {
                    alert(`Import failed: ${(e as Error).message}`);
                  }
                }}
              >
                Import
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Table */}
      <div className="overflow-auto border border-border-default rounded-lg">
        <table className="w-full text-sm">
          <thead className="bg-background-muted text-text-muted">
            <tr>
              <th className="text-left p-2 font-normal">Created</th>
              <th className="text-left p-2 font-normal">Conversation</th>
              <th className="text-left p-2 font-normal">Quality</th>
              <th className="text-left p-2 font-normal">Tags</th>
              <th className="text-left p-2 font-normal">Messages</th>
              <th className="text-left p-2 font-normal">Provider/Model</th>
              <th className="text-right p-2 font-normal">Actions</th>
            </tr>
          </thead>
          <tbody>
            {groupByDataset && groupedBy ? (
              Object.entries(groupedBy).length === 0 ? (
                !loading && (
                  <tr>
                    <td colSpan={7} className="p-6 text-center text-text-muted">No training data found</td>
                  </tr>
                )
              ) : (
                Object.entries(groupedBy)
                  .slice(0, groupsVisibleCount)
                  .map(([datasetName, rows]) => {
                    const expanded = expandedGroups[datasetName] ?? true;
                    const pageItems = groupItems[datasetName] || [];
                    const total = groupCounts[datasetName] || rows.length;
                    const totalPages = Math.max(1, Math.ceil(total / groupPerPage));
                    const currentPage = groupPages[datasetName] ?? 1;
                    return (
                      <React.Fragment key={datasetName}>
                        <tr className="border-t border-border-default">
                          <td colSpan={7} className="p-0">
                            <div className="border border-border-default rounded-md overflow-hidden">
                              <div className="flex items-center justify-between px-3 py-2 bg-background-subtle">
                                <button
                                  className="flex items-center gap-2 text-left"
                                  onClick={() => setExpandedGroups((prev) => ({ ...prev, [datasetName]: !expanded }))}
                                >
                                  <ChevronDown className={`w-4 h-4 transition-transform ${expanded ? '' : '-rotate-90'}`} />
                                  <span className="font-medium text-text-default">{datasetName}</span>
                                  <span className="text-xs text-text-muted">({total})</span>
                                </button>
                                <div className="flex items-center gap-2 text-xs text-text-muted">
                                  {expanded && (
                                    <>
                                      <span>Page {currentPage} / {totalPages}</span>
                                      <div className="flex items-center gap-1">
                                        <button
                                          className="p-1 rounded hover:bg-background-muted disabled:opacity-50"
                                          disabled={currentPage <= 1}
                                          onClick={() => setGroupPages(prev => ({ ...prev, [datasetName]: Math.max(1, currentPage - 1) }))}
                                          title="Prev"
                                          aria-label="Prev"
                                        >
                                          <ChevronLeft className="w-4 h-4" />
                                        </button>
                                        <button
                                          className="p-1 rounded hover:bg-background-muted disabled:opacity-50"
                                          disabled={currentPage >= totalPages}
                                          onClick={() => setGroupPages(prev => ({ ...prev, [datasetName]: Math.min(totalPages, currentPage + 1) }))}
                                          title="Next"
                                          aria-label="Next"
                                        >
                                          <ChevronRight className="w-4 h-4" />
                                        </button>
                                        <select
                                          value={groupPerPage}
                                          onChange={(e) => setGroupPerPage(Number(e.target.value))}
                                          className="px-2 py-1 rounded border border-border-default bg-background-default text-xs"
                                        >
                                          {[5, 10, 25, 50].map(n => <option key={n} value={n}>{n}/group</option>)}
                                        </select>
                                      </div>
                                    </>
                                  )}
                                </div>
                              </div>
                              {expanded && (
                                <div className="overflow-auto">
                                  <table className="w-full text-sm">
                                    <thead className="bg-background-muted text-text-muted">
                                      <tr>
                                        <th className="text-left p-2 font-normal">Created</th>
                                        <th className="text-left p-2 font-normal">Conversation</th>
                                        <th className="text-left p-2 font-normal">Quality</th>
                                        <th className="text-left p-2 font-normal">Tags</th>
                                        <th className="text-left p-2 font-normal">Messages</th>
                                        <th className="text-left p-2 font-normal">Provider/Model</th>
                                        <th className="text-right p-2 font-normal">Actions</th>
                                      </tr>
                                    </thead>
                                    <tbody>
                                      {pageItems.map((row) => renderRow(row, openView, openEdit, (id) => setDeleteId(id)))}
                                      {!loading && pageItems.length === 0 && (
                                        <tr>
                                          <td colSpan={7} className="p-4 text-center text-text-muted">No items in this group</td>
                                        </tr>
                                      )}
                                    </tbody>
                                  </table>
                                </div>
                              )}
                            </div>
                          </td>
                        </tr>
                      </React.Fragment>
                    );
                  })
              )
            ) : (
              <>
                {items.map((row) => renderRow(row, openView, openEdit, (id) => setDeleteId(id)))}
                {!loading && items.length === 0 && (
                  <tr>
                    <td colSpan={7} className="p-6 text-center text-text-muted">No training data found</td>
                  </tr>
                )}
              </>
            )}
            {loading && (
              <tr>
                <td colSpan={7} className="p-6 text-center text-text-muted">Loading…</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between">
        <div className="text-xs text-text-muted">Total: {count}</div>
        <div className="flex items-center gap-2">
          <button
            className="p-2 rounded hover:bg-background-muted disabled:opacity-50"
            disabled={page <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
          <span className="text-sm">{page} / {totalPages}</span>
          <button
            className="p-2 rounded hover:bg-background-muted disabled:opacity-50"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
          >
            <ChevronRight className="w-4 h-4" />
          </button>
          <select
            value={perPage}
            onChange={(e) => { setPerPage(Number(e.target.value)); setPage(1); }}
            className="ml-2 px-2 py-1 rounded border border-border-default bg-background-default text-sm"
          >
            {[10, 25, 50, 100].map(n => <option key={n} value={n}>{n}/page</option>)}
          </select>
        </div>
      </div>

      {/* View modal */}
      {viewId && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setViewId(null)}>
          <div className="bg-background-default rounded-2xl p-4 w-full max-w-2xl max-h-[80vh] overflow-auto" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-3">
              <h4 className="text-lg font-medium">Example Details</h4>
              <button className="p-2 rounded hover:bg-background-medium" onClick={() => setViewId(null)}><X className="w-4 h-4" /></button>
            </div>
            {viewLoading ? (
              <div className="text-text-muted">Loading…</div>
            ) : viewData?.error ? (
              <div className="text-red-500">{String(viewData.error)}</div>
            ) : viewData ? (
              <div className="space-y-3 text-sm">
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <div className="text-text-muted">ID</div>
                    <div className="break-all">{viewData.id}</div>
                  </div>
                  <div>
                    <div className="text-text-muted">Conversation</div>
                    <div>{viewData.conversation_id}</div>
                  </div>
                  <div>
                    <div className="text-text-muted">Created</div>
                    <div>{new Date(viewData.created_at).toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-text-muted">Quality</div>
                    <div>{(viewData.quality_metrics?.overall_score ?? 0).toFixed(2)}</div>
                  </div>
                </div>
                <div>
                  <div className="text-text-muted mb-1">Tags</div>
                  <div className="flex flex-wrap gap-1">
                    {(viewData.domain_tags || []).map((t: string) => (
                      <span key={t} className="px-2 py-0.5 rounded bg-background-muted text-text-muted text-xs">{t}</span>
                    ))}
                  </div>
                </div>
                <div>
                  <div className="text-text-muted mb-1">Messages</div>
                  <div className="space-y-2">
                    {(viewData.messages || []).map((m: Message, idx: number) => (
                      <div key={idx} className="p-2 rounded border border-border-default">
                        <div className="text-xs text-text-muted mb-1">{m.role}</div>
                        <div className="whitespace-pre-wrap text-text-default">{m?.content || ''}</div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        </div>
      )}

      {/* Edit modal */}
      {editId && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setEditId(null)}>
          <div className="bg-background-default rounded-2xl p-4 w-full max-w-lg" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-3">
              <h4 className="text-lg font-medium">Edit Training Example</h4>
              <button className="p-2 rounded hover:bg-background-medium" onClick={() => setEditId(null)}><X className="w-4 h-4" /></button>
            </div>
            <div className="space-y-3 text-sm">
              <div>
                <label className="text-xs text-text-muted">Tags (comma)</label>
                <input value={editTags} onChange={(e) => setEditTags(e.target.value)} className="w-full px-3 py-2 rounded border border-border-default bg-background-default" />
              </div>
              <div>
                <label className="text-xs text-text-muted">Quality Score (0..1)</label>
                <input value={editQuality} onChange={(e) => setEditQuality(e.target.value)} className="w-full px-3 py-2 rounded border border-border-default bg-background-default" />
              </div>
              <div>
                <label className="text-xs text-text-muted">Correction</label>
                <textarea value={editCorrection} onChange={(e) => setEditCorrection(e.target.value)} className="w-full px-3 py-2 rounded border border-border-default bg-background-default" rows={3} />
              </div>
              <div>
                <label className="text-xs text-text-muted">Comments</label>
                <textarea value={editComments} onChange={(e) => setEditComments(e.target.value)} className="w-full px-3 py-2 rounded border border-border-default bg-background-default" rows={3} />
              </div>
            </div>
            <div className="flex items-center justify-end gap-2 mt-4">
              <button className="px-3 py-2 rounded border border-border-default" onClick={() => setEditId(null)}>Cancel</button>
              <button disabled={editSaving} className="px-3 py-2 rounded bg-background-accent text-text-on-accent disabled:opacity-50" onClick={saveEdit}>{editSaving ? 'Saving…' : 'Save'}</button>
            </div>
          </div>
        </div>
      )}

      {/* Delete confirm */}
      {deleteId && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setDeleteId(null)}>
          <div className="bg-background-default rounded-2xl p-4 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-lg font-medium">Delete Example</h4>
              <button className="p-2 rounded hover:bg-background-medium" onClick={() => setDeleteId(null)}><X className="w-4 h-4" /></button>
            </div>
            <p className="text-sm text-text-muted mb-4">Are you sure you want to delete this training example? This cannot be undone.</p>
            <div className="flex items-center justify-end gap-2">
              <button className="px-3 py-2 rounded border border-border-default" onClick={() => setDeleteId(null)}>Cancel</button>
              <button className="px-3 py-2 rounded bg-red-600 text-white" onClick={confirmDelete}>Delete</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
