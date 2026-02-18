import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from './components/ConfigContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import SuspenseLoader from './suspense-loader';
import { client } from './api/client.gen';
import { setTelemetryEnabled } from './utils/analytics';
import { readConfig } from './api';

const App = lazy(() => import('./App'));

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

(async () => {
  // Check if we're in the launcher view (doesn't need goosed connection)
  const isLauncher = window.location.hash === '#/launcher';

  if (!isLauncher) {
    const gooseApiHost = await window.electron.getGoosedHostPort();
    if (gooseApiHost === null) {
      window.alert('failed to start goose backend process');
      return;
    }
    client.setConfig({
      baseUrl: gooseApiHost,
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': await window.electron.getSecretKey(),
      },
    });

    try {
      const telemetryResponse = await readConfig({
        body: { key: TELEMETRY_CONFIG_KEY, is_secret: false },
      });
      const isTelemetryEnabled = telemetryResponse.data !== false;
      setTelemetryEnabled(isTelemetryEnabled);
    } catch (error) {
      console.warn('[Analytics] Failed to initialize analytics:', error);
    }
  }

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <Suspense fallback={SuspenseLoader()}>
        <ConfigProvider>
          <ErrorBoundary>
            <App />
          </ErrorBoundary>
        </ConfigProvider>
      </Suspense>
    </React.StrictMode>
  );
})();
