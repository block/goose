import { useState, useEffect, useCallback, useRef } from 'react';
import { Zap, RefreshCw, Loader2, Copy, Check } from 'lucide-react';
import { getApiUrl } from '../../../config';

interface WalletBalance {
  trusted_sats: number;
  lightning_sats: number;
  pending_sats: number;
  total_sats: number;
}

interface Invoice {
  bolt11: string;
  qr_svg: string;
  amount_sats: number | null;
}

type WalletState =
  | 'disabled'
  | 'uninitialized'
  | 'initializing'
  | { error: { message: string } }
  | 'ready';

function walletStateLabel(state: WalletState): string {
  if (state === 'disabled') return 'Disabled (rebuild with --features lightning)';
  if (state === 'uninitialized') return 'Ready';
  if (state === 'initializing') return 'Starting...';
  if (state === 'ready') return 'Ready';
  if (typeof state === 'object' && 'error' in state) return `Error: ${state.error.message}`;
  return 'Unknown';
}

async function getAuthHeaders(): Promise<Record<string, string>> {
  const secretKey = await window.electron.getSecretKey();
  return {
    'Content-Type': 'application/json',
    'X-Secret-Key': secretKey,
  };
}

export default function WalletSettings() {
  const [walletState, setWalletState] = useState<WalletState | null>(null);
  const [balance, setBalance] = useState<WalletBalance | null>(null);
  const [loading, setLoading] = useState(true);

  // Deposit state
  const [depositAmount, setDepositAmount] = useState('1000');
  const [invoice, setInvoice] = useState<Invoice | null>(null);
  const [creatingInvoice, setCreatingInvoice] = useState(false);
  const [invoiceError, setInvoiceError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [paymentReceived, setPaymentReceived] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const headers = await getAuthHeaders();
      const resp = await fetch(getApiUrl('/wallet/status'), { headers });
      if (resp.ok) {
        const data = await resp.json();
        setWalletState(data.state);
      }
    } catch {
      // Server not reachable.
    }
  }, []);

  const fetchBalance = useCallback(async () => {
    try {
      const headers = await getAuthHeaders();
      const resp = await fetch(getApiUrl('/wallet/balance'), { headers });
      if (resp.ok) {
        setBalance(await resp.json());
      }
    } catch {
      // Wallet not ready.
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      setLoading(true);
      await fetchStatus();
      setLoading(false);
    };
    init();
  }, [fetchStatus]);

  useEffect(() => {
    if (walletState === 'ready') {
      fetchBalance();
    }
  }, [walletState, fetchBalance]);

  // SSE listener for payment events when an invoice is displayed.
  useEffect(() => {
    if (!invoice) return;

    const evtSource = new EventSource(getApiUrl('/wallet/events'));
    eventSourceRef.current = evtSource;

    evtSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.amount_sats) {
          setPaymentReceived(true);
          setInvoice(null);
          fetchBalance();
          fetchStatus();
        }
      } catch {
        // Ignore parse errors.
      }
    };

    return () => {
      evtSource.close();
      eventSourceRef.current = null;
    };
  }, [invoice, fetchBalance, fetchStatus]);

  const handleCreateInvoice = async () => {
    setCreatingInvoice(true);
    setInvoiceError(null);
    setInvoice(null);
    setPaymentReceived(false);

    try {
      const amount = parseInt(depositAmount, 10);
      if (isNaN(amount) || amount <= 0) {
        setInvoiceError('Please enter a valid amount in satoshis');
        setCreatingInvoice(false);
        return;
      }

      const headers = await getAuthHeaders();
      const resp = await fetch(getApiUrl('/wallet/invoice'), {
        method: 'POST',
        headers,
        body: JSON.stringify({ amount_sats: amount }),
      });

      if (!resp.ok) {
        try {
          const data = await resp.json();
          setInvoiceError(data.error || 'Failed to create invoice');
        } catch {
          setInvoiceError(`Failed to create invoice (${resp.status})`);
        }
        setCreatingInvoice(false);
        return;
      }

      const data: Invoice = await resp.json();
      setInvoice(data);

      // Refresh status in case wallet just initialized.
      await fetchStatus();
    } catch (e) {
      setInvoiceError(`Error: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setCreatingInvoice(false);
    }
  };

  const handleCopy = async () => {
    if (!invoice) return;
    try {
      await navigator.clipboard.writeText(invoice.bolt11);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard write failed.
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-gray-500 py-4">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading wallet status...
      </div>
    );
  }

  const isDisabled = walletState === 'disabled';

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-medium flex items-center gap-2">
          <Zap className="h-5 w-5 text-orange-500" />
          Lightning Wallet
        </h3>
        <p className="text-sm text-gray-500 mt-1">
          Receive Bitcoin via Lightning using the Orange SDK.
        </p>
      </div>

      {/* Status + Balance */}
      <div className="rounded-lg border p-4 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Status</span>
          <span className="text-sm text-gray-600 dark:text-gray-400">
            {walletState ? walletStateLabel(walletState) : 'Unknown'}
          </span>
        </div>

        {walletState === 'ready' && balance && (
          <div className="space-y-1 pt-2 border-t">
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">Trusted (Spark)</span>
              <span>{balance.trusted_sats.toLocaleString()} sats</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">Lightning</span>
              <span>{balance.lightning_sats.toLocaleString()} sats</span>
            </div>
            {balance.pending_sats > 0 && (
              <div className="flex justify-between text-sm">
                <span className="text-gray-500">Pending</span>
                <span>{balance.pending_sats.toLocaleString()} sats</span>
              </div>
            )}
            <div className="flex justify-between text-sm font-medium pt-1 border-t">
              <span>Total available</span>
              <span>{balance.total_sats.toLocaleString()} sats</span>
            </div>
            <button
              onClick={fetchBalance}
              className="mt-2 inline-flex items-center gap-1 text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
            >
              <RefreshCw className="h-3 w-3" />
              Refresh
            </button>
          </div>
        )}
      </div>

      {/* Deposit */}
      {!isDisabled && (
        <div className="rounded-lg border p-4 space-y-4">
          <h4 className="text-sm font-medium">Deposit</h4>

          {paymentReceived && (
            <div className="flex items-center gap-2 rounded-md bg-green-500/10 border border-green-600/30 p-3">
              <Check className="h-4 w-4 text-green-600 dark:text-green-400" />
              <span className="text-sm font-medium text-green-800 dark:text-green-200">
                Payment received!
              </span>
            </div>
          )}

          {!invoice && (
            <div className="flex items-center gap-2">
              <input
                type="number"
                min="1"
                value={depositAmount}
                onChange={(e) => setDepositAmount(e.target.value)}
                placeholder="Amount in sats"
                className="w-32 rounded-md border bg-white dark:bg-gray-800 text-sm px-3 py-1.5"
              />
              <span className="text-xs text-gray-500">sats</span>
              <button
                onClick={handleCreateInvoice}
                disabled={creatingInvoice}
                className="inline-flex items-center gap-1.5 rounded-md bg-orange-600 hover:bg-orange-500 text-white text-sm font-medium px-3 py-1.5 transition-colors disabled:opacity-50"
              >
                {creatingInvoice ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Zap className="h-3.5 w-3.5" />
                )}
                Create Invoice
              </button>
            </div>
          )}

          {invoiceError && (
            <div className="text-sm text-red-600 dark:text-red-400">{invoiceError}</div>
          )}

          {invoice && (
            <div className="space-y-3">
              <div
                className="bg-white rounded-lg p-2 w-fit"
                dangerouslySetInnerHTML={{ __html: invoice.qr_svg }}
              />

              <div className="flex items-center gap-2">
                <code className="text-xs bg-gray-100 dark:bg-gray-800 rounded px-2 py-1 break-all max-w-[300px] truncate">
                  {invoice.bolt11}
                </code>
                <button
                  onClick={handleCopy}
                  className="shrink-0 p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                  title="Copy invoice"
                >
                  {copied ? (
                    <Check className="h-3.5 w-3.5 text-green-600" />
                  ) : (
                    <Copy className="h-3.5 w-3.5 text-gray-500" />
                  )}
                </button>
              </div>

              <div className="flex items-center gap-2 text-sm text-gray-500">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                Waiting for payment...
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
