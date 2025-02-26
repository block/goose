import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ModelProvider } from './components/settings/models/ModelContext';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ActiveKeysProvider } from './components/settings/api_keys/ActiveKeysContext';
import { patchConsoleLogging } from './utils';

patchConsoleLogging();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ModelProvider>
      <ActiveKeysProvider>
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </ActiveKeysProvider>
    </ModelProvider>
  </React.StrictMode>
);
