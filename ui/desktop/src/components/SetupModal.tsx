import { useEffect } from 'react';
import { Button } from './ui/button';

interface SetupModalProps {
  title: string;
  message: string;
  showProgress?: boolean;
  showRetry?: boolean;
  onRetry?: () => void;
  autoClose?: number;
  onClose?: () => void;
}

export function SetupModal({
  title,
  message,
  showProgress,
  showRetry,
  onRetry,
  autoClose,
  onClose,
}: SetupModalProps) {
  useEffect(() => {
    if (autoClose && onClose) {
      const timer = window.setTimeout(() => {
        onClose();
      }, autoClose);
      return () => window.clearTimeout(timer);
    }
    return undefined;
  }, [autoClose, onClose]);

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full">
        <h2 className="text-xl font-bold mb-4 text-gray-900 dark:text-gray-100">{title}</h2>
        <p className="mb-6 text-gray-700 dark:text-gray-300">{message}</p>

        {showProgress && (
          <div className="flex justify-center mb-4">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
          </div>
        )}

        {onClose && (
          <div className="mb-4">
            <Button onClick={onClose} className="w-full">
              Close
            </Button>
            <br />
          </div>
        )}

        {showRetry && onRetry && (
          <Button onClick={onRetry} className="w-full">
            Retry Setup
          </Button>
        )}
      </div>
    </div>
  );
}
