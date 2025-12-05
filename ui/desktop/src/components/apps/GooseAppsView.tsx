import { useCallback, useEffect, useState } from 'react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Play, Plus, Trash, Pencil } from 'lucide-react';
import { deleteApp, GooseApp, listApps } from '../../api';
import GooseAppEditor from './GooseAppEditor';

const GridLayout = ({ children }: { children: React.ReactNode }) => {
  return (
    <div
      className="grid gap-4 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
        justifyContent: 'center',
      }}
    >
      {children}
    </div>
  );
};

const AddAppCard = ({ onClick }: { onClick: () => void }) => {
  return (
    <div
      onClick={onClick}
      className="flex items-center justify-center p-4 border-2 border-dashed border-border-muted rounded-lg bg-background-panel hover:bg-background-subtle cursor-pointer transition-colors min-h-[120px]"
    >
      <div className="flex flex-col items-center">
        <Plus className="w-8 h-8 text-text-muted mb-2" />
        <div className="text-sm text-text-muted text-center">
          <div>Add App</div>
        </div>
      </div>
    </div>
  );
};

export default function GooseAppsView() {
  const [apps, setApps] = useState<GooseApp[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [editingApp, setEditingApp] = useState<GooseApp | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  const loadApps = useCallback(async () => {
    try {
      const response = await listApps({ throwOnError: true });
      setApps(response.data?.apps || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load apps');
    }
  }, []);

  useEffect(() => {
    loadApps();
  }, [loadApps]);

  const handleLaunchApp = async (app: GooseApp) => {
    await window.electron.launchGooseApp(app);
  };

  const handleReturn = async () => {
    setIsCreating(false);
    setEditingApp(null);
    await loadApps();
  };

  if (isCreating || editingApp) {
    return <GooseAppEditor app={editingApp || undefined} onReturn={handleReturn} />;
  }

  const handleAddApp = () => {
    setIsCreating(true);
  };

  const handleEditApp = (app: GooseApp) => {
    setEditingApp(app);
  };

  const handleDeleteApp = async (app: GooseApp) => {
    const name = app.name;
    if (!window.confirm(`Are you sure you want to delete "${name}"?`)) {
      return;
    }

    try {
      await deleteApp({ throwOnError: true, path: { name } });
      setApps(apps.filter((a) => a !== app));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete app');
    }
  };

  if (error) {
    return (
      <MainPanelLayout>
        <div className="flex flex-col items-center justify-center h-64 text-center">
          <p className="text-red-500 mb-4">Error loading apps: {error}</p>
          <Button onClick={() => window.location.reload()}>Retry</Button>
        </div>
      </MainPanelLayout>
    );
  }

  return (
    <MainPanelLayout>
      <div className="flex flex-col min-w-0 flex-1 overflow-y-auto relative">
        <div className="bg-background-default px-8 pb-4 pt-16">
          <div className="flex flex-col page-transition">
            <div className="flex justify-between items-center mb-1">
              <h1 className="text-4xl font-light">Apps</h1>
            </div>
            <p className="text-sm text-text-muted mb-6">
              Self-contained JavaScript applications that run within Goose.
            </p>
          </div>
        </div>

        <div className="px-8 pb-16">
          {apps.length === 0 ? (
            <GridLayout>
              <AddAppCard onClick={handleAddApp} />
            </GridLayout>
          ) : (
            <GridLayout>
              {apps.map((app, index) => (
                <div
                  key={index}
                  className="flex flex-col p-4 border border-border-muted rounded-lg bg-background-panel"
                >
                  <div className="flex-1 mb-4">
                    <h3 className="font-medium text-text-default mb-2">{app.name}</h3>
                    {app.description && (
                      <p className="text-sm text-text-muted">{app.description}</p>
                    )}
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="default"
                      size="sm"
                      onClick={() => handleLaunchApp(app)}
                      className="flex items-center gap-2 flex-1"
                    >
                      <Play className="h-4 w-4" />
                      Launch
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleEditApp(app)}
                      className="flex items-center gap-2"
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleDeleteApp(app)}
                      className="flex items-center gap-2 text-red-500 hover:text-red-600"
                    >
                      <Trash className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
              <AddAppCard onClick={handleAddApp} />
            </GridLayout>
          )}
        </div>

        <div className="block h-8" />
      </div>
    </MainPanelLayout>
  );
}
