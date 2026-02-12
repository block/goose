import { useState, useEffect, useCallback } from 'react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { Input } from '../../ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '../../ui/dialog';
import {
  Loader2,
  Copy,
  Check,
  Plus,
  Square,
  Trash2,
  ExternalLink,
  Radio,
  User,
} from 'lucide-react';
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
  paired_users: PairedUserInfo[];
}

interface PairingCodeResponse {
  code: string;
  expires_at: number;
}

async function gatewayFetch(endpoint: string, options: globalThis.RequestInit = {}) {
  const secretKey = await window.electron.getSecretKey();
  const url = getApiUrl(endpoint);
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': secretKey,
      ...options.headers,
    },
  });
  return response;
}

export default function GatewaySettingsSection() {
  const [gateways, setGateways] = useState<GatewayStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showAddModal, setShowAddModal] = useState(false);
  const [pairingCode, setPairingCode] = useState<PairingCodeResponse | null>(null);
  const [pairingGatewayType, setPairingGatewayType] = useState<string | null>(null);
  const [copiedCode, setCopiedCode] = useState(false);

  const fetchStatus = useCallback(async () => {
    try {
      const response = await gatewayFetch('/gateway/status');
      if (response.ok) {
        const data: GatewayStatus[] = await response.json();
        setGateways(data);
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

  const handleStopGateway = async (gatewayType: string) => {
    setError(null);
    try {
      const response = await gatewayFetch('/gateway/stop', {
        method: 'POST',
        body: JSON.stringify({ gateway_type: gatewayType }),
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || `Failed to stop gateway`);
      }
      await fetchStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop gateway');
    }
  };

  const handleGeneratePairingCode = async (gatewayType: string) => {
    setError(null);
    try {
      const response = await gatewayFetch('/gateway/pair', {
        method: 'POST',
        body: JSON.stringify({ gateway_type: gatewayType }),
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || 'Failed to generate pairing code');
      }
      const data: PairingCodeResponse = await response.json();
      setPairingCode(data);
      setPairingGatewayType(gatewayType);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to generate pairing code');
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

  return (
    <section className="space-y-4 pr-4 mt-1">
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Gateways</CardTitle>
          <CardDescription>
            Connect Goose to external messaging platforms like Telegram. Messages sent to the bot
            are handled by Goose as a long-running session.
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 px-4 space-y-4">
          {error && (
            <div className="p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-800 rounded text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}

          {loading ? (
            <div className="flex items-center gap-2 text-sm text-text-muted">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading...
            </div>
          ) : gateways.length === 0 ? (
            <div className="text-sm text-text-muted py-2">
              No gateways configured. Add one to get started.
            </div>
          ) : (
            <div className="space-y-3">
              {gateways.map((gw) => (
                <GatewayCard
                  key={gw.gateway_type}
                  gateway={gw}
                  onStop={() => handleStopGateway(gw.gateway_type)}
                  onGenerateCode={() => handleGeneratePairingCode(gw.gateway_type)}
                  onUnpairUser={handleUnpairUser}
                />
              ))}
            </div>
          )}

          <Button
            variant="default"
            size="sm"
            onClick={() => setShowAddModal(true)}
            className="flex items-center gap-2"
          >
            <Plus className="h-4 w-4" />
            Add Gateway
          </Button>
        </CardContent>
      </Card>

      <AddGatewayModal
        open={showAddModal}
        onClose={() => setShowAddModal(false)}
        onAdded={() => {
          setShowAddModal(false);
          fetchStatus();
        }}
        onError={setError}
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
    </section>
  );
}

function GatewayCard({
  gateway,
  onStop,
  onGenerateCode,
  onUnpairUser,
}: {
  gateway: GatewayStatus;
  onStop: () => void;
  onGenerateCode: () => void;
  onUnpairUser: (platform: string, userId: string) => void;
}) {
  return (
    <div className="border border-border-default rounded-md p-3 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Radio className="h-4 w-4 text-text-muted" />
          <span className="text-sm font-medium capitalize">{gateway.gateway_type}</span>
          {gateway.running && (
            <span className="inline-flex items-center gap-1 text-xs text-green-700 dark:text-green-400 bg-green-100 dark:bg-green-900/30 px-2 py-0.5 rounded-full">
              Running
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={onGenerateCode}>
            Pair Device
          </Button>
          <Button variant="destructive" size="sm" onClick={onStop}>
            <Square className="h-3 w-3 mr-1" />
            Stop
          </Button>
        </div>
      </div>

      {gateway.paired_users.length > 0 && (
        <div className="space-y-1">
          <h4 className="text-xs text-text-muted font-medium">Paired Users</h4>
          {gateway.paired_users.map((user) => (
            <div
              key={`${user.platform}-${user.user_id}`}
              className="flex items-center justify-between py-1.5 px-2 bg-background-muted rounded text-sm"
            >
              <div className="flex items-center gap-2">
                <User className="h-3 w-3 text-text-muted" />
                <span>{user.display_name || user.user_id}</span>
                <span className="text-xs text-text-muted">({user.user_id})</span>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onUnpairUser(user.platform, user.user_id)}
                className="h-6 w-6 p-0 text-text-muted hover:text-red-600"
              >
                <Trash2 className="h-3 w-3" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function AddGatewayModal({
  open,
  onClose,
  onAdded,
  onError,
}: {
  open: boolean;
  onClose: () => void;
  onAdded: () => void;
  onError: (msg: string) => void;
}) {
  const [botToken, setBotToken] = useState('');
  const [starting, setStarting] = useState(false);

  const handleStart = async () => {
    if (!botToken.trim()) {
      onError('Bot token is required');
      return;
    }

    setStarting(true);
    try {
      const response = await gatewayFetch('/gateway/start', {
        method: 'POST',
        body: JSON.stringify({
          gateway_type: 'telegram',
          platform_config: { bot_token: botToken.trim() },
          max_sessions: 0,
        }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.message || 'Failed to start gateway');
      }

      setBotToken('');
      onAdded();
    } catch (err) {
      onError(err instanceof Error ? err.message : 'Failed to start gateway');
    } finally {
      setStarting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>Add Telegram Gateway</DialogTitle>
        </DialogHeader>

        <div className="py-4 space-y-4">
          <div className="space-y-2">
            <label htmlFor="bot-token" className="text-sm font-medium">
              Bot Token
            </label>
            <Input
              id="bot-token"
              type="password"
              placeholder="123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
              value={botToken}
              onChange={(e) => setBotToken(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleStart()}
            />
            <p className="text-xs text-text-muted">
              Create a bot with{' '}
              <a
                href="https://t.me/BotFather"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1 text-blue-600 dark:text-blue-400 hover:underline"
              >
                @BotFather
                <ExternalLink className="h-3 w-3" />
              </a>{' '}
              on Telegram and paste the token here.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleStart} disabled={starting || !botToken.trim()}>
            {starting ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Starting...
              </>
            ) : (
              'Start Gateway'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
            <span className="capitalize font-medium">{gatewayType}</span> bot to pair it with
            Goose.
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
