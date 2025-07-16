import React from 'react';
import { Button } from './ui/button';
import { AlertTriangle } from 'lucide-react';

// Capture unhandled promise rejections
window.addEventListener('unhandledrejection', (event) => {
  window.electron.logInfo(`[UNHANDLED REJECTION] ${event.reason}`);
});

// Capture global errors
window.addEventListener('error', (event) => {
  window.electron.logInfo(
    `[GLOBAL ERROR] ${event.message} at ${event.filename}:${event.lineno}:${event.colno}`
  );
});

export function ErrorUI({ error }: { error: Error }) {
  return (
    <div className="fixed inset-0 w-full h-full flex flex-col items-center justify-center gap-6 bg-background">
      <div className="flex flex-col items-center gap-4 max-w-[600px] text-center px-6">
        <div className="w-16 h-16 rounded-full bg-destructive/10 flex items-center justify-center mb-2">
          <AlertTriangle className="w-8 h-8 text-destructive" />
        </div>

        <h1 className="text-2xl font-semibold text-foreground dark:text-white">Honk!</h1>

        {window?.appConfig?.get('GOOSE_VERSION') !== undefined ? (
          <p className="text-base text-textSubtle dark:text-muted-foreground mb-2">
            An error occurred in Goose v{window?.appConfig?.get('GOOSE_VERSION') as string}.
          </p>
        ) : (
          <p className="text-base text-textSubtle dark:text-muted-foreground mb-2">
            An error occurred.
          </p>
        )}

        <pre className="text-destructive text-sm dark:text-white p-4 bg-muted rounded-lg w-full overflow-auto border border-border">
          {error.message}
        </pre>

        <Button onClick={() => window.electron.reloadApp()}>Reload</Button>
      </div>
    </div>
  );
}

export class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null; hasError: boolean }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Send error to main process
    window.electron.logInfo(`[ERROR] ${error.toString()}\n${errorInfo.componentStack}`);
  }

  render() {
    if (this.state.hasError) {
      return <ErrorUI error={this.state.error || new Error('Unknown error')} />;
    }
    return this.props.children;
  }
}
