import React, { useState, useEffect } from 'react';
import { Button } from '../ui/button';
import { AlertCircle, CheckCircle, Loader2 } from 'lucide-react';

interface MatrixServerConfigProps {
  onServerChange: (homeserverUrl: string) => void;
  currentServer?: string;
}

const RECOMMENDED_SERVERS = [
  {
    name: 'Element.io',
    url: 'https://matrix-client.matrix.org',
    description: 'Official Element homeserver, reliable and fast',
    recommended: true,
  },
  {
    name: 'Tchncs.de',
    url: 'https://matrix.tchncs.de',
    description: 'European privacy-focused server',
    recommended: true,
  },
  {
    name: 'Envs.net',
    url: 'https://matrix.envs.net',
    description: 'Community-run server',
    recommended: false,
  },
  {
    name: 'Custom',
    url: '',
    description: 'Use your own Matrix homeserver',
    recommended: false,
  },
];

interface ServerStatus {
  url: string;
  status: 'checking' | 'online' | 'offline' | 'registration_disabled';
  registrationOpen?: boolean;
  error?: string;
}

export const MatrixServerConfig: React.FC<MatrixServerConfigProps> = ({
  onServerChange,
  currentServer = 'https://matrix-client.matrix.org',
}) => {
  const [selectedServer, setSelectedServer] = useState(currentServer);
  const [customServer, setCustomServer] = useState('');
  const [serverStatuses, setServerStatuses] = useState<Record<string, ServerStatus>>({});
  const [isTestingServers, setIsTestingServers] = useState(false);

  // Test a single server
  const testServer = async (url: string): Promise<ServerStatus> => {
    try {
      // Test basic connectivity
      const versionsResponse = await fetch(`${url}/_matrix/client/versions`, {
        method: 'GET',
        signal: AbortSignal.timeout(5000), // 5 second timeout
      });

      if (!versionsResponse.ok) {
        return {
          url,
          status: 'offline',
          error: `HTTP ${versionsResponse.status}`,
        };
      }

      // Test registration endpoint
      const registerResponse = await fetch(`${url}/_matrix/client/v3/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: 'test', password: 'test' }),
        signal: AbortSignal.timeout(5000),
      });

      const registrationOpen = registerResponse.status !== 403;

      return {
        url,
        status: registrationOpen ? 'online' : 'registration_disabled',
        registrationOpen,
      };
    } catch (error) {
      return {
        url,
        status: 'offline',
        error: error instanceof Error ? error.message : 'Connection failed',
      };
    }
  };

  // Test all recommended servers
  const testAllServers = async () => {
    setIsTestingServers(true);
    const results: Record<string, ServerStatus> = {};

    for (const server of RECOMMENDED_SERVERS) {
      if (server.url) {
        setServerStatuses(prev => ({
          ...prev,
          [server.url]: { url: server.url, status: 'checking' },
        }));

        const result = await testServer(server.url);
        results[server.url] = result;
        
        setServerStatuses(prev => ({
          ...prev,
          [server.url]: result,
        }));
      }
    }

    setIsTestingServers(false);
  };

  // Test servers on component mount
  useEffect(() => {
    testAllServers();
  }, []);

  const handleServerSelect = (url: string) => {
    setSelectedServer(url);
    if (url !== '') {
      onServerChange(url);
    }
  };

  const handleCustomServerChange = (url: string) => {
    setCustomServer(url);
    if (url.trim()) {
      setSelectedServer(url);
      onServerChange(url);
    }
  };

  const getStatusIcon = (status: ServerStatus | undefined) => {
    if (!status) return null;

    switch (status.status) {
      case 'checking':
        return <Loader2 className="w-4 h-4 animate-spin text-blue-500" />;
      case 'online':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'registration_disabled':
        return <AlertCircle className="w-4 h-4 text-yellow-500" />;
      case 'offline':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      default:
        return null;
    }
  };

  const getStatusText = (status: ServerStatus | undefined) => {
    if (!status) return '';

    switch (status.status) {
      case 'checking':
        return 'Testing...';
      case 'online':
        return 'Online & Registration Open';
      case 'registration_disabled':
        return 'Online but Registration Disabled';
      case 'offline':
        return `Offline${status.error ? `: ${status.error}` : ''}`;
      default:
        return '';
    }
  };

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold mb-2">Matrix Homeserver</h3>
        <p className="text-sm text-text-muted mb-4">
          Choose a Matrix homeserver to connect to for peer-to-peer collaboration.
        </p>
      </div>

      <div className="space-y-3">
        {RECOMMENDED_SERVERS.map((server) => {
          if (server.name === 'Custom') {
            return (
              <div key="custom" className="border border-border-default rounded-lg p-4">
                <div className="flex items-center gap-3 mb-2">
                  <input
                    type="radio"
                    id="custom-server"
                    name="matrix-server"
                    checked={selectedServer !== '' && !RECOMMENDED_SERVERS.some(s => s.url === selectedServer)}
                    onChange={() => {}}
                    className="w-4 h-4"
                  />
                  <label htmlFor="custom-server" className="font-medium">
                    Custom Server
                  </label>
                </div>
                <input
                  type="url"
                  placeholder="https://your-matrix-server.com"
                  value={customServer}
                  onChange={(e) => handleCustomServerChange(e.target.value)}
                  className="w-full px-3 py-2 border border-border-default rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm"
                />
                <p className="text-xs text-text-muted mt-1">
                  Enter your own Matrix homeserver URL
                </p>
              </div>
            );
          }

          const status = serverStatuses[server.url];
          const isSelected = selectedServer === server.url;
          const canSelect = status?.status === 'online';

          return (
            <div
              key={server.url}
              className={`border rounded-lg p-4 cursor-pointer transition-all ${
                isSelected
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                  : canSelect
                  ? 'border-border-default hover:border-blue-300'
                  : 'border-border-default opacity-60'
              }`}
              onClick={() => canSelect && handleServerSelect(server.url)}
            >
              <div className="flex items-center gap-3 mb-2">
                <input
                  type="radio"
                  id={server.url}
                  name="matrix-server"
                  checked={isSelected}
                  onChange={() => canSelect && handleServerSelect(server.url)}
                  disabled={!canSelect}
                  className="w-4 h-4"
                />
                <label htmlFor={server.url} className="font-medium flex items-center gap-2">
                  {server.name}
                  {server.recommended && (
                    <span className="text-xs bg-green-100 text-green-700 px-2 py-1 rounded">
                      Recommended
                    </span>
                  )}
                </label>
                {getStatusIcon(status)}
              </div>
              
              <p className="text-sm text-text-muted mb-1">{server.description}</p>
              <p className="text-xs text-text-muted">{server.url}</p>
              
              {status && (
                <div className="mt-2 flex items-center gap-2">
                  <span className={`text-xs ${
                    status.status === 'online' ? 'text-green-600' :
                    status.status === 'registration_disabled' ? 'text-yellow-600' :
                    status.status === 'offline' ? 'text-red-600' :
                    'text-blue-600'
                  }`}>
                    {getStatusText(status)}
                  </span>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex items-center gap-2 pt-4 border-t border-border-default">
        <Button
          onClick={testAllServers}
          disabled={isTestingServers}
          variant="outline"
          size="sm"
        >
          {isTestingServers ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin mr-2" />
              Testing Servers...
            </>
          ) : (
            'Test All Servers'
          )}
        </Button>
        
        <div className="text-xs text-text-muted">
          Last tested: {new Date().toLocaleTimeString()}
        </div>
      </div>

      {selectedServer && serverStatuses[selectedServer]?.status === 'online' && (
        <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-3">
          <div className="flex items-center gap-2">
            <CheckCircle className="w-4 h-4 text-green-500" />
            <span className="text-sm font-medium text-green-700 dark:text-green-300">
              Ready to connect!
            </span>
          </div>
          <p className="text-xs text-green-600 dark:text-green-400 mt-1">
            You can now create an account and start collaborating.
          </p>
        </div>
      )}
    </div>
  );
};
