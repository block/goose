import { useState, useEffect, useRef, useCallback } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { Copy } from 'lucide-react';
import type { DeviceCodeResponse } from '../api/types.gen';
import { checkOauthCompletion } from '../api';

interface DeviceCodeModalProps {
  isOpen: boolean;
  deviceCodeData?: DeviceCodeResponse;
  onAuthorized: () => void;
  onCancel: () => void;
  onRetry: () => void;
}

function pollForOauthCompletion(
  onAuthorized: () => void,
  onError: () => void,
  intervalMs = 5000,
  maxDurationMs = 180000
): () => void {
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let isPolling = true;
  const startTime = Date.now();

  const poll = async () => {
    if (!isPolling) return;

    if (Date.now() - startTime > maxDurationMs) {
      stopPolling();
      onError();
      return;
    }

    try {
      const response = await checkOauthCompletion({
        path: { name: 'github_copilot' },
      });

      if (response.data?.completed) {
        stopPolling();
        onAuthorized();
      }
    } catch {
      // Error likely means authorization not complete yet, keep polling
    }
  };

  const stopPolling = () => {
    isPolling = false;
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  };

  // Start polling immediately, then at intervals
  poll();
  intervalId = setInterval(poll, intervalMs);

  return stopPolling;
}

export function DeviceCodeModal({
  isOpen,
  deviceCodeData,
  onAuthorized,
  onCancel,
  onRetry,
}: DeviceCodeModalProps) {
  const [copied, setCopied] = useState(false);
  const [isChecking, setIsChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const browserOpenedRef = useRef(false);
  const stopPollingRef = useRef<(() => void) | null>(null);

  const handleAuthorized = useCallback(() => {
    setIsChecking(false);
    setError(null);
    onAuthorized();
  }, [onAuthorized]);

  const handleError = useCallback(() => {
    setIsChecking(false);
    setError('Authorization timed out. Please refresh the code or try again.');
  }, []);

  const startPolling = useCallback(() => {
    if (stopPollingRef.current) {
      stopPollingRef.current();
    }
    setIsChecking(true);
    setError(null);
    stopPollingRef.current = pollForOauthCompletion(handleAuthorized, handleError, 5000, 180000);
  }, [handleAuthorized, handleError]);

  const stopPolling = useCallback(() => {
    if (stopPollingRef.current) {
      stopPollingRef.current();
      stopPollingRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (isOpen && deviceCodeData && !browserOpenedRef.current) {
      window.open(deviceCodeData.verificationUri, '_blank');
      browserOpenedRef.current = true;
      // Start polling for authorization
      startPolling();
    }
    if (!isOpen) {
      browserOpenedRef.current = false;
      stopPolling();
      setError(null);
    }
    return () => stopPolling();
  }, [isOpen, deviceCodeData, startPolling, stopPolling]);

  const handleCopy = async () => {
    if (deviceCodeData) {
      await navigator.clipboard.writeText(deviceCodeData.userCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[500px] max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>GitHub Copilot Setup</DialogTitle>
          <DialogDescription>
            Enter the code below on GitHub to authorize your account
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="bg-background-tertiary rounded-lg p-6 text-center">
            <div className="mb-3">
              <label className="text-xs text-text-muted uppercase tracking-wide font-medium">
                Device Code
              </label>
            </div>
            <div className="flex items-center justify-center gap-3">
              <code className="text-3xl font-mono tracking-wider text-text-default px-6 py-4 bg-background-primary rounded border border-background-accent/20">
                {deviceCodeData?.userCode}
              </code>
              <Button
                onClick={handleCopy}
                variant="outline"
                size="xs"
                title={copied ? 'Copied!' : 'Copy code'}
                className="shrink-0"
              >
                {copied ? (
                  <Copy className="w-5 h-5 text-green-600" />
                ) : (
                  <Copy className="w-5 h-5" />
                )}
              </Button>
            </div>
          </div>

          <p className="text-sm text-text-muted text-center">
            {error ? (
              <span className="text-red-500">{error}</span>
            ) : isChecking ? (
              'Waiting for you to authorize on GitHub. Enter the code in the browser, and we will automatically complete setup.'
            ) : (
              'A browser window has been opened. Copy the code above and enter it on GitHub.'
            )}
          </p>
        </div>

        {error && (
          <DialogFooter className="pt-2 shrink-0">
            <Button
              variant="outline"
              onClick={onRetry}
              className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
            >
              Refresh Code
            </Button>
            <Button
              onClick={startPolling}
              className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
            >
              Retry
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}
