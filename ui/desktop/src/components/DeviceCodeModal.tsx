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
  const pollingRef = useRef(false);

  const startPolling = useCallback(async () => {
    if (pollingRef.current) return;
    pollingRef.current = true;
    setIsChecking(true);
    setError(null);

    // Poll every 5 seconds, up to 3 minutes
    const maxAttempts = 36;
    const interval = 5000;

    for (let attempt = 0; attempt < maxAttempts && pollingRef.current; attempt++) {
      try {
        // Call the completion check endpoint
        const response = await checkOauthCompletion({
          path: { name: 'github_copilot' },
        });

        const data = response.data;

        // If completed is true, authorization succeeded
        if (data?.completed) {
          pollingRef.current = false;
          setIsChecking(false);
          onAuthorized();
          return;
        }
      } catch {
        // Error likely means authorization not complete yet, keep polling
      }

      // Wait before next poll
      await new Promise((resolve) => setTimeout(resolve, interval));
    }

    // Timeout reached
    pollingRef.current = false;
    setIsChecking(false);
    setError('Authorization timed out. Please refresh the code or try again.');
  }, [onAuthorized]);

  useEffect(() => {
    if (isOpen && deviceCodeData && !browserOpenedRef.current) {
      window.open(deviceCodeData.verificationUri, '_blank');
      browserOpenedRef.current = true;
      // Start polling for authorization
      startPolling();
    }
    if (!isOpen) {
      browserOpenedRef.current = false;
      pollingRef.current = false;
      setError(null);
    }
  }, [isOpen, deviceCodeData, startPolling]);

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
              onClick={() => {
                setError(null);
                startPolling();
              }}
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
