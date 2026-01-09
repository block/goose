import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import McpAppRenderer from '../McpApps/McpAppRenderer';
import { startAgent, resumeAgent, listApps } from '../../api';

export default function StandaloneAppView() {
  const [searchParams] = useSearchParams();
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [cachedHtml, setCachedHtml] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const resourceUri = searchParams.get('resourceUri');
  const extensionName = searchParams.get('extensionName');
  const appName = searchParams.get('appName');
  const workingDir = searchParams.get('workingDir');

  console.log('[StandaloneAppView] Rendering with:', {
    resourceUri,
    extensionName,
    appName,
    workingDir,
    loading,
    error,
    sessionId,
    hasCachedHtml: !!cachedHtml,
  });

  // Load cached HTML immediately
  useEffect(() => {
    async function loadCachedHtml() {
      if (
        !resourceUri ||
        !extensionName ||
        resourceUri === 'undefined' ||
        extensionName === 'undefined'
      ) {
        setError('Missing required parameters');
        setLoading(false);
        return;
      }

      try {
        // Try to get cached app HTML
        const response = await listApps({
          throwOnError: false,
          query: { use_cache: true },
        });

        const apps = response.data?.apps || [];
        const cachedApp = apps.find(
          (app) => app.resourceUri === resourceUri && app.mcpServer === extensionName
        );

        if (cachedApp?.html) {
          setCachedHtml(cachedApp.html);
          setLoading(false);
        }
      } catch (err) {
        console.warn('Failed to load cached HTML:', err);
      }
    }

    loadCachedHtml();
  }, [resourceUri, extensionName]);

  // Initialize session in the background (don't block on it)
  useEffect(() => {
    async function initSession() {
      if (
        !resourceUri ||
        !extensionName ||
        !workingDir ||
        resourceUri === 'undefined' ||
        extensionName === 'undefined'
      ) {
        return;
      }

      try {
        // Create a new session for this standalone app
        const startResponse = await startAgent({
          body: { working_dir: workingDir },
          throwOnError: true,
        });

        const sid = startResponse.data.id;

        // Load all configured extensions (including the MCP server for this app)
        await resumeAgent({
          body: {
            session_id: sid,
            load_model_and_extensions: true,
          },
          throwOnError: true,
        });

        setSessionId(sid);
        setLoading(false);
      } catch (err) {
        console.error('Failed to initialize session:', err);
        // Only set error if we don't have cached HTML to display
        if (!cachedHtml) {
          setError(err instanceof Error ? err.message : 'Failed to initialize session');
          setLoading(false);
        }
      }
    }

    initSession();
  }, [resourceUri, extensionName, workingDir, cachedHtml]);

  // Update window title when app name is available
  useEffect(() => {
    if (appName) {
      document.title = appName;
    }
  }, [appName]);

  if (error && !cachedHtml) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexDirection: 'column',
          gap: '16px',
          padding: '24px',
        }}
      >
        <h2 style={{ color: 'var(--text-error, #ef4444)' }}>Failed to Load App</h2>
        <p style={{ color: 'var(--text-muted, #6b7280)' }}>{error}</p>
      </div>
    );
  }

  if (loading && !cachedHtml) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <p style={{ color: 'var(--text-muted, #6b7280)' }}>Initializing app...</p>
      </div>
    );
  }

  // Show the app if we have either cached HTML or a session ready
  if (cachedHtml || sessionId) {
    return (
      <div style={{ width: '100vw', height: '100vh', overflow: 'hidden' }}>
        <McpAppRenderer
          resourceUri={resourceUri!}
          extensionName={extensionName!}
          sessionId={sessionId || 'loading'} // Pass 'loading' placeholder if session not ready yet
          fullscreen={true}
          cachedHtml={cachedHtml || undefined}
        />
      </div>
    );
  }

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <p style={{ color: 'var(--text-muted, #6b7280)' }}>Initializing app...</p>
    </div>
  );
}
