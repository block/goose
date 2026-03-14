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

interface DeviceCodeModalProps {
  isOpen: boolean;
  deviceCodeData?: DeviceCodeResponse;
  onComplete: () => void;
  onCancel: () => void;
  onRetry: () => void;
}

export function DeviceCodeModal({
  isOpen,
  deviceCodeData,
  onComplete,
  onCancel,
  onRetry,
}: DeviceCodeModalProps) {
  const [copied, setCopied] = useState(false);
  const browserOpenedRef = useRef(false);

  useEffect(() => {
    if (isOpen && deviceCodeData && !browserOpenedRef.current) {
      window.open(deviceCodeData.verificationUri, '_blank');
      browserOpenedRef.current = true;
    }
    if (!isOpen) {
      browserOpenedRef.current = false;
    }
  }, [isOpen, deviceCodeData]);

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
            A browser window has been opened to the GitHub authorization page. After entering the
            code, click <span className="font-medium text-text-default">Complete Setup</span> below.
          </p>
        </div>

        <DialogFooter className="pt-2 shrink-0">
          <Button
            variant="outline"
            onClick={onRetry}
            className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
          >
            Refresh
          </Button>
          <Button
            onClick={onComplete}
            className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
          >
            Complete Setup
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
