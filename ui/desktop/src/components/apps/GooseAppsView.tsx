import { useEffect, useState } from 'react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Play, Plus } from 'lucide-react';
import { GooseApp, listApps } from '../../api';
import { Recipe } from '../../recipe';

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
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const recipe = (for_app: GooseApp | null): Recipe => {
    return {
      description: '',
      title: for_app ? `update ${for_app.name} app` : 'Create goose app',
      internal: true,
    };
  };

  useEffect(() => {
    const loadApps = async () => {
      try {
        setLoading(true);
        const response = await listApps({ throwOnError: true });
        setApps(response.data?.apps || []);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load apps');
      } finally {
        setLoading(false);
      }
    };

    loadApps();
  }, []);

  const handleLaunchApp = async (app: GooseApp) => {
    await window.electron.launchGooseApp(app);
  };

  const handleAddApp = () => {
    window.electron.createChatWindow(undefined, undefined, undefined, undefined, recipe(null));
  };

  if (loading) {
    return (
      <MainPanelLayout>
        <div className="flex justify-center items-center h-64">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
        </div>
      </MainPanelLayout>
    );
  }

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
                  <Button
                    variant="default"
                    size="sm"
                    onClick={() => handleLaunchApp(app)}
                    className="flex items-center gap-2 w-full"
                  >
                    <Play className="h-4 w-4" />
                    Launch
                  </Button>
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
