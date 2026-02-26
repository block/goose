import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { listApps, resumeAgent, startAgent, stopAgent } from '@/api';
import { errorMessage, formatAppName } from '@/utils/conversionUtils';
import McpAppRenderer from '@/components/organisms/mcp-apps/McpAppRenderer';

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
          throwOnError: true,
        });

        const apps = response.data?.apps || [];
        const cachedApp = apps.find(
          (app) => app.uri === resourceUri && app.mcpServers?.includes(extensionName)
        );

        if (cachedApp?.text) {
          setCachedHtml(cachedApp.text);
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
      if (!resourceUri || !extensionName || !workingDir) {
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
          setError(errorMessage(err, 'Failed to initialize session'));
          setLoading(false);
        }
      }
    }

    initSession();
  }, [resourceUri, extensionName, workingDir, cachedHtml]);

  useEffect(() => {
    if (appName) {
      document.title = formatAppName(appName);
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
          sessionId={sessionId || null}
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
