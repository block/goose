import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '../../ui/dialog';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { QRCodeSVG } from 'qrcode.react';
import {
  Loader2,
  Copy,
  Check,
  ChevronDown,
  ChevronUp,
  Info,
  ExternalLink,
  QrCode,
} from 'lucide-react';
import { errorMessage } from '../../../utils/conversionUtils';
import { startTunnel, stopTunnel, getTunnelStatus } from '../../../api/sdk.gen';
import type { TunnelInfo } from '../../../api/types.gen';
import { t } from '../../../i18n';

const IOS_APP_STORE_URL = 'https://apps.apple.com/us/app/goose-ai/id6752889295';

export default function TunnelSection() {
  const [tunnelInfo, setTunnelInfo] = useState<TunnelInfo>({
    state: 'idle',
    url: '',
    hostname: '',
    secret: '',
  });
  const [showQRModal, setShowQRModal] = useState(false);
  const [showAppStoreQRModal, setShowAppStoreQRModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [copiedSecret, setCopiedSecret] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const statusMessages: Record<TunnelInfo['state'], string> = {
    idle: t('tunnel.status_idle', 'Tunnel is not running'),
    starting: t('tunnel.status_starting', 'Starting tunnel...'),
    running: t('tunnel.status_running', 'Tunnel is active'),
    error: t('tunnel.status_error', 'Tunnel encountered an error'),
    disabled: t('tunnel.status_disabled', 'Tunnel is disabled'),
  };

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

  if (tunnelInfo.state === 'disabled') {
    return null;
  }

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">{t('tunnel.title', 'Remote Access')}</CardTitle>
          <CardDescription className="flex flex-col gap-2">
            <div className="flex items-start gap-2 p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded">
              <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
              <div className="text-xs text-blue-800 dark:text-blue-200">
                <strong>{t('tunnel.preview', 'Preview feature:')}</strong>{' '}
                {t(
                  'tunnel.preview_desc',
                  'Enable remote access to goose from mobile devices using secure tunneling.'
                )}{' '}
                <a
                  href={IOS_APP_STORE_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 underline hover:no-underline"
                >
                  {t('tunnel.get_ios', 'Get the iOS app')}
                  <ExternalLink className="h-3 w-3" />
                </a>
                {` ${t('common.or', 'or')} `}
                <button
                  onClick={() => setShowAppStoreQRModal(true)}
                  className="inline-flex items-center gap-1 underline hover:no-underline"
                >
                  {t('tunnel.scan_qr', 'scan QR code')}
                  <QrCode className="h-3 w-3" />
                </button>
              </div>
            </div>
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
              <h3 className="text-text-default text-xs">{t('tunnel.status', 'Tunnel Status')}</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                {statusMessages[tunnelInfo.state]}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {tunnelInfo.state === 'starting' ? (
                <Button disabled variant="secondary" size="sm">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('tunnel.starting', 'Starting...')}
                </Button>
              ) : tunnelInfo.state === 'running' ? (
                <>
                  <Button onClick={() => setShowQRModal(true)} variant="default" size="sm">
                    {t('tunnel.show_qr', 'Show QR Code')}
                  </Button>
                  <Button onClick={handleToggleTunnel} variant="destructive" size="sm">
                    {t('tunnel.stop', 'Stop Tunnel')}
                  </Button>
                </>
              ) : (
                <Button onClick={handleToggleTunnel} variant="default" size="sm">
                  {tunnelInfo.state === 'error'
                    ? t('tunnel.retry', 'Retry')
                    : t('tunnel.start', 'Start Tunnel')}
                </Button>
              )}
            </div>
          </div>

          {tunnelInfo.state === 'running' && (
            <div className="p-3 bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-800 rounded">
              <p className="text-xs text-green-800 dark:text-green-200">
                <strong>{t('tunnel.url', 'URL:')}</strong> {tunnelInfo.url}
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={showQRModal} onOpenChange={setShowQRModal}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>{t('tunnel.connection', 'Remote Access Connection')}</DialogTitle>
          </DialogHeader>

          {tunnelInfo.state === 'running' && (
            <div className="py-4 space-y-4">
              <div className="flex justify-center">
                <div className="p-4 bg-white rounded-lg">
                  <QRCodeSVG value={getQRCodeData()} size={200} />
                </div>
              </div>

              <div className="text-center text-sm text-text-muted">
                {t(
                  'tunnel.qr_help',
                  'Scan this QR code with the goose mobile app. Do not share this code with anyone else as it is for your personal access.'
                )}
              </div>

              <div className="border-t pt-4">
                <button
                  onClick={() => setShowDetails(!showDetails)}
                  className="flex items-center justify-between w-full text-sm font-medium hover:opacity-70 transition-opacity"
                >
                  <span>{t('tunnel.connection_details', 'Connection Details')}</span>
                  {showDetails ? (
                    <ChevronUp className="h-4 w-4" />
                  ) : (
                    <ChevronDown className="h-4 w-4" />
                  )}
                </button>

                {showDetails && (
                  <div className="mt-3 space-y-3">
                    <div>
                      <h3 className="text-xs font-medium mb-1 text-text-muted">
                        {t('tunnel.tunnel_url', 'Tunnel URL')}
                      </h3>
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
                      <h3 className="text-xs font-medium mb-1 text-text-muted">
                        {t('tunnel.secret_key', 'Secret Key')}
                      </h3>
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
              {t('common.close', 'Close')}
            </Button>
            <Button variant="destructive" onClick={handleToggleTunnel}>
              {t('tunnel.stop', 'Stop Tunnel')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={showAppStoreQRModal} onOpenChange={setShowAppStoreQRModal}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>{t('tunnel.download_ios', 'Download goose iOS App')}</DialogTitle>
          </DialogHeader>

          <div className="py-4 space-y-4">
            <div className="flex justify-center">
              <div className="p-4 bg-white rounded-lg">
                <QRCodeSVG value={IOS_APP_STORE_URL} size={200} />
              </div>
            </div>

            <div className="text-center text-sm text-text-muted">
              {t(
                'tunnel.appstore_qr_help',
                'Scan this QR code with your iPhone camera to install the goose mobile app from the App Store'
              )}
            </div>

            <div className="text-center">
              <a
                href={IOS_APP_STORE_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400 hover:underline"
              >
                <ExternalLink className="h-4 w-4" />
                {t('tunnel.open_app_store', 'Open in App Store')}
              </a>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAppStoreQRModal(false)}>
              {t('common.close', 'Close')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
