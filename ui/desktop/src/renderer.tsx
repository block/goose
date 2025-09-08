import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from './components/ConfigContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import SuspenseLoader from './suspense-loader';
import { client } from './api/client.gen';

const App = lazy(() => import('./App'));

(async () => {
  client.setConfig({
    baseUrl: window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT'),
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
          </ErrorBoundary>
        </ConfigProvider>
      </Suspense>
    </React.StrictMode>
  );
})();
