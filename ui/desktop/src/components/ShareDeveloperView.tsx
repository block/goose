import React, { useState, useEffect } from 'react';
import { Copy, Check, Server, Play, CheckCircle, XCircle, Loader2 } from 'lucide-react';

interface ShareDeveloperViewProps {
  onClose?: () => void;
}

interface ToolCallInfo {
  tool_name: string;
  request_id: string;
  status: 'Running' | 'Completed' | 'Failed';
  started_at: number;
  completed_at?: number;
  error?: string;
}

const ShareDeveloperView: React.FC<ShareDeveloperViewProps> = ({ onClose }) => {
  const [copied, setCopied] = useState(false);
  const [connectionString, setConnectionString] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toolCallHistory, setToolCallHistory] = useState<ToolCallInfo[]>([]);

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

  // Poll for tool call status
  useEffect(() => {
    const pollToolCallStatus = async () => {
      try {
        const config = window.electron.getConfig();
        const port = config.GOOSE_PORT as number;
        const secretKey = config.secretKey as string;

        if (!port || !secretKey) {
          return;
        }

        const response = await fetch(`http://127.0.0.1:${port}/agent/tool_call_status`, {
          headers: {
            'X-Secret-Key': secretKey,
          },
        });

        if (response.ok) {
          const data = await response.json();
          if (data) {
            setToolCallHistory((prevHistory) => {
              // Check if this tool call already exists in history
              const existingIndex = prevHistory.findIndex(
                (call) => call.request_id === data.request_id
              );

              if (existingIndex >= 0) {
                // Update existing tool call
                const updatedHistory = [...prevHistory];
                updatedHistory[existingIndex] = data;
                return updatedHistory;
              } else {
                // Add new tool call and keep only the most recent 3
                const newHistory = [data, ...prevHistory].slice(0, 3);
                return newHistory;
              }
            });
          }
        }
      } catch (err) {
        console.error('Error polling tool call status:', err);
      }
    };

    // Initial poll
    pollToolCallStatus();

    // Set up polling interval (every 1 second)
    const interval = setInterval(pollToolCallStatus, 1000);

    return () => clearInterval(interval);
  }, []);

  const renderStatusIcon = (status: 'Running' | 'Completed' | 'Failed') => {
    switch (status) {
      case 'Running':
        return <Loader2 className="w-4 h-4 text-blue-500 animate-spin" />;
      case 'Completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'Failed':
        return <XCircle className="w-4 h-4 text-red-500" />;
      default:
        return <Play className="w-4 h-4 text-gray-500" />;
    }
  };

  const formatToolName = (toolName: string) => {
    // Remove common prefixes and format nicely
    return toolName
      .replace(/^(platform__|developer__)/, '')
      .replace(/_/g, ' ')
      .replace(/\b\w/g, (l) => l.toUpperCase());
  };

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
            <h1 className="text-xl font-medium text-textStandard mb-1">Share MCPs</h1>
            <p className="text-sm text-textSubtle">
              Your local MCPs are running and ready to be shared.
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

          {/* Tool Call History */}
          {toolCallHistory.length > 0 && (
            <div className="mt-4">
              <h3 className="text-sm font-medium text-textStandard mb-2">Recent Tool Calls</h3>
              <div className="max-h-32 overflow-y-auto space-y-2 pr-1">
                {toolCallHistory.map((toolCall, index) => (
                  <div
                    key={`${toolCall.request_id}-${index}`}
                    className="p-3 border border-borderSubtle rounded-lg bg-bgSubtle"
                  >
                    <div className="flex items-center gap-2">
                      {renderStatusIcon(toolCall.status)}
                      <span className="text-sm text-textStandard truncate">
                        {formatToolName(toolCall.tool_name)}
                      </span>
                      <span className="text-xs text-textSubtle">
                        {toolCall.status === 'Running'
                          ? 'running...'
                          : toolCall.status === 'Completed'
                            ? 'completed'
                            : 'failed'}
                      </span>
                    </div>
                    {toolCall.error && (
                      <div className="mt-2 text-xs text-red-500 break-words">{toolCall.error}</div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default ShareDeveloperView;
