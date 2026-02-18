import { useState, useMemo } from 'react';
import { Activity, Clock, Cpu, Rocket, Search, SlidersHorizontal, ArrowUpDown } from 'lucide-react';
import type { InstanceResponse, InstanceStatus } from '../../lib/instances';
import { InstanceStatusBadge } from './InstanceStatusBadge';
import { StatCard } from '../ui/design-system';

function formatElapsed(secs?: number): string {
  if (secs == null) return '—';
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${Math.round(secs % 60)}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

function formatLastActivity(ms?: number): string {
  if (ms == null) return '';
  if (ms < 1000) return 'just now';
  if (ms < 60000) return `${Math.round(ms / 1000)}s ago`;
  if (ms < 3600000) return `${Math.round(ms / 60000)}m ago`;
  return `${Math.round(ms / 3600000)}h ago`;
}

type StatusFilter = InstanceStatus | 'all';
type SortField = 'status' | 'persona' | 'elapsed';

const STATUS_ORDER: Record<string, number> = {
  running: 0,
  completed: 1,
  failed: 2,
  cancelled: 3,
};

interface InstanceListProps {
  instances: InstanceResponse[];
  loading: boolean;
  onSelect: (id: string) => void;
  selectedId?: string | null;
  onSpawnClick: () => void;
  runningCount: number;
  completedCount: number;
  failedCount: number;
}

export function InstanceList({
  instances,
  loading,
  onSelect,
  selectedId,
  onSpawnClick,
  runningCount,
  completedCount,
  failedCount,
}: InstanceListProps) {
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [sortField, setSortField] = useState<SortField>('status');
  const [sortAsc, setSortAsc] = useState(true);

  const filtered = useMemo(() => {
    let result = instances.filter((inst) => {
      if (statusFilter !== 'all' && inst.status !== statusFilter) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        return (
          inst.persona.toLowerCase().includes(q) ||
          inst.id.toLowerCase().includes(q) ||
          (inst.provider_name || '').toLowerCase().includes(q) ||
          (inst.model_name || '').toLowerCase().includes(q)
        );
      }
      return true;
    });

    // Sort
    result = [...result].sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'status':
          cmp = (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9);
          break;
        case 'persona':
          cmp = a.persona.localeCompare(b.persona);
          break;
        case 'elapsed':
          cmp = (a.elapsed_secs ?? 0) - (b.elapsed_secs ?? 0);
          break;
      }
      return sortAsc ? cmp : -cmp;
    });

    return result;
  }, [instances, statusFilter, searchQuery, sortField, sortAsc]);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(true);
    }
  };

  const statusFilters: { value: StatusFilter; label: string; count: number }[] = [
    { value: 'all', label: 'All', count: instances.length },
    { value: 'running', label: 'Running', count: runningCount },
    { value: 'completed', label: 'Completed', count: completedCount },
    { value: 'failed', label: 'Failed', count: failedCount },
  ];

  return (
    <div className="space-y-6">
      {/* Stats Row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="Total" value={instances.length} icon={Cpu} />
        <StatCard
          label="Running"
          value={runningCount}
          icon={Activity}
          variant={runningCount > 0 ? 'warning' : 'default'}
        />
        <StatCard label="Completed" value={completedCount} icon={Clock} variant="success" />
        <StatCard
          label="Failed"
          value={failedCount}
          icon={Activity}
          variant={failedCount > 0 ? 'danger' : 'default'}
        />
      </div>

      {/* Search + Filter + Sort Bar */}
      <div className="space-y-3">
        <div className="flex items-center gap-3">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-subtle" />
            <input
              type="text"
              placeholder="Search instances..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-8 pr-3 py-1.5 text-sm rounded-lg border border-border-default bg-background-default outline-none focus:ring-2 focus:ring-accent-default/30 focus:border-accent-default"
            />
          </div>
          <button
            onClick={() =>
              handleSort(
                sortField === 'status' ? 'elapsed' : sortField === 'elapsed' ? 'persona' : 'status'
              )
            }
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-text-muted hover:text-text-default rounded-md hover:bg-background-muted transition-colors"
            title={`Sort by ${sortField} (${sortAsc ? 'asc' : 'desc'})`}
          >
            <ArrowUpDown className="w-3.5 h-3.5" />
            {sortField}
          </button>
        </div>
        <div className="flex items-center gap-1">
          <SlidersHorizontal className="w-3.5 h-3.5 text-text-subtle mr-1" />
          {statusFilters.map((f) => (
            <button
              key={f.value}
              onClick={() => setStatusFilter(f.value)}
              className={`px-2.5 py-1 text-xs rounded-full transition-colors ${
                statusFilter === f.value
                  ? 'bg-accent-default text-white'
                  : 'bg-background-muted text-text-muted hover:bg-background-muted/80'
              }`}
            >
              {f.label}
              {f.count > 0 && <span className="ml-1 opacity-70">({f.count})</span>}
            </button>
          ))}
        </div>
      </div>

      {/* Instance List */}
      {loading && instances.length === 0 ? (
        <div className="text-center py-16 text-text-muted">
          <Activity className="w-8 h-8 mx-auto mb-3 animate-pulse" />
          <p>Loading instances...</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-16 text-text-muted">
          {instances.length === 0 ? (
            <>
              <Rocket className="w-12 h-12 mx-auto mb-3 opacity-30" />
              <p className="text-lg font-medium">No instances yet</p>
              <p className="text-sm mt-1 mb-4">
                Deploy an agent from the Catalog tab to get started
              </p>
              <button
                onClick={onSpawnClick}
                className="inline-flex items-center gap-2 px-4 py-2 text-sm bg-accent-default text-white rounded-lg hover:bg-accent-default/90 transition-colors"
              >
                <Rocket className="w-4 h-4" />
                Deploy Instance
              </button>
            </>
          ) : (
            <>
              <Search className="w-8 h-8 mx-auto mb-3 opacity-30" />
              <p className="text-sm">No instances match your filters</p>
            </>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((inst) => (
            <button
              key={inst.id}
              onClick={() => onSelect(inst.id)}
              className={`w-full text-left p-4 rounded-xl border transition-all duration-150 ${
                selectedId === inst.id
                  ? 'border-accent-default bg-accent-default/5 ring-1 ring-accent-default/20'
                  : 'border-border-default hover:border-border-muted hover:shadow-sm'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-medium text-sm truncate">{inst.persona}</span>
                    <InstanceStatusBadge status={inst.status as InstanceStatus} size="sm" />
                  </div>
                  <div className="flex items-center gap-3 text-xs text-text-muted">
                    <span className="font-mono truncate max-w-[120px]" title={inst.id}>
                      {inst.id.slice(0, 8)}
                    </span>
                    {inst.model_name && (
                      <span className="truncate max-w-[160px]">
                        {inst.provider_name ? `${inst.provider_name}/` : ''}
                        {inst.model_name}
                      </span>
                    )}
                  </div>
                </div>
                <div className="text-right shrink-0 ml-3">
                  <div className="text-xs text-text-muted">
                    {inst.turns} turn{inst.turns !== 1 ? 's' : ''}
                  </div>
                  <div className="text-xs text-text-subtle">
                    {inst.status === 'running'
                      ? formatLastActivity(inst.last_activity_ms)
                      : formatElapsed(inst.elapsed_secs)}
                  </div>
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
