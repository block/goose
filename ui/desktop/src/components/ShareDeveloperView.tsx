import React, { useState, useEffect } from 'react';
import { Copy, Check, Server } from 'lucide-react';

interface ShareDeveloperViewProps {
  onClose?: () => void;
}

const ShareDeveloperView: React.FC<ShareDeveloperViewProps> = ({ onClose }) => {
  const [copied, setCopied] = useState(false);
  const [connectionString, setConnectionString] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const generateConnectionString = () => {
      try {
        const config = window.electron.getConfig();
        console.log('ShareDeveloperView config:', config);

        const port = config.GOOSE_PORT as number;
        const secretKey = config.secretKey as string;

        console.log(
          'ShareDeveloperView - port:',
          port,
          'secretKey:',
          secretKey ? '[REDACTED]' : 'undefined'
        );

        if (!port || !secretKey) {
          console.error('Missing configuration:', { port, secretKey });
          throw new Error('Missing port or secret key configuration');
        }

        // Create the simple connection string: 127.0.0.1:PORT:SECRET
        const connectionStr = `127.0.0.1:${port}:${secretKey}`;
        console.log(
          'ShareDeveloperView - Generated connection string:',
          `127.0.0.1:${port}:[REDACTED]`
        );
        setConnectionString(connectionStr);
        setIsLoading(false);
      } catch (err) {
        console.error('Error generating connection string:', err);
        setError(err instanceof Error ? err.message : 'Failed to generate connection string');
        setIsLoading(false);
      }
    };

    generateConnectionString();
  }, []);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(connectionString);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const handleClose = () => {
    if (onClose) {
      onClose();
    } else {
      window.close();
    }
  };

  if (isLoading) {
    return (
      <div className="flex flex-col h-screen bg-background-default">
        <div className="titlebar-drag-region h-8" />
        <div className="flex-1 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-text-standard"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col h-screen bg-background-default">
        <div className="titlebar-drag-region h-8" />
        <div className="flex-1 flex items-center justify-center p-6">
          <div className="text-center">
            <div className="text-red-500 text-lg mb-2">Error</div>
            <div className="text-text-standard mb-4">{error}</div>
            <button
              onClick={handleClose}
              className="px-4 py-2 text-sm hover:bg-background-subtle transition-colors border border-border-subtle rounded"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Mask the secret part of the connection string
  const maskedConnectionString = connectionString.replace(/:[^:]+$/, ':••••••••••••••••');

  return (
    <div className="flex flex-col h-screen bg-background-default">
      <div className="titlebar-drag-region h-8" />

      <div className="flex-1 flex items-center justify-center p-6">
        <div className="max-w-md w-full">
          {/* Header */}
          <div className="text-center mb-6">
            <div className="flex items-center justify-center mb-3">
              <div className="p-2 rounded-full border border-border-subtle">
                <Server className="w-6 h-6 text-text-standard" />
              </div>
            </div>
            <h1 className="text-xl font-medium text-text-standard mb-1">Share Developer Agent</h1>
            <div className="space-y-2">
              <p className="text-sm text-text-subtle">
                Your isolated developer agent is running and ready to be shared.
              </p>
              <p className="text-sm text-text-subtle">
                Note: The agent will be terminated when this window is closed.
              </p>
              <p className="text-sm text-text-subtle font-medium">
                Only developer tools are available in this shared instance.
              </p>
            </div>
          </div>

          {/* Connection String */}
          <div className="border border-border-subtle rounded-lg overflow-hidden">
            <div className="flex items-center">
              <div className="flex-1 px-3 py-3 font-mono text-sm text-text-standard bg-background-default select-all break-all">
                {maskedConnectionString}
              </div>
              <button
                onClick={handleCopy}
                className="p-3 hover:bg-background-subtle transition-colors border-l border-border-subtle"
                title="Copy to clipboard"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-500" />
                ) : (
                  <Copy className="w-4 h-4 text-text-subtle" />
                )}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ShareDeveloperView;
