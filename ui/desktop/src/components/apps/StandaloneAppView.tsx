import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import McpAppRenderer from '../McpApps/McpAppRenderer';
import { startAgent, resumeAgent } from '../../api';

export default function StandaloneAppView() {
  const [searchParams] = useSearchParams();
  const [sessionId, setSessionId] = useState<string | null>(null);
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
  });

  useEffect(() => {
    async function initSession() {
      if (
        !resourceUri ||
        !extensionName ||
        !workingDir ||
        resourceUri === 'undefined' ||
        extensionName === 'undefined'
      ) {
        setError('Missing required parameters');
        setLoading(false);
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
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to initialize session');
      } finally {
        setLoading(false);
      }
    }

    initSession();
  }, [resourceUri, extensionName, workingDir]);

  // Update window title when app name is available
  useEffect(() => {
    if (appName) {
      document.title = appName;
    }
  }, [appName]);

  if (error) {
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

  if (loading || !sessionId) {
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

  return (
    <div style={{ width: '100vw', height: '100vh', overflow: 'hidden' }}>
      <McpAppRenderer
        resourceUri={resourceUri!}
        extensionName={extensionName!}
        sessionId={sessionId}
        fullscreen={true}
      />
    </div>
  );
}
