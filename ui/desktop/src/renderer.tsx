import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from './components/ConfigContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import SuspenseLoader from './suspense-loader';
import { client } from './api/client.gen';

const App = lazy(() => import('./App'));

(async () => {
  // Check if we're in the launcher view (doesn't need goosed connection)
  const isLauncher = window.location.hash === '#/launcher';

  if (!isLauncher) {
    console.log('window created, getting goosed connection info');
    const gooseApiHost = await window.electron.getGoosedHostPort();
    if (gooseApiHost === null) {
      window.alert('failed to start goose backend process');
      return;
    }
    console.log('connecting at', gooseApiHost);
    client.setConfig({
      baseUrl: gooseApiHost,
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': await window.electron.getSecretKey(),
      },
    });
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
