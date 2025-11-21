import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '../../ui/dialog';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { QRCodeSVG } from 'qrcode.react';
import { Loader2, Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';
import { errorMessage } from '../../../utils/conversionUtils';
import { startTunnel, stopTunnel, getTunnelStatus } from '../../../api/sdk.gen';
import type { TunnelInfo } from '../../../api/types.gen';

const STATUS_MESSAGES = {
  idle: 'Tunnel is not running',
  starting: 'Starting tunnel...',
  running: 'Tunnel is active',
  error: 'Tunnel encountered an error',
} as const;

export default function TunnelSection() {
  const [tunnelInfo, setTunnelInfo] = useState<TunnelInfo>({
    state: 'idle',
    url: '',
    hostname: '',
    secret: '',
  });
  const [showQRModal, setShowQRModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [copiedSecret, setCopiedSecret] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  useEffect(() => {
    const loadTunnelInfo = async () => {
      try {
        const { data } = await getTunnelStatus();
        if (data) {
          setTunnelInfo(data);
        }
      } catch (err) {
        const errorMsg = errorMessage(err, 'Failed to load tunnel status');
        setError(errorMsg);
        setTunnelInfo({ state: 'error', url: '', hostname: '', secret: '' });
      }
    };

    loadTunnelInfo();
  }, []);

  const handleToggleTunnel = async () => {
    if (tunnelInfo.state === 'running') {
      try {
        await stopTunnel();
        setTunnelInfo({ state: 'idle', url: '', hostname: '', secret: '' });
        setShowQRModal(false);
      } catch (err) {
        setError(errorMessage(err, 'Failed to stop tunnel'));
        try {
          const { data } = await getTunnelStatus();
          if (data) {
            setTunnelInfo(data);
          }
        } catch (statusErr) {
          console.error('Failed to fetch tunnel status after stop error:', statusErr);
        }
      }
    } else {
      setError(null);
      setTunnelInfo({ state: 'starting', url: '', hostname: '', secret: '' });

      try {
        const { data } = await startTunnel();
        if (data) {
          setTunnelInfo(data);
          setShowQRModal(true);
        }
      } catch (err) {
        const errorMsg = errorMessage(err, 'Failed to start tunnel');
        setError(errorMsg);
        setTunnelInfo({ state: 'error', url: '', hostname: '', secret: '' });
      }
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

  const getQRCodeData = () => {
    if (tunnelInfo.state !== 'running') return '';

    const configJson = JSON.stringify({
      url: tunnelInfo.url,
      secret: tunnelInfo.secret,
    });
    const urlEncodedConfig = encodeURIComponent(configJson);
    return `goosechat://configure?data=${urlEncodedConfig}`;
  };

  if (!process.env.GOOSE_TUNNEL) {
    return null;
  }

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Remote Access</CardTitle>
          <CardDescription>
            Enable remote access to goose from mobile devices using secure tunneling via Cloudflare
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4 space-y-4">
          {error && (
            <div className="p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-800 rounded text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}

          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Tunnel Status</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                {STATUS_MESSAGES[tunnelInfo.state]}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {tunnelInfo.state === 'starting' ? (
                <Button disabled variant="secondary" size="sm">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Starting...
                </Button>
              ) : tunnelInfo.state === 'running' ? (
                <>
                  <Button onClick={() => setShowQRModal(true)} variant="default" size="sm">
                    Show QR Code
                  </Button>
                  <Button onClick={handleToggleTunnel} variant="destructive" size="sm">
                    Stop Tunnel
                  </Button>
                </>
              ) : (
                <Button onClick={handleToggleTunnel} variant="default" size="sm">
                  {tunnelInfo.state === 'error' ? 'Retry' : 'Start Tunnel'}
                </Button>
              )}
            </div>
          </div>

          {tunnelInfo.state === 'running' && (
            <div className="p-3 bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-800 rounded">
              <p className="text-xs text-green-800 dark:text-green-200">
                <strong>URL:</strong> {tunnelInfo.url}
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={showQRModal} onOpenChange={setShowQRModal}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Remote Access Connection</DialogTitle>
          </DialogHeader>

          {tunnelInfo.state === 'running' && (
            <div className="py-4 space-y-4">
              <div className="flex justify-center">
                <div className="p-4 bg-white rounded-lg">
                  <QRCodeSVG value={getQRCodeData()} size={200} />
                </div>
              </div>

              <div className="text-center text-sm text-text-muted">
                Scan this QR code with the goose mobile app. Do not share this code with anyone else
                as it is for your personal access.
              </div>

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
                        <code className="flex-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs break-all overflow-hidden">
                          {tunnelInfo.url}
                        </code>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="flex-shrink-0"
                          onClick={() => tunnelInfo.url && copyToClipboard(tunnelInfo.url, 'url')}
                        >
                          {copiedUrl ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                        </Button>
                      </div>
                    </div>

                    <div>
                      <h3 className="text-xs font-medium mb-1 text-text-muted">Secret Key</h3>
                      <div className="flex items-center gap-2">
                        <code className="flex-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs break-all overflow-hidden">
                          {tunnelInfo.secret}
                        </code>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="flex-shrink-0"
                          onClick={() =>
                            tunnelInfo.secret && copyToClipboard(tunnelInfo.secret, 'secret')
                          }
                        >
                          {copiedSecret ? (
                            <Check className="h-4 w-4" />
                          ) : (
                            <Copy className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowQRModal(false)}>
              Close
            </Button>
            <Button variant="destructive" onClick={handleToggleTunnel}>
              Stop Tunnel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
