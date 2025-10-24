import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from './components/ConfigContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import SuspenseLoader from './suspense-loader';
import { client } from './api/client.gen';
import UpdateToast from './components/UpdateToast';

const App = lazy(() => import('./App'));

(async () => {
  console.log('window created, getting goosed connection info');
  const baseUrl = await window.electron.getGoosedHostPort();
  if (baseUrl === null) {
    window.alert('failed to start goose backend process');
    return;
  }
  console.log('connecting at', baseUrl);
  client.setConfig({
    baseUrl,
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': await window.electron.getSecretKey(),
    },
  });

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <Suspense fallback={SuspenseLoader()}>
        <ConfigProvider>
          <ErrorBoundary>
            <App />
            {/* Global update toast — shows a compact restart button when an update is downloaded */}
            <UpdateToast />
          </ErrorBoundary>
        </ConfigProvider>
      </Suspense>
    </React.StrictMode>
  );
})();
