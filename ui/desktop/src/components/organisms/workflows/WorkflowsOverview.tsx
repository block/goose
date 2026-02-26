import {
  AlertCircle,
  Calendar,
  CheckCircle2,
  ChevronRight,
  Clock,
  FileText,
  GitBranch,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Search,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { listPipelines, listSchedules, type RecipeManifest } from '@/api';
import { listSavedRecipes } from '@/recipe/recipe_management';
import { PageShell } from '@/components/templates/layout/PageShell';

interface WorkflowItem {
  id: string;
  name: string;
  description: string;
  status: 'active' | 'paused' | 'draft' | 'error';
  type: string;
}

interface WorkflowCategory {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  route: string;
  color: string;
  items: WorkflowItem[];
  loading: boolean;
  actions: { label: string; icon: React.ReactNode; onClick: () => void }[];
}

function WorkflowCard({ category }: { category: WorkflowCategory }) {
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState(false);
  const active = category.items.filter((i) => i.status === 'active').length;
  const errors = category.items.filter((i) => i.status === 'error').length;
  const visibleItems = expanded ? category.items : category.items.slice(0, 4);
  const hasMore = category.items.length > 4;

  return (
    <div
      className="group relative bg-background-default border border-border-default rounded-xl p-6 hover:border-border-accent hover:shadow-lg transition-all cursor-pointer"
      onClick={() => navigate(category.route)}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div
            className="w-10 h-10 rounded-lg flex items-center justify-center"
            style={{
              backgroundColor: `${category.color}20`,
              color: category.color,
            }}
          >
            {category.icon}
          </div>
          <div>
            <h3 className="text-lg font-semibold text-text-default">{category.label}</h3>
            <p className="text-sm text-text-muted">{category.description}</p>
          </div>
        </div>
        <ChevronRight className="w-5 h-5 text-text-muted opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>

      <div className="flex items-center gap-4 mb-4">
        <div className="flex items-center gap-1.5 text-sm">
          <span className="text-text-default font-medium">{category.items.length}</span>
          <span className="text-text-muted">total</span>
        </div>
        {active > 0 && (
          <div className="flex items-center gap-1.5 text-sm">
            <CheckCircle2 className="w-4 h-4 text-text-success" />
            <span className="text-text-default font-medium">{active}</span>
            <span className="text-text-muted">active</span>
          </div>
        )}
        {errors > 0 && (
          <div className="flex items-center gap-1.5 text-sm">
            <AlertCircle className="w-4 h-4 text-text-danger" />
            <span className="text-text-danger font-medium">{errors}</span>
            <span className="text-text-muted">issues</span>
          </div>
        )}
      </div>

      <div className="space-y-1.5" onClick={(e) => e.stopPropagation()}>
        {visibleItems.map((item) => (
          <div
            key={item.id}
            className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-background-muted transition-colors"
          >
            <div
              className={`w-2 h-2 rounded-full flex-shrink-0 ${
                item.status === 'active'
                  ? 'bg-green-500'
                  : item.status === 'paused'
                    ? 'bg-amber-500'
                    : item.status === 'error'
                      ? 'bg-red-500'
                      : 'bg-gray-400'
              }`}
            />
            <span className="text-text-default truncate text-sm">{item.name}</span>
            {item.type && (
              <span className="text-xs text-text-muted px-1.5 py-0.5 rounded bg-background-default flex-shrink-0">
                {item.type}
              </span>
            )}
          </div>
        ))}
        {hasMore && (
          <button
            className="w-full text-xs text-text-muted hover:text-text-default text-center py-1 rounded-md hover:bg-background-muted transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              setExpanded(!expanded);
            }}
          >
            {expanded ? '▲ Show less' : `▼ +${category.items.length - 4} more`}
          </button>
        )}
        {category.items.length === 0 && !category.loading && (
          <div className="text-sm text-text-muted text-center py-3">
            No items yet — click to get started
          </div>
        )}
        {category.loading && (
          <div className="space-y-1.5">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-8 bg-background-muted rounded-md animate-pulse" />
            ))}
          </div>
        )}
      </div>

      {category.actions.length > 0 && (
        <div className="flex items-center gap-2 mt-4 pt-4 border-t border-border-default">
          {category.actions.map((action, i) => (
            <button
              key={i}
              className="flex items-center gap-1.5 text-xs text-text-muted hover:text-text-default px-2 py-1 rounded-md hover:bg-background-muted transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                action.onClick();
              }}
            >
              {action.icon}
              {action.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default function WorkflowsOverview() {
  const navigate = useNavigate();
  const [categories, setCategories] = useState<WorkflowCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');

  const loadWorkflows = useCallback(async () => {
    setLoading(true);

    const initial: WorkflowCategory[] = [
      {
        id: 'recipes',
        label: 'Recipes',
        description: 'Conversational workflows with step-by-step instructions',
        icon: <FileText className="w-5 h-5" />,
        route: '/recipes',
        color: '#f59e0b',
        items: [],
        loading: true,
        actions: [
          {
            label: 'Create Recipe',
            icon: <Plus className="w-3.5 h-3.5" />,
            onClick: () => navigate('/recipes'),
          },
          {
            label: 'Import',
            icon: <FileText className="w-3.5 h-3.5" />,
            onClick: () => navigate('/recipes'),
          },
        ],
      },
      {
        id: 'pipelines',
        label: 'Pipelines',
        description: 'Visual DAG workflows connecting agents and tools',
        icon: <GitBranch className="w-5 h-5" />,
        route: '/pipelines',
        color: '#8b5cf6',
        items: [],
        loading: true,
        actions: [
          {
            label: 'New Pipeline',
            icon: <Plus className="w-3.5 h-3.5" />,
            onClick: () => navigate('/pipelines'),
          },
        ],
      },
      {
        id: 'schedules',
        label: 'Schedules',
        description: 'Automated recurring recipe and pipeline runs',
        icon: <Clock className="w-5 h-5" />,
        route: '/schedules',
        color: '#06b6d4',
        items: [],
        loading: true,
        actions: [
          {
            label: 'New Schedule',
            icon: <Calendar className="w-3.5 h-3.5" />,
            onClick: () => navigate('/schedules'),
          },
        ],
      },
    ];

    setCategories(initial);

    // Load recipes
    try {
      const recipes: RecipeManifest[] = await listSavedRecipes();
      setCategories((prev) =>
        prev.map((cat) =>
          cat.id === 'recipes'
            ? {
                ...cat,
                loading: false,
                items: recipes.map((r) => ({
                  id: r.id,
                  name: r.recipe.title || r.id,
                  description: r.recipe.description || '',
                  status: r.schedule_cron ? ('active' as const) : ('draft' as const),
                  type: r.schedule_cron ? 'scheduled' : 'manual',
                })),
              }
            : cat
        )
      );
    } catch {
      setCategories((prev) =>
        prev.map((cat) => (cat.id === 'recipes' ? { ...cat, loading: false } : cat))
      );
    }

    // Load pipelines
    try {
      const { data } = await listPipelines();
      const pipelines = data ?? [];
      setCategories((prev) =>
        prev.map((cat) =>
          cat.id === 'pipelines'
            ? {
                ...cat,
                loading: false,
                items: (
                  pipelines as Array<{
                    id: string;
                    name: string;
                    description?: string;
                    node_count?: number;
                  }>
                ).map((p) => ({
                  id: p.id,
                  name: p.name || p.id,
                  description: p.description || '',
                  status: 'draft' as const,
                  type: p.node_count ? `${p.node_count} nodes` : 'pipeline',
                })),
              }
            : cat
        )
      );
    } catch {
      setCategories((prev) =>
        prev.map((cat) => (cat.id === 'pipelines' ? { ...cat, loading: false } : cat))
      );
    }

    // Load schedules
    try {
      const { data } = await listSchedules();
      const jobs =
        (
          data as {
            jobs?: Array<{ id: string; recipe_name?: string; cron?: string; status?: string }>;
          }
        )?.jobs ?? [];
      setCategories((prev) =>
        prev.map((cat) =>
          cat.id === 'schedules'
            ? {
                ...cat,
                loading: false,
                items: jobs.map((j) => ({
                  id: j.id,
                  name: j.recipe_name || j.id,
                  description: j.cron || '',
                  status: j.status === 'paused' ? ('paused' as const) : ('active' as const),
                  type: j.cron || 'scheduled',
                })),
              }
            : cat
        )
      );
    } catch {
      setCategories((prev) =>
        prev.map((cat) => (cat.id === 'schedules' ? { ...cat, loading: false } : cat))
      );
    }

    setLoading(false);
  }, [navigate]);

  useEffect(() => {
    loadWorkflows();
  }, [loadWorkflows]);

  const totalItems = categories.reduce((sum, cat) => sum + cat.items.length, 0);
  const totalActive = categories.reduce(
    (sum, cat) => sum + cat.items.filter((i) => i.status === 'active').length,
    0
  );

  const filteredCategories = searchTerm
    ? categories.map((cat) => ({
        ...cat,
        items: cat.items.filter(
          (item) =>
            item.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
            item.description.toLowerCase().includes(searchTerm.toLowerCase())
        ),
      }))
    : categories;

  const allFilteredItems = filteredCategories.flatMap((cat) =>
    cat.items.map((item) => ({ ...item, categoryLabel: cat.label }))
  );

  const subtitle = [
    totalActive > 0 ? `${totalActive} active` : null,
    `${totalItems} total across ${categories.length} types`,
  ]
    .filter(Boolean)
    .join(' · ');

  return (
    <PageShell
      title="Workflows"
      subtitle={subtitle}
      actions={
        <>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
            <input
              type="text"
              placeholder="Search workflows..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 pr-4 py-2 text-sm bg-background-default border border-border-default rounded-lg text-text-default placeholder-text-subtle focus:outline-none focus:border-border-accent w-64"
            />
          </div>
          <button
            onClick={() => loadWorkflows()}
            className="p-2 text-text-muted hover:text-text-default hover:bg-background-muted rounded-lg transition-colors"
            title="Refresh"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </>
      }
    >
      {/* Category cards */}
      {!searchTerm && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
          {filteredCategories.map((category) => (
            <WorkflowCard key={category.id} category={category} />
          ))}
        </div>
      )}

      {/* Search results */}
      {searchTerm && (
        <div className="space-y-2">
          {allFilteredItems.length > 0 ? (
            allFilteredItems.map((item) => (
              <div
                key={`${item.categoryLabel}-${item.id}`}
                className="flex items-center gap-3 p-3 bg-background-default border border-border-default rounded-lg hover:border-border-accent transition-colors cursor-pointer"
              >
                <div
                  className={`w-2 h-2 rounded-full flex-shrink-0 ${
                    item.status === 'active'
                      ? 'bg-green-500'
                      : item.status === 'paused'
                        ? 'bg-amber-500'
                        : item.status === 'error'
                          ? 'bg-red-500'
                          : 'bg-gray-400'
                  }`}
                />
                <div className="flex-1 min-w-0">
                  <div className="text-text-default font-medium truncate">{item.name}</div>
                  {item.description && (
                    <div className="text-sm text-text-muted truncate">{item.description}</div>
                  )}
                </div>
                <span className="text-xs text-text-muted px-2 py-0.5 rounded bg-background-muted flex-shrink-0">
                  {item.categoryLabel}
                </span>
              </div>
            ))
          ) : (
            <div className="text-center py-12 text-text-muted">
              No workflows matching &ldquo;{searchTerm}&rdquo;
            </div>
          )}
        </div>
      )}

      {/* Quick stats footer */}
      {!searchTerm && !loading && (
        <div className="flex items-center justify-center gap-8 pt-4 border-t border-border-muted text-sm text-text-muted">
          <div className="flex items-center gap-2">
            <Play className="w-4 h-4" />
            <span>{totalActive} running</span>
          </div>
          <div className="flex items-center gap-2">
            <Pause className="w-4 h-4" />
            <span>
              {categories.reduce(
                (sum, cat) => sum + cat.items.filter((i) => i.status === 'paused').length,
                0
              )}{' '}
              paused
            </span>
          </div>
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4" />
            <span>
              {categories.reduce(
                (sum, cat) => sum + cat.items.filter((i) => i.status === 'draft').length,
                0
              )}{' '}
              drafts
            </span>
          </div>
        </div>
      )}
    </PageShell>
  );
}
