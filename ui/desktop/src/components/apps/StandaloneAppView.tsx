import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import McpAppRenderer from '../McpApps/McpAppRenderer';
import { startAgent, resumeAgent, listApps, stopAgent } from '../../api';

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

  useEffect(() => {
    async function initSession() {
      if (
        !resourceUri ||
        !extensionName ||
        !workingDir ||
        resourceUri === 'undefined' ||
        extensionName === 'undefined' ||
        workingDir === 'undefined' ||
        workingDir === 'null'
      ) {
        return;
      }

      try {
        const startResponse = await startAgent({
          body: { working_dir: workingDir },
          throwOnError: true,
        });

        const sid = startResponse.data.id;

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
        if (!cachedHtml) {
          setError(err instanceof Error ? err.message : 'Failed to initialize session');
          setLoading(false);
        }
      }
    }

    initSession();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resourceUri, extensionName, workingDir]);

  useEffect(() => {
    if (appName) {
      document.title = appName;
    }
  }, [appName]);

  // Cleanup session when component unmounts
  useEffect(() => {
    return () => {
      if (sessionId) {
        stopAgent({
          body: { session_id: sessionId },
          throwOnError: false,
        }).catch((err: unknown) => {
          console.warn('Failed to stop agent on unmount:', err);
        });
      }
    };
  }, [sessionId]);

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

  if (cachedHtml || sessionId) {
    return (
      <div style={{ width: '100vw', height: '100vh', overflow: 'hidden' }}>
        <McpAppRenderer
          resourceUri={resourceUri!}
          extensionName={extensionName!}
          sessionId={sessionId || 'loading'}
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
