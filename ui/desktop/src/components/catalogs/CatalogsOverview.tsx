import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Wrench,
  Bot,
  FileText,
  ChevronRight,
  Package,
  Download,
  Plus,
  Search,
  RefreshCw,
  ExternalLink,
  CheckCircle2,
  AlertCircle,
} from 'lucide-react';
import { PageShell } from '../Layout/PageShell';
import { getExtensions, listBuiltinAgents, listAgents } from '../../api';
import { listSavedRecipes } from '../../recipe/recipe_management';

interface CatalogItem {
  id: string;
  name: string;
  description: string;
  status: 'installed' | 'available' | 'error';
  type: string;
}

interface CatalogCategory {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  route: string;
  color: string;
  items: CatalogItem[];
  loading: boolean;
  actions: { label: string; icon: React.ReactNode; onClick: () => void }[];
}

function CatalogCard({ category }: { category: CatalogCategory }) {
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState(false);
  const installed = category.items.filter((i) => i.status === 'installed').length;
  const errors = category.items.filter((i) => i.status === 'error').length;
  const visibleItems = expanded ? category.items : category.items.slice(0, 3);
  const hasMore = category.items.length > 3;

  return (
    <div
      className="group relative bg-background-default border border-border-default rounded-xl p-6 hover:border-border-accent hover:shadow-lg transition-all cursor-pointer"
      onClick={() => navigate(category.route)}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div
            className="w-10 h-10 rounded-lg flex items-center justify-center"
            style={{ backgroundColor: `${category.color}20`, color: category.color }}
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
          <CheckCircle2 className="w-4 h-4 text-green-500" />
          <span className="text-text-default font-medium">{installed}</span>
          <span className="text-text-muted">installed</span>
        </div>
        {errors > 0 && (
          <div className="flex items-center gap-1.5 text-sm">
            <AlertCircle className="w-4 h-4 text-red-500" />
            <span className="text-red-400 font-medium">{errors}</span>
            <span className="text-text-muted">issues</span>
          </div>
        )}
        <div className="flex items-center gap-1.5 text-sm">
          <Package className="w-4 h-4 text-text-muted" />
          <span className="text-text-default font-medium">{category.items.length}</span>
          <span className="text-text-muted">total</span>
        </div>
      </div>

      <div className="space-y-1.5">
        {visibleItems.map((item) => (
          <div
            key={item.id}
            className="flex items-center justify-between text-sm px-2 py-1.5 rounded-md bg-background-subtle"
          >
            <div className="flex items-center gap-2 min-w-0">
              <div
                className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                  item.status === 'installed'
                    ? 'bg-green-500'
                    : item.status === 'error'
                      ? 'bg-red-500'
                      : 'bg-gray-400'
                }`}
              />
              <span className="text-text-default truncate">{item.name}</span>
            </div>
            {item.type && (
              <span className="text-xs text-text-muted px-1.5 py-0.5 rounded bg-background-default flex-shrink-0">
                {item.type}
              </span>
            )}
          </div>
        ))}
        {hasMore && (
          <button
            className="w-full text-xs text-text-muted hover:text-text-default text-center py-1 rounded-md hover:bg-background-subtle transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              setExpanded(!expanded);
            }}
          >
            {expanded
              ? '▲ Show less'
              : `▼ +${category.items.length - 3} more`}
          </button>
        )}
        {category.items.length === 0 && !category.loading && (
          <div className="text-sm text-text-muted text-center py-3">
            No items yet — click to browse
          </div>
        )}
        {category.loading && (
          <div className="space-y-1.5">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-8 bg-background-subtle rounded-md animate-pulse" />
            ))}
          </div>
        )}
      </div>

      {category.actions.length > 0 && (
        <div className="flex items-center gap-2 mt-4 pt-4 border-t border-border-default">
          {category.actions.map((action, i) => (
            <button
              key={i}
              className="flex items-center gap-1.5 text-xs text-text-muted hover:text-text-default px-2 py-1 rounded-md hover:bg-background-subtle transition-colors"
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

export default function CatalogsOverview() {
  const navigate = useNavigate();
  const [categories, setCategories] = useState<CatalogCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    loadCatalogs();
  }, []);

  async function loadCatalogs() {
    setLoading(true);

    const initial: CatalogCategory[] = [
      {
        id: 'tools',
        label: 'Tools',
        description: 'MCP extensions that provide capabilities to agents',
        icon: <Wrench className="w-5 h-5" />,
        route: '/extensions',
        color: '#8b5cf6',
        items: [],
        loading: true,
        actions: [
          {
            label: 'Add Tool',
            icon: <Plus className="w-3.5 h-3.5" />,
            onClick: () => navigate('/extensions'),
          },
          {
            label: 'Browse Registry',
            icon: <ExternalLink className="w-3.5 h-3.5" />,
            onClick: () =>
              window.open('https://block.github.io/goose/v1/extensions/', '_blank'),
          },
        ],
      },
      {
        id: 'agents',
        label: 'Agents',
        description: 'AI agents with specialized modes and capabilities',
        icon: <Bot className="w-5 h-5" />,
        route: '/agents',
        color: '#3b82f6',
        items: [],
        loading: true,
        actions: [
          {
            label: 'View Agents',
            icon: <Bot className="w-3.5 h-3.5" />,
            onClick: () => navigate('/agents'),
          },
        ],
      },
      {
        id: 'workflows',
        label: 'Workflows',
        description: 'Reusable prompts and automation pipelines',
        icon: <FileText className="w-5 h-5" />,
        route: '/recipes',
        color: '#10b981',
        items: [],
        loading: true,
        actions: [
          {
            label: 'Create Workflow',
            icon: <Plus className="w-3.5 h-3.5" />,
            onClick: () => navigate('/recipes'),
          },
          {
            label: 'Import',
            icon: <Download className="w-3.5 h-3.5" />,
            onClick: () => navigate('/recipes'),
          },
        ],
      },
    ];
    setCategories(initial);

    // Load tools (extensions)
    try {
      const resp = await getExtensions();
      const extensions = resp.data?.extensions || [];
      const toolItems: CatalogItem[] = extensions.map((ext) => ({
        id: ext.name,
        name: ext.name,
        description: ext.description || '',
        status: ext.enabled ? ('installed' as const) : ('available' as const),
        type: ext.type,
      }));
      setCategories((prev) =>
        prev.map((c) => (c.id === 'tools' ? { ...c, items: toolItems, loading: false } : c))
      );
    } catch {
      setCategories((prev) =>
        prev.map((c) => (c.id === 'tools' ? { ...c, loading: false } : c))
      );
    }

    // Load agents (builtin + external A2A)
    try {
      const agentItems: CatalogItem[] = [];

      // Builtin agents
      const builtinResp = await listBuiltinAgents();
      const builtinAgents = builtinResp.data?.agents || [];
      for (const a of builtinAgents) {
        agentItems.push({
          id: a.name,
          name: a.name,
          description: a.description || '',
          status: a.enabled ? ('installed' as const) : ('available' as const),
          type: `${a.modes.length} modes`,
        });
      }

      // External A2A agents
      try {
        const externalResp = await listAgents();
        const externalAgents = externalResp.data?.agents || [];
        const seen = new Set(agentItems.map((a) => a.id));
        for (const a of externalAgents) {
          if (!seen.has(a.name)) {
            agentItems.push({
              id: a.name,
              name: a.name,
              description: a.description || 'External agent',
              status: 'installed' as const,
              type: `${(a.modes || []).length} modes`,
            });
          }
        }
      } catch {
        // External agents are optional — don't fail the whole section
      }

      setCategories((prev) =>
        prev.map((c) => (c.id === 'agents' ? { ...c, items: agentItems, loading: false } : c))
      );
    } catch {
      setCategories((prev) =>
        prev.map((c) => (c.id === 'agents' ? { ...c, loading: false } : c))
      );
    }

    // Load workflows (recipes)
    try {
      const recipes = await listSavedRecipes();
      const workflowItems: CatalogItem[] = (recipes || []).map((r) => ({
        id: r.recipe.title || r.id,
        name: r.recipe.title || 'Untitled',
        description: r.recipe.description || '',
        status: 'installed' as const,
        type: 'recipe',
      }));
      setCategories((prev) =>
        prev.map((c) =>
          c.id === 'workflows' ? { ...c, items: workflowItems, loading: false } : c
        )
      );
    } catch {
      setCategories((prev) =>
        prev.map((c) => (c.id === 'workflows' ? { ...c, loading: false } : c))
      );
    }

    setLoading(false);
  }

  const totalInstalled = categories.reduce(
    (sum, c) => sum + c.items.filter((i) => i.status === 'installed').length,
    0
  );
  const totalItems = categories.reduce((sum, c) => sum + c.items.length, 0);

  const filteredCategories = searchTerm
    ? categories.map((c) => ({
        ...c,
        items: c.items.filter(
          (i) =>
            i.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
            i.description.toLowerCase().includes(searchTerm.toLowerCase())
        ),
      }))
    : categories;

  return (
    <PageShell
      title="Catalogs"
      subtitle={`${totalInstalled} installed across ${categories.length} catalogs • ${totalItems} total packages`}
      actions={
        <>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
            <input
              type="text"
              placeholder="Search all catalogs..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 pr-4 py-2 bg-background-subtle border border-border-default rounded-lg text-sm text-text-default placeholder-text-muted focus:outline-none focus:border-border-accent w-64"
            />
          </div>
          <button
            onClick={loadCatalogs}
            className="p-2 text-text-muted hover:text-text-default rounded-lg hover:bg-background-subtle transition-colors"
            title="Refresh"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </>
      }
    >

        {/* Catalog cards grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {filteredCategories.map((category) => (
            <CatalogCard key={category.id} category={category} />
          ))}
        </div>

        {/* Search results flat view */}
        {searchTerm && (
          <div className="bg-background-default border border-border-default rounded-xl p-6">
            <h3 className="text-sm font-semibold text-text-muted uppercase tracking-wider mb-4">
              Search Results
            </h3>
            <div className="space-y-2">
              {filteredCategories.flatMap((c) =>
                c.items.map((item) => (
                  <div
                    key={`${c.id}-${item.id}`}
                    className="flex items-center justify-between p-3 rounded-lg hover:bg-background-subtle cursor-pointer"
                    onClick={() => navigate(c.route)}
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-8 h-8 rounded-lg flex items-center justify-center"
                        style={{
                          backgroundColor: `${c.color}20`,
                          color: c.color,
                        }}
                      >
                        {c.icon}
                      </div>
                      <div>
                        <div className="text-sm font-medium text-text-default">{item.name}</div>
                        <div className="text-xs text-text-muted">{item.description}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs px-2 py-0.5 rounded bg-background-subtle text-text-muted">
                        {c.label}
                      </span>
                      <div
                        className={`w-2 h-2 rounded-full ${
                          item.status === 'installed'
                            ? 'bg-green-500'
                            : item.status === 'error'
                              ? 'bg-red-500'
                              : 'bg-gray-400'
                        }`}
                      />
                    </div>
                  </div>
                ))
              )}
              {filteredCategories.every((c) => c.items.length === 0) && (
                <div className="text-sm text-text-muted text-center py-6">
                  No results for &quot;{searchTerm}&quot;
                </div>
              )}
            </div>
          </div>
        )}
    </PageShell>
  );
}
