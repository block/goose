import { useCallback, useEffect, useState } from 'react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Play } from 'lucide-react';
import { GooseApp, listApps } from '../../api';
import { useChatContext } from '../../contexts/ChatContext';

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

export default function AppsView() {
  const [apps, setApps] = useState<GooseApp[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const chatContext = useChatContext();
  const sessionId = chatContext?.chat.sessionId;

  // Load cached apps immediately on mount
  useEffect(() => {
    const loadCachedApps = async () => {
      try {
        const response = await listApps({
          throwOnError: true,
        });
        const cachedApps = response.data?.apps || [];
        setApps(cachedApps);
      } catch (err) {
        console.warn('Failed to load cached apps:', err);
      } finally {
        setLoading(false);
      }
    };

    loadCachedApps();
  }, []);

  // When sessionId becomes available, fetch fresh apps and update cache
  useEffect(() => {
    if (!sessionId) return;

    const refreshApps = async () => {
      try {
        const response = await listApps({
          throwOnError: true,
          query: { session_id: sessionId },
        });
        const freshApps = response.data?.apps || [];
        setApps(freshApps);
        setError(null);
      } catch (err) {
        console.warn('Failed to refresh apps:', err);
        // Don't set error if we already have cached apps
        if (apps.length === 0) {
          setError(err instanceof Error ? err.message : 'Failed to load apps');
        }
      }
    };

    refreshApps();
    // apps.length intentionally not in deps: we want to capture the initial apps.length to check
    // "did we have cached apps when refresh started?" Adding it would cause infinite loop since setApps() changes apps.length
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  const loadApps = useCallback(async () => {
    if (!sessionId) return;

    try {
      setLoading(true);
      const response = await listApps({
        throwOnError: true,
        query: { session_id: sessionId },
      });
      const fetchedApps = response.data?.apps || [];
      setApps(fetchedApps);
      setError(null);
    } catch (err) {
      // Only set error if we don't have apps to show
      if (apps.length === 0) {
        setError(err instanceof Error ? err.message : 'Failed to load apps');
      }
    } finally {
      setLoading(false);
    }
  }, [sessionId, apps.length]);

  const handleLaunchApp = async (app: GooseApp) => {
    try {
      await window.electron.launchApp(app);
    } catch (err) {
      console.error('Failed to launch app:', err);
      // App launch errors shouldn't hide the apps list, just log it
    }
  };

  // Only show error-only UI if we have no apps to display
  if (error && apps.length === 0) {
    return (
      <MainPanelLayout>
        <div className="flex flex-col items-center justify-center h-64 text-center">
          <p className="text-red-500 mb-4">Error loading apps: {error}</p>
          <Button onClick={loadApps}>Retry</Button>
        </div>
      </MainPanelLayout>
    );
  }

  return (
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0">
        <div className="bg-background-default px-8 pb-8 pt-16">
          <div className="flex flex-col page-transition">
            <div className="flex justify-between items-center mb-1">
              <h1 className="text-4xl font-light">Apps</h1>
            </div>
            <p className="text-sm text-text-muted mb-4">
              Applications from your MCP servers that can run in standalone windows.
            </p>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto bg-background-subtle px-8 pb-8">
          {loading ? (
            <div className="flex items-center justify-center h-64">
              <p className="text-text-muted">Loading apps...</p>
            </div>
          ) : apps.length === 0 ? (
            <div className="flex items-center justify-center h-64">
              <div className="text-center">
                <h3 className="text-lg font-medium mb-2">No apps available</h3>
                <p className="text-sm text-text-muted">
                  Install MCP servers that provide UI resources to see apps here.
                </p>
              </div>
            </div>
          ) : (
            <GridLayout>
              {apps.map((app) => (
                <div
                  key={`${app.uri}-${app.mcpServer}`}
                  className="flex flex-col p-4 border border-border-muted rounded-lg bg-background-panel hover:border-border-default transition-colors"
                >
                  <div className="flex-1 mb-4">
                    <h3 className="font-medium text-text-default mb-2">{app.name}</h3>
                    {app.description && (
                      <p className="text-sm text-text-muted mb-2">{app.description}</p>
                    )}
                    {app.mcpServer && (
                      <span className="inline-block px-2 py-1 text-xs bg-background-subtle text-text-muted rounded">
                        {app.mcpServer}
                      </span>
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
                  </div>
                </div>
              ))}
            </GridLayout>
          )}
        </div>
      </div>
    </MainPanelLayout>
  );
}
