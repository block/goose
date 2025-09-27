import { useState, useEffect } from 'react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Play } from 'lucide-react';
import { listApps, GooseApp } from '../../api';

export default function AppsView() {
  const [apps, setApps] = useState<GooseApp[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
            <div className="text-center py-12">
              <p className="text-text-muted">No apps installed</p>
            </div>
          ) : (
            <div className="grid gap-4">
              {apps.map((app, index) => (
                <div
                  key={index}
                  className="flex items-center justify-between p-4 border border-border-muted rounded-lg bg-background-panel"
                >
                  <div className="flex-1">
                    <h3 className="font-medium text-text-default">{app.name}</h3>
                    {app.description && (
                      <p className="text-sm text-text-muted mt-1">{app.description}</p>
                    )}
                  </div>
                  <Button
                    variant="default"
                    size="sm"
                    onClick={() => handleLaunchApp(app)}
                    className="flex items-center gap-2"
                  >
                    <Play className="h-4 w-4" />
                    Launch
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Bottom padding space */}
        <div className="block h-8" />
      </div>
    </MainPanelLayout>
  );
}
