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

  // Theme management - same logic as MoreMenu.tsx
  const [themeMode, _] = useState<'light' | 'dark' | 'system'>(() => {
    const savedUseSystemTheme = localStorage.getItem('use_system_theme') === 'true';
    if (savedUseSystemTheme) {
      return 'system';
    }
    const savedTheme = localStorage.getItem('theme');
    return savedTheme === 'dark' ? 'dark' : 'light';
  });

  const [isDarkMode, setDarkMode] = useState(() => {
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    if (themeMode === 'system') {
      return systemPrefersDark;
    }
    return themeMode === 'dark';
  });

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleThemeChange = (e: { matches: boolean }) => {
      if (themeMode === 'system') {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);

    if (themeMode === 'system') {
      setDarkMode(mediaQuery.matches);
    } else {
      setDarkMode(themeMode === 'dark');
    }

    return () => mediaQuery.removeEventListener('change', handleThemeChange);
  }, [themeMode]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
      document.documentElement.classList.remove('light');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.classList.add('light');
    }
  }, [isDarkMode]);

  useEffect(() => {
    const generateConnectionString = () => {
      try {
        const config = window.electron.getConfig();
        const port = config.GOOSE_PORT as number;
        const secretKey = config.secretKey as string;

        if (!port || !secretKey) {
          throw new Error('Missing port or secret key configuration');
        }

        // Create the simple connection string: 127.0.0.1:PORT:SECRET
        const connectionStr = `127.0.0.1:${port}:${secretKey}`;
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
      <div className="flex flex-col h-screen bg-bgApp">
        <div className="titlebar-drag-region h-8" />
        <div className="flex-1 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col h-screen bg-bgApp">
        <div className="titlebar-drag-region h-8" />
        <div className="flex-1 flex items-center justify-center p-6">
          <div className="text-center">
            <div className="text-red-500 text-lg mb-2">Error</div>
            <div className="text-textStandard mb-4">{error}</div>
            <button
              onClick={handleClose}
              className="px-4 py-2 text-sm hover:bg-bgSubtle transition-colors border border-borderSubtle rounded"
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
    <div className="flex flex-col h-screen bg-bgApp">
      <div className="titlebar-drag-region h-8" />

      <div className="flex-1 flex items-center justify-center p-6">
        <div className="max-w-md w-full">
          {/* Header */}
          <div className="text-center mb-6">
            <div className="flex items-center justify-center mb-3">
              <div className="p-2 rounded-full border border-borderSubtle">
                <Server className="w-6 h-6 text-textStandard" />
              </div>
            </div>
            <h1 className="text-xl font-medium text-textStandard mb-1">Share Developer Agent</h1>
            <p className="text-sm text-textSubtle">
              Your developer agent is running and ready to be shared.
            </p>
          </div>

          {/* Connection String */}
          <div className="border border-borderSubtle rounded-lg overflow-hidden">
            <div className="flex items-center">
              <div className="flex-1 px-3 py-3 font-mono text-sm text-textStandard bg-bgApp select-all break-all">
                {maskedConnectionString}
              </div>
              <button
                onClick={handleCopy}
                className="p-3 hover:bg-bgSubtle transition-colors border-l border-borderSubtle"
                title="Copy to clipboard"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-500" />
                ) : (
                  <Copy className="w-4 h-4 text-textSubtle" />
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
