import React from 'react';

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

export class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(_: Error) {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Send error to main process
    window.electron.logInfo(`[ERROR] ${error.toString()}\n${errorInfo.componentStack}`);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="fixed inset-0 w-full h-full flex items-center justify-center bg-background">
          <h1 className="text-xl font-semibold text-foreground">Something went wrong.</h1>
        </div>
      );
    }

    return this.props.children;
  }
}
