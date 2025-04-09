import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { ModelProvider } from './components/settings/models/ModelContext';
import { ConfigProvider } from './components/ConfigContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ActiveKeysProvider } from './components/settings/api_keys/ActiveKeysContext';
import { patchConsoleLogging } from './utils';
import SuspenseLoader from './suspense-loader';

patchConsoleLogging();

const App = lazy(() => import('./App'));

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Suspense fallback={SuspenseLoader()}>
      <ConfigProvider>
        <ModelProvider>
          <ActiveKeysProvider>
            <ErrorBoundary>
              <App />
            </ErrorBoundary>
          </ActiveKeysProvider>
        </ModelProvider>
      </ConfigProvider>
    </Suspense>
  </React.StrictMode>
);
