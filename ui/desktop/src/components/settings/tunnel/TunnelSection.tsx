import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '../../ui/dialog';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { QRCodeSVG } from 'qrcode.react';
import { Loader2, Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';

interface TunnelInfo {
  url: string;
  ipv4: string;
  ipv6: string;
  hostname: string;
  secret: string;
  port: number;
}

interface TunnelStatus {
  state: 'idle' | 'starting' | 'running' | 'error';
  info: TunnelInfo | null;
}

type TunnelMode = 'lapstone' | 'tailscale';

export default function TunnelSection() {
  const [tunnelStatus, setTunnelStatus] = useState<TunnelStatus>({ state: 'idle', info: null });
  const [showQRModal, setShowQRModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [copiedSecret, setCopiedSecret] = useState(false);
  const [showDetails, setShowDetails] = useState(false);
  const [tunnelMode, setTunnelMode] = useState<TunnelMode>('lapstone');

  // Load tunnel mode preference on mount
  useEffect(() => {
    const loadTunnelMode = async () => {
      try {
        const mode = await window.electron.getTunnelMode();
        setTunnelMode(mode as TunnelMode);
      } catch (err) {
        console.error('Error loading tunnel mode:', err);
      }
    };
    loadTunnelMode();
  }, []);

  // Check tunnel status on mount
  useEffect(() => {
    checkTunnelStatus();
  }, []);

  const checkTunnelStatus = async () => {
    try {
      const status = await window.electron.getTunnelStatus();
      setTunnelStatus(status as TunnelStatus);
    } catch (err) {
      console.error('Error checking tunnel status:', err);
    }
  };

  const handleStartTunnel = async () => {
    setError(null);
    setTunnelStatus({ state: 'starting', info: null });

    try {
      const tunnelInfo = await window.electron.startTunnel();
      setTunnelStatus({ state: 'running', info: tunnelInfo as TunnelInfo });
      setShowQRModal(true);
    } catch (err) {
      console.error('Error starting tunnel:', err);
      setError(err instanceof Error ? err.message : 'Failed to start tunnel');
      setTunnelStatus({ state: 'error', info: null });
    }
  };

  const handleStopTunnel = async () => {
    try {
      await window.electron.stopTunnel();
      setTunnelStatus({ state: 'idle', info: null });
      setShowQRModal(false);
    } catch (err) {
      console.error('Error stopping tunnel:', err);
      setError(err instanceof Error ? err.message : 'Failed to stop tunnel');
    }
  };

  const handleTunnelModeChange = async (mode: TunnelMode) => {
    try {
      await window.electron.setTunnelMode(mode);
      setTunnelMode(mode);
    } catch (err) {
      console.error('Error setting tunnel mode:', err);
    }
  };

  const copyToClipboard = async (text: string, type: 'url' | 'secret') => {
    try {
      await navigator.clipboard.writeText(text);
      if (type === 'url') {
        setCopiedUrl(true);
        setTimeout(() => setCopiedUrl(false), 2000);
      } else {
        setCopiedSecret(true);
        setTimeout(() => setCopiedSecret(false), 2000);
      }
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  // Generate QR code data
  const getQRCodeData = () => {
    if (!tunnelStatus.info) return '';

    // Use the actual tunnel URL (works for both Tailscale and Lapstone)
    // For Tailscale: uses ipv4 if available, otherwise falls back to url
    // For Lapstone: always uses the public Cloudflare URL
    const tunnelUrl = tunnelStatus.info.ipv4
      ? `http://${tunnelStatus.info.ipv4}:${tunnelStatus.info.port}`
      : tunnelStatus.info.url;

    const configJson = JSON.stringify({
      url: tunnelUrl,
      secret: tunnelStatus.info.secret,
    });
    const urlEncodedConfig = encodeURIComponent(configJson);
    return `goosechat://configure?data=${urlEncodedConfig}`;
  };

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Remote Access</CardTitle>
          <CardDescription>
            Enable remote access to goose via Tailscale tunnel for mobile devices
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4 space-y-4">
          {error && (
            <div className="p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-800 rounded text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}

          {/* Tunnel Mode Selector - only shown when tunnel is not running */}
          {tunnelStatus.state === 'idle' && (
            <div className="space-y-2">
              <h3 className="text-text-default text-xs">Tunnel Mode</h3>
              <div className="flex flex-col space-y-2">
                <button
                  onClick={() => handleTunnelModeChange('lapstone')}
                  className={`flex items-center space-x-2 p-2 rounded border transition-colors ${
                    tunnelMode === 'lapstone'
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                >
                  <div
                    className={`w-4 h-4 rounded-full border-2 flex items-center justify-center ${
                      tunnelMode === 'lapstone'
                        ? 'border-blue-500'
                        : 'border-gray-400 dark:border-gray-500'
                    }`}
                  >
                    {tunnelMode === 'lapstone' && (
                      <div className="w-2 h-2 rounded-full bg-blue-500"></div>
                    )}
                  </div>
                  <span className="text-xs font-normal cursor-pointer">
                    Lapstone (Cloudflare Tunnel) - Recommended
                  </span>
                </button>
                <button
                  onClick={() => handleTunnelModeChange('tailscale')}
                  className={`flex items-center space-x-2 p-2 rounded border transition-colors ${
                    tunnelMode === 'tailscale'
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                >
                  <div
                    className={`w-4 h-4 rounded-full border-2 flex items-center justify-center ${
                      tunnelMode === 'tailscale'
                        ? 'border-blue-500'
                        : 'border-gray-400 dark:border-gray-500'
                    }`}
                  >
                    {tunnelMode === 'tailscale' && (
                      <div className="w-2 h-2 rounded-full bg-blue-500"></div>
                    )}
                  </div>
                  <span className="text-xs font-normal cursor-pointer">Tailscale</span>
                </button>
              </div>
            </div>
          )}

          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Tunnel Status</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                {tunnelStatus.state === 'idle' && 'Tunnel is not running'}
                {tunnelStatus.state === 'starting' && 'Starting tunnel...'}
                {tunnelStatus.state === 'running' && 'Tunnel is active'}
                {tunnelStatus.state === 'error' && 'Tunnel encountered an error'}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {tunnelStatus.state === 'idle' && (
                <Button onClick={handleStartTunnel} variant="default" size="sm">
                  Start Tunnel
                </Button>
              )}
              {tunnelStatus.state === 'starting' && (
                <Button disabled variant="secondary" size="sm">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Starting...
                </Button>
              )}
              {tunnelStatus.state === 'running' && (
                <>
                  <Button onClick={() => setShowQRModal(true)} variant="default" size="sm">
                    Show QR Code
                  </Button>
                  <Button onClick={handleStopTunnel} variant="destructive" size="sm">
                    Stop Tunnel
                  </Button>
                </>
              )}
              {tunnelStatus.state === 'error' && (
                <Button onClick={handleStartTunnel} variant="default" size="sm">
                  Retry
                </Button>
              )}
            </div>
          </div>

          {tunnelStatus.state === 'running' && tunnelStatus.info && (
            <div className="p-3 bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-800 rounded">
              <p className="text-xs text-green-800 dark:text-green-200">
                <strong>URL:</strong> {tunnelStatus.info.url}
              </p>
              <p className="text-xs text-green-800 dark:text-green-200 mt-1">
                <strong>Port:</strong> {tunnelStatus.info.port}
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* QR Code Modal */}
      <Dialog open={showQRModal} onOpenChange={setShowQRModal}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Remote Access Connection</DialogTitle>
          </DialogHeader>

          {tunnelStatus.info && (
            <div className="py-4 space-y-4">
              {/* QR Code */}
              <div className="flex justify-center">
                <div className="p-4 bg-white rounded-lg">
                  <QRCodeSVG value={getQRCodeData()} size={200} />
                </div>
              </div>

              <div className="text-center text-sm text-text-muted">
                Scan this QR code with the Goose mobile app
              </div>

              {/* Collapsible Details */}
              <div className="border-t pt-4">
                <button
                  onClick={() => setShowDetails(!showDetails)}
                  className="flex items-center justify-between w-full text-sm font-medium hover:opacity-70 transition-opacity"
                >
                  <span>Connection Details</span>
                  {showDetails ? (
                    <ChevronUp className="h-4 w-4" />
                  ) : (
                    <ChevronDown className="h-4 w-4" />
                  )}
                </button>

                {showDetails && (
                  <div className="mt-3 space-y-3">
                    <div>
                      <h3 className="text-xs font-medium mb-1 text-text-muted">Tunnel URL</h3>
                      <div className="flex items-center gap-2">
                        <code className="flex-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs break-all">
                          {tunnelStatus.info.url}
                        </code>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => copyToClipboard(tunnelStatus.info!.url, 'url')}
                        >
                          {copiedUrl ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                        </Button>
                      </div>
                    </div>

                    <div>
                      <h3 className="text-xs font-medium mb-1 text-text-muted">Secret Key</h3>
                      <div className="flex items-center gap-2">
                        <code className="flex-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs">
                          {tunnelStatus.info.secret}
                        </code>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => copyToClipboard(tunnelStatus.info!.secret, 'secret')}
                        >
                          {copiedSecret ? (
                            <Check className="h-4 w-4" />
                          ) : (
                            <Copy className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <h3 className="text-xs font-medium mb-1 text-text-muted">IPv4</h3>
                        <code className="block p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs">
                          {tunnelStatus.info.ipv4}
                        </code>
                      </div>
                      <div>
                        <h3 className="text-xs font-medium mb-1 text-text-muted">Port</h3>
                        <code className="block p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs">
                          {tunnelStatus.info.port}
                        </code>
                      </div>
                    </div>

                    {tunnelStatus.info.ipv6 && (
                      <div>
                        <h3 className="text-xs font-medium mb-1 text-text-muted">IPv6</h3>
                        <code className="block p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs break-all">
                          {tunnelStatus.info.ipv6}
                        </code>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowQRModal(false)}>
              Close
            </Button>
            <Button variant="destructive" onClick={handleStopTunnel}>
              Stop Tunnel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
