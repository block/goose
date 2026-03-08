import React from 'react';

interface ElementErrorBoundaryProps {
  elementId?: string;
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface ElementErrorBoundaryState {
  error: Error | null;
}

/**
 * Per-element error boundary for generative UI.
 * Isolates rendering failures so one bad element doesn't crash the entire spec.
 */
export class ElementErrorBoundary extends React.Component<
  ElementErrorBoundaryProps,
  ElementErrorBoundaryState
> {
  state: ElementErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ElementErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(
      `[GenUI] Element "${this.props.elementId ?? 'unknown'}" crashed:`,
      error,
      info.componentStack
    );
  }

  render() {
    if (this.state.error) {
      if (this.props.fallback) return this.props.fallback;

      return (
        <div
          role="alert"
          className="rounded-md border border-border-danger bg-background-danger/10 px-3 py-2 text-sm text-text-danger"
        >
          <span className="font-medium">Render error</span>
          {this.props.elementId && (
            <span className="text-text-muted"> in "{this.props.elementId}"</span>
          )}
          <span className="text-text-muted">: {this.state.error.message}</span>
        </div>
      );
    }

    return this.props.children;
  }
}
