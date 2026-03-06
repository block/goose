import { useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import McpAppRenderer from '../McpApps/McpAppRenderer';
import { startAgent, resumeAgent, listApps, stopAgent } from '../../api';
import { formatAppName } from '../../utils/conversionUtils';
import { errorMessage } from '../../utils/conversionUtils';
import { useLocalization } from '../../contexts/LocalizationContext';

export default function StandaloneAppView() {
  const { t } = useLocalization();
  const [searchParams] = useSearchParams();
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [cachedHtml, setCachedHtml] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const tRef = useRef(t);
  const cachedHtmlRef = useRef<string | null>(null);

  const resourceUri = searchParams.get('resourceUri');
  const extensionName = searchParams.get('extensionName');
  const appName = searchParams.get('appName');
  const workingDir = searchParams.get('workingDir');

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  useEffect(() => {
    cachedHtmlRef.current = cachedHtml;
  }, [cachedHtml]);

  useEffect(() => {
    let cancelled = false;

    async function loadCachedHtml() {
      if (
        !resourceUri ||
        !extensionName ||
        resourceUri === 'undefined' ||
        extensionName === 'undefined'
      ) {
        if (!cancelled) {
          setError(tRef.current('apps.standalone.missingParams'));
          setLoading(false);
        }
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

        if (!cancelled && cachedApp?.text) {
          setCachedHtml(cachedApp.text);
          setLoading(false);
        }
      } catch (err) {
        console.warn('Failed to load cached HTML:', err);
      }
    }

    void loadCachedHtml();

    return () => {
      cancelled = true;
    };
  }, [resourceUri, extensionName]);

  useEffect(() => {
    let cancelled = false;
    let createdSessionId: string | null = null;

    async function initSession() {
      if (!resourceUri || !extensionName || !workingDir || sessionId) {
        return;
      }

      try {
        const startResponse = await startAgent({
          body: { working_dir: workingDir },
          throwOnError: true,
        });

        const sid = startResponse.data.id;
        createdSessionId = sid;

        await resumeAgent({
          body: {
            session_id: sid,
            load_model_and_extensions: true,
          },
          throwOnError: true,
        });

        if (cancelled) {
          await stopAgent({
            body: { session_id: sid },
            throwOnError: false,
          });
          return;
        }

        createdSessionId = null;
        setSessionId(sid);
        setLoading(false);
      } catch (err) {
        console.error('Failed to initialize session:', err);
        if (createdSessionId) {
          stopAgent({
            body: { session_id: createdSessionId },
            throwOnError: false,
          }).catch((stopError: unknown) => {
            console.warn('Failed to stop agent after initialization failure:', stopError);
          });
          createdSessionId = null;
        }
        if (!cancelled && !cachedHtmlRef.current) {
          setError(errorMessage(err, tRef.current('apps.standalone.initializeFailed')));
          setLoading(false);
        }
      }
    }

    void initSession();

    return () => {
      cancelled = true;
      if (createdSessionId) {
        stopAgent({
          body: { session_id: createdSessionId },
          throwOnError: false,
        }).catch((err: unknown) => {
          console.warn('Failed to stop agent after init cleanup:', err);
        });
      }
    };
  }, [resourceUri, extensionName, workingDir, sessionId]);

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
        <h2 style={{ color: 'var(--text-error, #ef4444)' }}>{t('apps.standalone.loadFailedTitle')}</h2>
        <p style={{ color: 'var(--color-text-secondary, #6b7280)' }}>{error}</p>
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
        <p style={{ color: 'var(--color-text-secondary, #6b7280)' }}>
          {t('apps.standalone.initializing')}
        </p>
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
          displayMode="standalone"
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
      <p style={{ color: 'var(--color-text-secondary, #6b7280)' }}>
        {t('apps.standalone.initializing')}
      </p>
    </div>
  );
}
