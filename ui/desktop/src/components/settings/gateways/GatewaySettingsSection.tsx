import { useState, useEffect, useCallback } from 'react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../../ui/card';
import { Input } from '../../ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '../../ui/dialog';
import { Loader2, Copy, Check, Square, Trash2, ExternalLink, User } from 'lucide-react';
import { getApiUrl } from '../../../config';

interface PairedUserInfo {
  platform: string;
  user_id: string;
  display_name: string | null;
  session_id: string;
  paired_at: number;
}

interface GatewayStatus {
  gateway_type: string;
  running: boolean;
  configured: boolean;
  paired_users: PairedUserInfo[];
  info?: Record<string, string>;
}

interface PairingCodeResponse {
  code: string;
  expires_at: number;
}

async function gatewayFetch(endpoint: string, options: globalThis.RequestInit = {}) {
  const secretKey = await window.electron.getSecretKey();
  const url = getApiUrl(endpoint);
  return fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': secretKey,
      ...options.headers,
    },
  });
}

export default function GatewaySettingsSection() {
  const [gateways, setGateways] = useState<GatewayStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pairingCode, setPairingCode] = useState<PairingCodeResponse | null>(null);
  const [pairingGatewayType, setPairingGatewayType] = useState<string | null>(null);
  const [copiedCode, setCopiedCode] = useState(false);

  const fetchStatus = useCallback(async () => {
    try {
      const response = await gatewayFetch('/gateway/status');
      if (response.ok) {
        setGateways(await response.json());
      }
    } catch (err) {
      console.error('Failed to fetch gateway status:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  const doPost = async (endpoint: string, body: object, errorMsg: string) => {
    setError(null);
    try {
      const response = await gatewayFetch(endpoint, {
        method: 'POST',
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || errorMsg);
      }
      await fetchStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : errorMsg);
    }
  };

  const handleUnpairUser = async (platform: string, userId: string) => {
    setError(null);
    try {
      const response = await gatewayFetch(`/gateway/pair/${platform}/${userId}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || 'Failed to unpair user');
      }
      await fetchStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to unpair user');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedCode(true);
      setTimeout(() => setCopiedCode(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-text-muted py-4">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading...
      </div>
    );
  }

  const telegram = gateways.find((g) => g.gateway_type === 'telegram');

  return (
    <>
      {error && (
        <div className="p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-800 rounded text-sm text-red-800 dark:text-red-200 mb-4">
          {error}
        </div>
      )}

      <TelegramGatewayCard
        status={telegram}
        onStart={(config) =>
          doPost('/gateway/start', { gateway_type: 'telegram', platform_config: config, max_sessions: 0 }, 'Failed to start')
        }
        onRestart={() => doPost('/gateway/restart', { gateway_type: 'telegram' }, 'Failed to start')}
        onStop={() => doPost('/gateway/stop', { gateway_type: 'telegram' }, 'Failed to stop')}
        onRemove={() => doPost('/gateway/remove', { gateway_type: 'telegram' }, 'Failed to remove')}
        onGenerateCode={async () => {
          setError(null);
          try {
            const response = await gatewayFetch('/gateway/pair', {
              method: 'POST',
              body: JSON.stringify({ gateway_type: 'telegram' }),
            });
            if (!response.ok) {
              const data = await response.json().catch(() => ({}));
              throw new Error(data.message || 'Failed to generate pairing code');
            }
            const data: PairingCodeResponse = await response.json();
            setPairingCode(data);
            setPairingGatewayType('telegram');
          } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to generate pairing code');
          }
        }}
        onUnpairUser={handleUnpairUser}
      />

      <PairingCodeModal
        open={pairingCode !== null}
        onClose={() => {
          setPairingCode(null);
          setPairingGatewayType(null);
        }}
        code={pairingCode}
        gatewayType={pairingGatewayType}
        onCopy={copyToClipboard}
        copied={copiedCode}
      />
    </>
  );
}

function PairedUsersList({
  users,
  onUnpairUser,
}: {
  users: PairedUserInfo[];
  onUnpairUser: (platform: string, userId: string) => void;
}) {
  if (users.length === 0) return null;

  return (
    <div className="space-y-1 mt-2">
      <h4 className="text-xs text-text-muted font-medium">Paired Users</h4>
      {users.map((user) => (
        <div
          key={`${user.platform}-${user.user_id}`}
          className="flex items-center justify-between py-1.5 px-2 bg-background-muted rounded text-sm"
        >
          <div className="flex items-center gap-2 min-w-0">
            <User className="h-3 w-3 text-text-muted flex-shrink-0" />
            <span className="truncate">{user.display_name || user.user_id}</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onUnpairUser(user.platform, user.user_id)}
            className="h-6 w-6 p-0 text-text-muted hover:text-red-600 flex-shrink-0"
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      ))}
    </div>
  );
}

function TelegramGatewayCard({
  status,
  onStart,
  onRestart,
  onStop,
  onRemove,
  onGenerateCode,
  onUnpairUser,
}: {
  status: GatewayStatus | undefined;
  onStart: (config: Record<string, unknown>) => Promise<void>;
  onRestart: () => Promise<void>;
  onStop: () => Promise<void>;
  onRemove: () => Promise<void>;
  onGenerateCode: () => void;
  onUnpairUser: (platform: string, userId: string) => void;
}) {
  const [botToken, setBotToken] = useState('');
  const [busy, setBusy] = useState(false);
  const running = status?.running ?? false;
  const configured = status?.configured ?? false;

  const wrap = (fn: () => Promise<void>) => async () => {
    setBusy(true);
    try { await fn(); } finally { setBusy(false); }
  };

  const handleFirstStart = wrap(async () => {
    if (!botToken.trim()) return;
    await onStart({ bot_token: botToken.trim() });
    setBotToken('');
  });

  return (
    <Card className="rounded-lg">
      <CardHeader className="pb-0">
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            Telegram
            {running && (
              <span className="inline-flex items-center text-xs text-green-700 dark:text-green-400 bg-green-100 dark:bg-green-900/30 px-2 py-0.5 rounded-full">
                Running
              </span>
            )}
            {!running && configured && (
              <span className="inline-flex items-center text-xs text-yellow-700 dark:text-yellow-400 bg-yellow-100 dark:bg-yellow-900/30 px-2 py-0.5 rounded-full">
                Stopped
              </span>
            )}
          </CardTitle>
          <div className="flex items-center gap-2">
            {running && (
              <>
                <Button variant="outline" size="sm" onClick={onGenerateCode}>
                  Pair Device
                </Button>
                <Button variant="destructive" size="sm" disabled={busy} onClick={wrap(onStop)}>
                  <Square className="h-3 w-3 mr-1" />
                  Stop
                </Button>
              </>
            )}
            {!running && configured && (
              <>
                <Button size="sm" disabled={busy} onClick={wrap(onRestart)}>
                  {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Start'}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={busy}
                  onClick={wrap(onRemove)}
                  className="text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:text-red-300 dark:hover:bg-red-900/20"
                >
                  <Trash2 className="h-3 w-3 mr-1" />
                  Remove
                </Button>
              </>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-3 space-y-2">
        {!running && !configured && (
          <>
            <div className="text-xs text-text-muted space-y-1.5 mb-2">
              <p>
                Open{' '}
                <a
                  href="https://t.me/BotFather"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-0.5 text-blue-600 dark:text-blue-400 hover:underline"
                >
                  @BotFather
                  <ExternalLink className="h-3 w-3" />
                </a>
                {' '}on your phone, send <code className="bg-background-muted px-1 py-0.5 rounded">/newbot</code>, and follow
                the prompts to name your bot. BotFather will reply with an API token — paste it below.
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Input
                type="password"
                placeholder="Paste bot token here"
                value={botToken}
                onChange={(e) => setBotToken(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleFirstStart()}
                className="text-sm"
              />
              <Button size="sm" onClick={handleFirstStart} disabled={busy || !botToken.trim()}>
                {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Start'}
              </Button>
            </div>
          </>
        )}
        {status && <PairedUsersList users={status.paired_users} onUnpairUser={onUnpairUser} />}
      </CardContent>
    </Card>
  );
}

function PairingCodeModal({
  open,
  onClose,
  code,
  gatewayType,
  onCopy,
  copied,
}: {
  open: boolean;
  onClose: () => void;
  code: PairingCodeResponse | null;
  gatewayType: string | null;
  onCopy: (text: string) => void;
  copied: boolean;
}) {
  const [timeRemaining, setTimeRemaining] = useState(0);

  useEffect(() => {
    if (!code) return;

    const updateTimer = () => {
      const remaining = Math.max(0, code.expires_at - Math.floor(Date.now() / 1000));
      setTimeRemaining(remaining);
      if (remaining === 0) {
        onClose();
      }
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [code, onClose]);

  if (!code) return null;

  const minutes = Math.floor(timeRemaining / 60);
  const seconds = timeRemaining % 60;

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Pairing Code</DialogTitle>
        </DialogHeader>

        <div className="py-6 space-y-4">
          <div className="flex justify-center">
            <div className="flex items-center gap-2">
              <code className="text-4xl font-mono font-bold tracking-[0.3em] select-all">
                {code.code}
              </code>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onCopy(code.code)}
                className="flex-shrink-0"
              >
                {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
              </Button>
            </div>
          </div>

          <p className="text-center text-sm text-text-muted">
            Send this code to your{' '}
            <span className="capitalize font-medium">{gatewayType}</span> bot to pair.
          </p>

          <div className="text-center text-xs text-text-muted">
            Expires in {minutes}:{seconds.toString().padStart(2, '0')}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
