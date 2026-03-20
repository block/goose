import { useState, useEffect, useRef } from 'react';
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

const POLL_INTERVAL_MS = 5000;
const POLL_TIMEOUT_MS = 180000;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
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
  const authorizedRef = useRef(false);
  const onAuthorizedRef = useRef(onAuthorized);
  const onRetryRef = useRef(onRetry);
  onAuthorizedRef.current = onAuthorized;
  onRetryRef.current = onRetry;

  useEffect(() => {
    if (!isOpen || !deviceCodeData) {
      browserOpenedRef.current = false;
      authorizedRef.current = false;
      return;
    }

    if (!browserOpenedRef.current) {
      window.open(deviceCodeData.verificationUri, '_blank');
      browserOpenedRef.current = true;
    }

    let active = true;
    authorizedRef.current = false;
    setIsChecking(true);
    setError(null);

    const poll = async () => {
      const startTime = Date.now();

      while (active) {
        if (Date.now() - startTime > POLL_TIMEOUT_MS) {
          if (active) {
            setIsChecking(false);
            setError('Authorization timed out. Please refresh the code or try again.');
          }
          return;
        }

        try {
          const response = await checkOauthCompletion({
            path: { name: 'github_copilot' },
          });

          console.log('[DeviceCodeModal] poll response:', JSON.stringify(response.data), 'error:', JSON.stringify((response as any).error));

          if (response.data?.completed && active) {
            authorizedRef.current = true;
            setIsChecking(false);
            setError(null);
            onAuthorizedRef.current();
            return;
          }
        } catch (err) {
          console.error('[DeviceCodeModal] poll exception:', err);
        }

        if (active) {
          await sleep(POLL_INTERVAL_MS);
        }
      }
    };

    poll();

    return () => {
      active = false;
    };
  }, [isOpen, deviceCodeData]);

  const handleRetry = () => {
    onRetryRef.current();
  };

  const handleCopy = async () => {
    if (deviceCodeData) {
      await navigator.clipboard.writeText(deviceCodeData.userCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && !authorizedRef.current && onCancel()}>
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
              onClick={handleRetry}
              className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
            >
              Refresh Code
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}
