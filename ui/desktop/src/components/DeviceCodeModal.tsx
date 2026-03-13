import { useState, useEffect } from 'react';
import { X, Copy, ExternalLink } from 'lucide-react';
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

  useEffect(() => {
    if (deviceCodeData) {
      // Open browser to verification URL
      window.open(deviceCodeData.verificationUri, '_blank');
    }
  }, [deviceCodeData]);

  const handleCopy = async () => {
    if (deviceCodeData) {
      await navigator.clipboard.writeText(deviceCodeData.userCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleComplete = () => {
    onComplete();
  };

  if (!isOpen || !deviceCodeData) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm" onClick={onCancel} />
      <div className="relative bg-background-secondary rounded-lg shadow-xl max-w-md w-full p-6">
        <button
          onClick={onCancel}
          className="absolute top-4 right-4 text-text-muted hover:text-text-default transition-colors"
        >
          <X className="w-5 h-5" />
        </button>

        <div className="text-center space-y-4">
          <div className="w-12 h-12 bg-blue-600/10 rounded-full flex items-center justify-center mx-auto">
            <svg className="w-6 h-6 text-blue-600" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .316.192.69.796.577 4.765-1.872 8-6.269 8-12.77 0-6.627-5.373-12-12-12z" />
            </svg>
          </div>

          <div>
            <h3 className="text-lg font-medium text-text-default">GitHub Copilot Setup</h3>
            <p className="text-text-secondary text-sm mt-1">Enter the code below to authorize</p>
          </div>

          <div className="bg-background-tertiary rounded-lg p-4 space-y-3">
            <div>
              <label className="text-xs text-text-muted uppercase tracking-wide font-medium">
                Device Code
              </label>
              <div className="flex items-center gap-2 mt-1">
                <code className="flex-1 text-center text-2xl font-mono tracking-wider text-text-default bg-background-primary rounded px-4 py-3">
                  {deviceCodeData.userCode}
                </code>
                <button
                  onClick={handleCopy}
                  className="p-2 text-text-muted hover:text-text-default transition-colors bg-background-primary rounded"
                  title={copied ? 'Copied!' : 'Copy code'}
                >
                  {copied ? (
                    <svg
                      className="w-5 h-5 text-green-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M5 13l4 4L19 7"
                      />
                    </svg>
                  ) : (
                    <Copy className="w-5 h-5" />
                  )}
                </button>
              </div>
            </div>

            <button
              onClick={() => window.open(deviceCodeData.verificationUri, '_blank')}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 text-sm text-blue-600 hover:text-blue-500 transition-colors bg-background-primary rounded"
            >
              <ExternalLink className="w-4 h-4" />
              Open GitHub in browser
            </button>
          </div>

          <p className="text-text-muted text-xs">
            After entering the code on GitHub, click the button below to complete the setup.
          </p>

          <div className="flex gap-2 pt-2">
            <button
              onClick={onRetry}
              className="flex-1 px-4 py-2 text-sm font-medium text-text-secondary hover:text-text-default bg-background-tertiary hover:bg-background-primary rounded transition-colors"
            >
              Refresh
            </button>
            <button
              onClick={handleComplete}
              className="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded transition-colors"
            >
              Complete Setup
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
